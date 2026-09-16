use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::events::AppEvent;

/// Build debug APK and deploy to running target
pub async fn build_and_deploy_apk(
    serial: String,
    event_tx: mpsc::Sender<AppEvent>,
) {
    let gradlew_path = Path::new("./gradlew");
    if !gradlew_path.exists() {
        let _ = event_tx
            .send(AppEvent::ActionFinished {
                action: "Build & Deploy".to_string(),
                success: false,
                message: "No ./gradlew found in current directory. Run from an Android project root."
                    .to_string(),
            })
            .await;
        return;
    }

    let _ = event_tx
        .send(AppEvent::ActionStarted(format!(
            "Building debug APK for target '{serial}'..."
        )))
        .await;

    // Step 1: assembleDebug
    let mut build_cmd = Command::new("./gradlew");
    build_cmd
        .args(["assembleDebug", "--console=plain"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match build_cmd.spawn() {
        Ok(mut child) => {
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                let tx = event_tx.clone();
                tokio::spawn(async move {
                    while let Ok(Some(line)) = lines.next_line().await {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            let _ = tx.send(AppEvent::StatusLog(format!("[gradle] {trimmed}"))).await;
                        }
                    }
                });
            }

            match child.wait().await {
                Ok(status) if status.success() => {
                    let _ = event_tx
                        .send(AppEvent::StatusLog(
                            "Gradle build succeeded. Installing on target...".to_string(),
                        ))
                        .await;
                }
                Ok(status) => {
                    let _ = event_tx
                        .send(AppEvent::ActionFinished {
                            action: "assembleDebug".to_string(),
                            success: false,
                            message: format!("Gradle assembleDebug failed with exit code: {:?}", status.code()),
                        })
                        .await;
                    return;
                }
                Err(e) => {
                    let _ = event_tx
                        .send(AppEvent::ActionFinished {
                            action: "assembleDebug".to_string(),
                            success: false,
                            message: format!("Failed to run assembleDebug: {e}"),
                        })
                        .await;
                    return;
                }
            }
        }
        Err(e) => {
            let _ = event_tx
                .send(AppEvent::ActionFinished {
                    action: "assembleDebug".to_string(),
                    success: false,
                    message: format!("Failed to spawn ./gradlew: {e}"),
                })
                .await;
            return;
        }
    }

    // Step 2: installDebug
    let mut install_cmd = Command::new("./gradlew");
    install_cmd
        .args(["installDebug", "--console=plain"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match install_cmd.output().await {
        Ok(output) if output.status.success() => {
            let _ = event_tx
                .send(AppEvent::ActionFinished {
                    action: "Build & Deploy".to_string(),
                    success: true,
                    message: format!("Successfully deployed debug APK to target '{serial}'."),
                })
                .await;
        }
        Ok(output) => {
            let err = String::from_utf8_lossy(&output.stderr);
            let _ = event_tx
                .send(AppEvent::ActionFinished {
                    action: "installDebug".to_string(),
                    success: false,
                    message: format!("installDebug failed: {err}"),
                })
                .await;
        }
        Err(e) => {
            let _ = event_tx
                .send(AppEvent::ActionFinished {
                    action: "installDebug".to_string(),
                    success: false,
                    message: format!("Failed to execute installDebug: {e}"),
                })
                .await;
        }
    }
}

/// Force stop an application package on target
pub async fn force_stop_app(serial: &str, package: &str) -> Result<String, String> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "am", "force-stop", package])
        .output()
        .await
        .map_err(|e| format!("Failed to execute force-stop: {e}"))?;

    if output.status.success() {
        Ok(format!("Force-stopped '{package}' on '{serial}'."))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("force-stop error: {err}"))
    }
}
