use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::events::AppEvent;
use crate::model::{AvdInfo, DeviceTarget, TargetType};

/// List all installed virtual devices using `emulator -list-avds`
pub async fn list_installed_avds() -> Result<Vec<AvdInfo>, String> {
    let output = Command::new("emulator")
        .arg("-list-avds")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run 'emulator -list-avds': {e}"))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr);
        return Err(format!("'emulator -list-avds' exited with error: {err_str}"));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    Ok(parse_avd_output(&stdout_str))
}

pub fn parse_avd_output(output: &str) -> Vec<AvdInfo> {
    let mut avds = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            avds.push(AvdInfo {
                name: trimmed.to_string(),
                is_running: false,
            });
        }
    }
    avds
}

/// List all active USB devices and emulators using `adb devices -l`
pub async fn list_active_devices() -> Result<Vec<DeviceTarget>, String> {
    let output = Command::new("adb")
        .args(["devices", "-l"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run 'adb devices -l': {e}"))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr);
        return Err(format!("'adb devices -l' error: {err_str}"));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout_str.lines() {
        if let Some(mut target) = parse_adb_device_line(line) {
            target.boot_completed = check_boot_completed_fast(&target.serial).await;
            devices.push(target);
        }
    }

    Ok(devices)
}

/// Convenient helper to query both AVDs and active targets and send event
pub async fn refresh_all_devices(tx: &mpsc::Sender<AppEvent>) {
    let avds = list_installed_avds().await.unwrap_or_else(|err| {
        let _ = tx.try_send(AppEvent::StatusLog(err));
        Vec::new()
    });

    let devices = list_active_devices().await.unwrap_or_else(|err| {
        let _ = tx.try_send(AppEvent::StatusLog(err));
        Vec::new()
    });

    let _ = tx.send(AppEvent::DevicesRefreshed { avds, devices }).await;
}

pub fn parse_adb_device_line(line: &str) -> Option<DeviceTarget> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("List of devices attached")
        || trimmed.starts_with("* daemon")
    {
        return None;
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let serial = parts[0].to_string();
    let state = parts[1].to_string();

    let mut product = None;
    let mut model = None;
    let mut device = None;
    let mut transport_id = None;
    let mut has_usb = false;

    for part in &parts[2..] {
        if part.starts_with("usb:") {
            has_usb = true;
        } else if let Some(val) = part.strip_prefix("product:") {
            product = Some(val.to_string());
        } else if let Some(val) = part.strip_prefix("model:") {
            model = Some(val.to_string());
        } else if let Some(val) = part.strip_prefix("device:") {
            device = Some(val.to_string());
        } else if let Some(val) = part.strip_prefix("transport_id:") {
            transport_id = Some(val.to_string());
        }
    }

    let is_emulator = serial.starts_with("emulator-")
        || product.as_ref().is_some_and(|p| {
            p.contains("sdk") || p.contains("goldfish") || p.contains("ranchu")
        })
        || (!has_usb && !serial.chars().all(|c| c.is_ascii_alphanumeric() && !c.is_numeric()));

    let target_type = if is_emulator {
        TargetType::Emulator
    } else {
        TargetType::UsbPhone
    };

    Some(DeviceTarget {
        serial,
        state,
        target_type,
        product,
        model,
        device,
        transport_id,
        boot_completed: false,
    })
}

async fn check_boot_completed_fast(serial: &str) -> bool {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "getprop", "sys.boot_completed"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;

    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        s.trim() == "1"
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_avd_output() {
        let raw = "Pixel_6a\nmedium_phone\n\n";
        let avds = parse_avd_output(raw);
        assert_eq!(avds.len(), 2);
        assert_eq!(avds[0].name, "Pixel_6a");
        assert_eq!(avds[1].name, "medium_phone");
    }

    #[test]
    fn test_parse_adb_physical_phone() {
        let line = "RZCY80FFWAV            device usb:1-2 product:a17xins model:SM_A176B device:a17x transport_id:6";
        let dev = parse_adb_device_line(line).expect("Should parse physical phone");
        assert_eq!(dev.serial, "RZCY80FFWAV");
        assert_eq!(dev.state, "device");
        assert_eq!(dev.target_type, TargetType::UsbPhone);
        assert_eq!(dev.model.as_deref(), Some("SM_A176B"));
        assert_eq!(dev.transport_id.as_deref(), Some("6"));
    }

    #[test]
    fn test_parse_adb_emulator() {
        let line = "emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emulator64_x86_64 transport_id:1";
        let dev = parse_adb_device_line(line).expect("Should parse emulator");
        assert_eq!(dev.serial, "emulator-5554");
        assert_eq!(dev.state, "device");
        assert_eq!(dev.target_type, TargetType::Emulator);
    }
}
