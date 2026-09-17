use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::events::AppEvent;

/// Detects package name from Android build artifacts, manifest files, or build.gradle
pub fn detect_package_name() -> Option<String> {
    // 1. Check build.gradle(.kts) namespace or applicationId
    let gradle_files = [
        "app/build.gradle.kts",
        "app/build.gradle",
        "build.gradle.kts",
        "build.gradle",
    ];

    for file_path in &gradle_files {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if (trimmed.starts_with("namespace") || trimmed.starts_with("applicationId"))
                    && (trimmed.contains('=') || trimmed.contains(' '))
                {
                    // Extract value between quotes
                    if let Some(first_quote) = trimmed.find('"')
                        && let Some(second_quote) = trimmed[first_quote + 1..].find('"')
                    {
                        let pkg = &trimmed[first_quote + 1..first_quote + 1 + second_quote];
                        if !pkg.is_empty() {
                            return Some(pkg.to_string());
                        }
                    }
                    if let Some(first_quote) = trimmed.find('\'')
                        && let Some(second_quote) = trimmed[first_quote + 1..].find('\'')
                    {
                        let pkg = &trimmed[first_quote + 1..first_quote + 1 + second_quote];
                        if !pkg.is_empty() {
                            return Some(pkg.to_string());
                        }
                    }
                }
            }
        }
    }

    // 2. Check AndroidManifest.xml package attribute
    let manifest_files = [
        "app/src/main/AndroidManifest.xml",
        "src/main/AndroidManifest.xml",
    ];

    for file_path in &manifest_files {
        if let Ok(content) = std::fs::read_to_string(file_path)
            && let Some(pkg_idx) = content.find("package=\"")
        {
            let rest = &content[pkg_idx + 9..];
            if let Some(end_quote) = rest.find('"') {
                let pkg = &rest[..end_quote];
                if !pkg.is_empty() {
                    return Some(pkg.to_string());
                }
            }
        }
    }

    None
}

/// Locates aapt / aapt2 binary in Android SDK
pub fn find_aapt_binary() -> Option<PathBuf> {
    let sdk_roots = [
        std::env::var("ANDROID_HOME").ok(),
        std::env::var("ANDROID_SDK_ROOT").ok(),
        std::env::var("HOME").ok().map(|h| format!("{h}/Android/Sdk")),
    ];

    for root_opt in sdk_roots.into_iter().flatten() {
        let build_tools_dir = Path::new(&root_opt).join("build-tools");
        if let Ok(entries) = std::fs::read_dir(build_tools_dir) {
            let mut versions: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|ent| ent.path()))
                .filter(|p| p.is_dir())
                .collect();
            // Sort descending so newer build-tools are prioritized
            versions.sort_by(|a, b| b.cmp(a));

            for ver_dir in versions {
                let aapt2 = ver_dir.join("aapt2");
                if aapt2.is_file() {
                    return Some(aapt2);
                }
                let aapt = ver_dir.join("aapt");
                if aapt.is_file() {
                    return Some(aapt);
                }
            }
        }
    }
    None
}

/// Query installed package version info from device using `dumpsys package <pkg>`
pub async fn query_installed_package_info(
    serial: &str,
    package: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "dumpsys", "package", package])
        .output()
        .await;

    let mut version_name = None;
    let mut version_code = None;
    let mut last_update_time = None;

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("versionName=") && version_name.is_none() {
                version_name = Some(trimmed.trim_start_matches("versionName=").to_string());
            } else if trimmed.starts_with("versionCode=") && version_code.is_none() {
                let part = trimmed.split_whitespace().next().unwrap_or(trimmed);
                version_code = Some(part.trim_start_matches("versionCode=").to_string());
            } else if trimmed.starts_with("lastUpdateTime=") && last_update_time.is_none() {
                last_update_time = Some(trimmed.trim_start_matches("lastUpdateTime=").to_string());
            }
        }
    }

    (version_name, version_code, last_update_time)
}

/// Resolve the PID of a package running on a target device
pub async fn get_package_pid(serial: &str, package: &str) -> Option<u32> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "pidof", package])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        text.split_whitespace()
            .next()
            .and_then(|s| s.parse::<u32>().ok())
    } else {
        None
    }
}

