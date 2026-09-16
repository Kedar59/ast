use std::path::Path;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Instant;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::events::AppEvent;

static ACTIVE_GRADLE_PID: Mutex<Option<u32>> = Mutex::new(None);

/// Attempts to cancel any actively running Gradle task by terminating its process
pub fn cancel_current_gradle_task() -> bool {
    if let Ok(mut guard) = ACTIVE_GRADLE_PID.lock()
        && let Some(pid) = guard.take() {
            // Send SIGTERM to the gradle child process
            let _ = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .spawn();
            return true;
        }
    false
}

/// Checks whether `./gradlew` is available in the current working directory
pub fn check_gradlew_exists() -> bool {
    Path::new("./gradlew").exists()
}

/// Dispatches an asynchronous Gradle task and streams stdout/stderr lines into `tx`
pub async fn run_gradle_task(
    args: Vec<String>,
    display_name: String,
    tx: mpsc::Sender<AppEvent>,
) {
    if !check_gradlew_exists() {
        let _ = tx
            .send(AppEvent::GradleTaskStarted(display_name.clone()))
            .await;
        let _ = tx
            .send(AppEvent::GradleLogLine(
                "Error: No './gradlew' found in current directory.".to_string(),
            ))
            .await;
        let _ = tx
            .send(AppEvent::GradleLogLine(
                "Please run 'ast' from the root of an Android/Gradle project containing './gradlew'."
                    .to_string(),
            ))
            .await;
        let _ = tx
            .send(AppEvent::GradleTaskFinished {
                task: display_name,
                success: false,
                exit_code: Some(1),
                duration: std::time::Duration::from_millis(0),
                apk_path: None,
            })
            .await;
        return;
    }

    let _ = tx
        .send(AppEvent::GradleTaskStarted(display_name.clone()))
        .await;
    let _ = tx
        .send(AppEvent::GradleLogLine(format!(
            "$ ./gradlew {}",
            args.join(" ")
        )))
        .await;

    let start_time = Instant::now();

    let mut cmd = Command::new("./gradlew");
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            let _ = tx
                .send(AppEvent::GradleLogLine(format!(
                    "Failed to spawn './gradlew': {err}"
                )))
                .await;
            let _ = tx
                .send(AppEvent::GradleTaskFinished {
                    task: display_name,
                    success: false,
                    exit_code: None,
                    duration: start_time.elapsed(),
                    apk_path: None,
                })
                .await;
            return;
        }
    };

    // Store child PID for cancellation
    if let Some(pid) = child.id()
        && let Ok(mut guard) = ACTIVE_GRADLE_PID.lock() {
            *guard = Some(pid);
        }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let tx_out = tx.clone();
    let stdout_handle = tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx_out.send(AppEvent::GradleLogLine(line)).await;
            }
        }
    });

    let tx_err = tx.clone();
    let stderr_handle = tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx_err.send(AppEvent::GradleLogLine(line)).await;
            }
        }
    });

    let _ = tokio::join!(stdout_handle, stderr_handle);
    let status_res = child.wait().await;

    // Clear active PID
    if let Ok(mut guard) = ACTIVE_GRADLE_PID.lock() {
        *guard = None;
    }

    let elapsed = start_time.elapsed();

    match status_res {
        Ok(status) => {
            let success = status.success();
            let exit_code = status.code();

            let apk_path = if success && args.iter().any(|a| a.contains("assemble")) {
                if let Some((path, size)) = find_debug_apk() {
                    let size_mb = size as f64 / (1024.0 * 1024.0);
                    let _ = tx
                        .send(AppEvent::GradleLogLine(format!(
                            "✔ APK generated: {path} ({size_mb:.2} MB)"
                        )))
                        .await;
                    Some(path)
                } else {
                    None
                }
            } else {
                None
            };

            let _ = tx
                .send(AppEvent::GradleTaskFinished {
                    task: display_name,
                    success,
                    exit_code,
                    duration: elapsed,
                    apk_path,
                })
                .await;
        }
        Err(err) => {
            let _ = tx
                .send(AppEvent::GradleLogLine(format!(
                    "Gradle process error: {err}"
                )))
                .await;
            let _ = tx
                .send(AppEvent::GradleTaskFinished {
                    task: display_name,
                    success: false,
                    exit_code: None,
                    duration: elapsed,
                    apk_path: None,
                })
                .await;
        }
    }
}

/// Locates generated debug APK in standard Android build output directories
pub fn find_debug_apk() -> Option<(String, u64)> {
    let candidate_paths = [
        "app/build/outputs/apk/debug/app-debug.apk",
        "build/outputs/apk/debug/app-debug.apk",
        "app/build/outputs/apk/app-debug.apk",
        "build/outputs/apk/app-debug.apk",
    ];

    for rel_path in candidate_paths {
        if let Ok(meta) = std::fs::metadata(rel_path) {
            return Some((rel_path.to_string(), meta.len()));
        }
    }
    None
}
