use std::process::Stdio;
use tokio::process::Command;

/// Launch emulator headless with host GPU acceleration (detached)
pub fn launch_emulator_headless(avd_name: &str) -> Result<(), String> {
    // Spawn detached process
    let mut cmd = std::process::Command::new("emulator");
    cmd.args([
        "-avd",
        avd_name,
        "-no-window",
        "-no-audio",
        "-no-boot-anim",
        "-gpu",
        "host",
    ]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd.stdin(Stdio::null());

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn emulator '{avd_name}': {e}"))?;

    Ok(())
}

/// Launch scrcpy display mirroring for an emulator target
pub fn launch_scrcpy_for_emulator(serial: &str) -> Result<(), String> {
    let title = format!("ast - {serial}");
    let mut cmd = std::process::Command::new("scrcpy");
    cmd.args([
        "-s",
        serial,
        "--window-title",
        &title,
        "--stay-awake",
        "--mouse=sdk",
        "--keyboard=sdk",
    ]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd.stdin(Stdio::null());

    cmd.spawn()
        .map_err(|e| format!("Failed to launch scrcpy for '{serial}': {e}"))?;

    Ok(())
}

/// Stop a running emulator using `adb emu kill`
pub async fn stop_emulator(serial: &str) -> Result<String, String> {
    let output = Command::new("adb")
        .args(["-s", serial, "emu", "kill"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run 'adb emu kill': {e}"))?;

    if output.status.success() {
        Ok(format!("Emulator '{serial}' stopped successfully."))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error stopping emulator '{serial}': {err}"))
    }
}

/// Check if boot is completed for device
pub async fn is_boot_completed(serial: &str) -> bool {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "getprop", "sys.boot_completed"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;

    if let Ok(out) = output {
        String::from_utf8_lossy(&out.stdout).trim() == "1"
    } else {
        false
    }
}