/// Launch application on target device via `cmd package resolve-activity` or monkey fallback
pub async fn launch_app_on_target(serial: &str, package: &str) -> Result<String, String> {
    // 1. Resolve launcher activity
    let resolve_out = Command::new("adb")
        .args([
            "-s",
            serial,
            "shell",
            "cmd",
            "package",
            "resolve-activity",
            "--brief",
            package,
        ])
        .output()
        .await;

    let mut launched = false;
    if let Ok(out) = resolve_out {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.contains('/') && !trimmed.starts_with("priority=") {
                let component = if trimmed.starts_with('/') {
                    format!("{package}{trimmed}")
                } else {
                    trimmed.to_string()
                };

                let start_res = Command::new("adb")
                    .args([
                        "-s",
                        serial,
                        "shell",
                        "am",
                        "start",
                        "-n",
                        &component,
                        "-a",
                        "android.intent.action.MAIN",
                        "-c",
                        "android.intent.category.LAUNCHER",
                    ])
                    .output()
                    .await;

                if let Ok(res) = start_res
                    && res.status.success()
                {
                    launched = true;
                    break;
                }
            }
        }
    }

    // 2. Fallback to monkey launch if activity resolution was empty or failed
    if !launched {
        let monkey_res = Command::new("adb")
            .args([
                "-s",
                serial,
                "shell",
                "monkey",
                "-p",
                package,
                "-c",
                "android.intent.category.LAUNCHER",
                "1",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to launch app via monkey: {e}"))?;

        if !monkey_res.status.success() {
            let err = String::from_utf8_lossy(&monkey_res.stderr);
            return Err(format!("App launch failed: {err}"));
        }
    }

    Ok(format!("Launched '{package}' on '{serial}'"))
}

/// Build debug APK and deploy to running target, then launch the app
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

    let detected_package = detect_package_name();

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

    // Step 2: installDebug (targeting the specific device serial)
    let mut install_cmd = Command::new("./gradlew");
    install_cmd
        .args(["installDebug", "--console=plain"])
        .env("ANDROID_SERIAL", &serial)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match install_cmd.output().await {
        Ok(output) if output.status.success() => {
            // Step 3: Launch app and query deployed version metadata
            let mut version_name = None;
            let mut version_code = None;

            if let Some(ref pkg) = detected_package {
                let (v_name, v_code, last_update) =
                    query_installed_package_info(&serial, pkg).await;
                version_name = v_name;
                version_code = v_code;

                let version_summary = match (&version_name, &version_code, &last_update) {
                    (Some(v), Some(c), Some(t)) => format!("v{v} (code {c}) updated at {t}"),
                    (Some(v), Some(c), None) => format!("v{v} (code {c})"),
                    _ => "installed".to_string(),
                };

                let _ = event_tx
                    .send(AppEvent::StatusLog(format!(
                        "App '{pkg}' is updated on '{serial}': {version_summary}"
                    )))
                    .await;

                match launch_app_on_target(&serial, pkg).await {
                    Ok(msg) => {
                        let _ = event_tx.send(AppEvent::StatusLog(msg)).await;
                    }
                    Err(err) => {
                        let _ = event_tx
                            .send(AppEvent::StatusLog(format!("Could not auto-launch app: {err}")))
                            .await;
                    }
                }
            }

            let deploy_msg = if let Some(ref pkg) = detected_package {
                let v_str = version_name.as_deref().unwrap_or("latest");
                format!("Successfully deployed and launched '{pkg}' (version: {v_str}) on '{serial}'.")
            } else {
                format!("Successfully deployed debug APK to target '{serial}'.")
            };

            let _ = event_tx
                .send(AppEvent::ActionFinished {
                    action: "Build & Deploy".to_string(),
                    success: true,
                    message: deploy_msg,
                })
                .await;

            let _ = event_tx
                .send(AppEvent::ApkDeployed {
                    serial: serial.clone(),
                    package_name: detected_package,
                    version_name,
                    version_code,
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

#[cfg(test)]
mod tests {
    use super::detect_package_name;

    #[test]
    fn test_detect_package_name_from_gradle_content() {
        let sample = r#"
            android {
                namespace = "com.example.flocky"
                compileSdk = 35
                defaultConfig {
                    applicationId = "com.example.flocky"
                    versionCode = 1
                    versionName = "1.0"
                }
            }
        "#;
        let mut detected = None;
        for line in sample.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("namespace = \"") {
                let pkg = trimmed.trim_start_matches("namespace = \"").trim_end_matches('"');
                detected = Some(pkg.to_string());
                break;
            }
        }
        assert_eq!(detected, Some("com.example.flocky".to_string()));
        let _ = detect_package_name();
    }
}
