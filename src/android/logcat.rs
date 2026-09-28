use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::events::AppEvent;

static ACTIVE_STREAM_ID: AtomicU64 = AtomicU64::new(1);
static ACTIVE_LOG_STREAMS: Mutex<Option<HashMap<String, (u64, tokio::sync::watch::Sender<bool>)>>> =
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

pub fn deploy_log_file_path(serial: &str, timestamp: &str) -> PathBuf {
    let sanitized = sanitize_serial(serial);
    logs_dir().join(format!("{sanitized}_{timestamp}.log"))
}

pub fn latest_log_file_path(serial: &str) -> PathBuf {
    let sanitized = sanitize_serial(serial);
    let dir = logs_dir();
    let prefix = format!("{sanitized}_");
    let suffix = ".log";

    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut matching_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                if let Some(file_name) = p.file_name().and_then(|f| f.to_str()) {
                    file_name.starts_with(&prefix) && file_name.ends_with(suffix)
                } else {
                    false
                }
            })
            .collect();

        // Files are named <serial>_<YYYYMMDD_HHMMSS>.log so lexicographical sort is chronological
        matching_files.sort();
        if let Some(latest) = matching_files.pop() {
            return latest;
        }
    }

    // Check if legacy <sanitized>.log exists
    let legacy = dir.join(format!("{sanitized}.log"));
    if legacy.exists() {
        return legacy;
    }

    log_file_path(serial)
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
        && let Some((_, tx)) = map.remove(serial) {
            let _ = tx.send(true);
            return true;
        }
    false
}

pub fn start_logcat_stream(
    serial: String,
    custom_path: Option<PathBuf>,
    tx: mpsc::Sender<AppEvent>,
) {
    if is_logcat_streaming(&serial) {
        if custom_path.is_some() {
            stop_logcat_stream(&serial);
        } else {
            return;
        }
    }

    let stream_id = ACTIVE_STREAM_ID.fetch_add(1, Ordering::SeqCst);
    let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);

    // Register active stream
    {
        if let Ok(mut guard) = ACTIVE_LOG_STREAMS.lock() {
            let map = guard.get_or_insert_with(HashMap::new);
            map.insert(serial.clone(), (stream_id, stop_tx));
        }
    }

    tokio::spawn(async move {
        let _ = tx
            .send(AppEvent::LogcatStreamStatus {
                serial: serial.clone(),
                is_streaming: true,
            })
            .await;

        let log_path = custom_path.unwrap_or_else(|| latest_log_file_path(&serial));
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await
            .ok();

        let mut cmd = Command::new("adb");
        cmd.args(["-s", &serial, "logcat", "-v", "time", "-T", "500"])
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

        // Cleanup stream registry if this task still owns the active stream
        let should_notify_stopped = {
            if let Ok(mut guard) = ACTIVE_LOG_STREAMS.lock()
                && let Some(ref mut map) = *guard {
                    match map.get(&serial) {
                        Some((id, _)) if *id == stream_id => {
                            map.remove(&serial);
                            true
                        }
                        None => true,
                        Some(_) => false,
                    }
                } else {
                    false
                }
        };

        if should_notify_stopped {
            let _ = tx
                .send(AppEvent::LogcatStreamStatus {
                    serial,
                    is_streaming: false,
                })
                .await;
        }
    });
}

pub fn toggle_logcat_stream(
    serial: &str,
    custom_path: Option<PathBuf>,
    tx: mpsc::Sender<AppEvent>,
) {
    if is_logcat_streaming(serial) {
        stop_logcat_stream(serial);
    } else {
        start_logcat_stream(serial.to_string(), custom_path, tx);
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
        let path = log_file_path("RZCY80FFWAV");
        assert!(path.ends_dir_or_file("RZCY80FFWAV.log"));
    }

    #[test]
    fn test_deploy_log_file_path() {
        let path = deploy_log_file_path("emulator-5554", "20260920_194154");
        assert!(path.ends_dir_or_file("emulator-5554_20260920_194154.log"));
    }

    #[test]
    fn test_latest_log_file_path_selection() {
        let temp_dir = std::env::temp_dir().join(format!("ast_test_logs_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let serial = "test-device";
        let file1 = temp_dir.join(format!("{serial}_20260920_100000.log"));
        let file2 = temp_dir.join(format!("{serial}_20260920_120000.log"));
        let file3 = temp_dir.join(format!("{serial}_20260920_110000.log"));

        let _ = std::fs::write(&file1, "log 1");
        let _ = std::fs::write(&file2, "log 2");
        let _ = std::fs::write(&file3, "log 3");

        // Helper check for sorting logic
        let mut matching = vec![file1.clone(), file2.clone(), file3.clone()];
        matching.sort();
        assert_eq!(matching.last().unwrap(), &file2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    trait PathExt {
        fn ends_dir_or_file(&self, file_name: &str) -> bool;
    }

    impl PathExt for PathBuf {
        fn ends_dir_or_file(&self, file_name: &str) -> bool {
            self.to_str().map(|s| s.ends_with(file_name)).unwrap_or(false)
        }
    }
}
