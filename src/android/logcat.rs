use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::events::AppEvent;

static ACTIVE_LOG_STREAMS: Mutex<Option<HashMap<String, tokio::sync::watch::Sender<bool>>>> =
    Mutex::new(None);

pub fn logs_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let dir = home.join(".ast").join("logs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn sanitize_serial(serial: &str) -> String {
    serial
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
}

pub fn log_file_path(serial: &str) -> PathBuf {
    let sanitized = sanitize_serial(serial);
    logs_dir().join(format!("{sanitized}.log"))
}

pub fn is_logcat_streaming(serial: &str) -> bool {
    if let Ok(guard) = ACTIVE_LOG_STREAMS.lock()
        && let Some(ref map) = *guard {
            return map.contains_key(serial);
        }
    false
}

pub fn stop_logcat_stream(serial: &str) -> bool {
    if let Ok(mut guard) = ACTIVE_LOG_STREAMS.lock()
        && let Some(ref mut map) = *guard
        && let Some(tx) = map.remove(serial) {
            let _ = tx.send(true);
            return true;
        }
    false
}

pub fn start_logcat_stream(serial: String, tx: mpsc::Sender<AppEvent>) {
    if is_logcat_streaming(&serial) {
        return;
    }

    let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);

    {
        if let Ok(mut guard) = ACTIVE_LOG_STREAMS.lock() {
            let map = guard.get_or_insert_with(HashMap::new);
            map.insert(serial.clone(), stop_tx);
        }
    }

    tokio::spawn(async move {
        let _ = tx
            .send(AppEvent::LogcatStreamStatus {
                serial: serial.clone(),
                is_streaming: true,
            })
            .await;

        let log_path = log_file_path(&serial);
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await
            .ok();

        let mut cmd = Command::new("adb");
        cmd.args(["-s", &serial, "logcat", "-v", "time"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(stdout) = child.stdout.take() {
                    let mut reader = BufReader::new(stdout).lines();

                    loop {
                        tokio::select! {
                            _ = stop_rx.changed() => {
                                let _ = child.kill().await;
                                break;
                            }
                            line_res = reader.next_line() => {
                                match line_res {
                                    Ok(Some(line)) => {
                                        // Write to disk
                                        if let Some(ref mut f) = file {
                                            let _ = f.write_all(format!("{line}\n").as_bytes()).await;
                                            let _ = f.flush().await;
                                        }
                                        // Dispatch to UI
                                        let _ = tx.send(AppEvent::LogcatLine {
                                            serial: serial.clone(),
                                            line,
                                        }).await;
                                    }
                                    _ => break,
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx
                    .send(AppEvent::StatusLog(format!(
                        "Failed to start logcat for '{serial}': {e}"
                    )))
                    .await;
            }
        }

        // Cleanup stream registry
        {
            if let Ok(mut guard) = ACTIVE_LOG_STREAMS.lock()
                && let Some(ref mut map) = *guard {
                    map.remove(&serial);
                }
        }

        let _ = tx
            .send(AppEvent::LogcatStreamStatus {
                serial,
                is_streaming: false,
            })
            .await;
    });
}

pub fn toggle_logcat_stream(serial: &str, tx: mpsc::Sender<AppEvent>) {
    if is_logcat_streaming(serial) {
        stop_logcat_stream(serial);
    } else {
        start_logcat_stream(serial.to_string(), tx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_serial() {
        assert_eq!(sanitize_serial("emulator-5554"), "emulator-5554");
        assert_eq!(sanitize_serial("192.168.1.100:5555"), "192.168.1.100_5555");
        assert_eq!(sanitize_serial("RZCY80FFWAV"), "RZCY80FFWAV");
    }

    #[test]
    fn test_log_file_path() {
        let path = log_file_path("emulator-5554");
        assert!(path.to_string_lossy().ends_with(".ast/logs/emulator-5554.log"));
    }
}
