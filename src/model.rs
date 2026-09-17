use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Emulator,
    UsbPhone,
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Emulator => write!(f, "Emulator"),
            TargetType::UsbPhone => write!(f, "Physical (USB)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTarget {
    pub serial: String,
    pub state: String,
    pub target_type: TargetType,
    pub product: Option<String>,
    pub model: Option<String>,
    pub device: Option<String>,
    pub transport_id: Option<String>,
    pub boot_completed: bool,
}

impl DeviceTarget {
    pub fn display_name(&self) -> String {
        if let Some(ref model) = self.model {
            format!("{} ({})", model, self.serial)
        } else {
            self.serial.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvdInfo {
    pub name: String,
    pub is_running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaneFocus {
    #[default]
    InstalledAvds,
    RunningTargets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(usize)]
pub enum ScreenType {
    #[default]
    Emu = 0,
    Build = 1,
    Logs = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TaskStatus {
    #[default]
    Idle,
    Running {
        started_at: Instant,
    },
    Success {
        duration: Duration,
    },
    Failed {
        exit_code: Option<i32>,
        duration: Duration,
    },
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct GradleState {
    pub output_lines: Vec<String>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub status: TaskStatus,
    pub active_task_name: Option<String>,
    pub apk_path: Option<String>,
    pub available_tasks: Vec<(&'static str, &'static str, &'static str)>,
    pub selected_task_index: usize,
}

impl Default for GradleState {
    fn default() -> Self {
        Self {
            output_lines: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            status: TaskStatus::Idle,
            active_task_name: None,
            apk_path: None,
            available_tasks: vec![
                ("assembleDebug", "Build Debug APK", "./gradlew assembleDebug --console=plain"),
                ("installDebug", "Install Debug APK to target", "./gradlew installDebug --console=plain"),
                ("testDebugUnitTest", "Run Unit Tests", "./gradlew testDebugUnitTest --console=plain"),
                ("clean", "Clean build artifacts", "./gradlew clean --console=plain"),
                ("syncDependencies", "Refresh Gradle Dependencies", "./gradlew --refresh-dependencies help --console=plain"),
            ],
            selected_task_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Debug, Clone)]
pub struct DeviceLogSession {
    pub serial: String,
    pub display_name: String,
    pub is_streaming: bool,
    pub lines: Vec<String>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub search_query: String,
    pub log_file_path: PathBuf,
    pub app_package: Option<String>,
    pub app_pid: Option<u32>,
}

impl DeviceLogSession {
    pub fn new(serial: String, display_name: String, log_file_path: PathBuf) -> Self {
        Self {
            serial,
            display_name,
            is_streaming: false,
            lines: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            search_query: String::new(),
            log_file_path,
            app_package: None,
            app_pid: None,
        }
    }

    pub fn filtered_lines(&self) -> Vec<&str> {
        let query = self.search_query.trim();
        if query.is_empty() {
            return self.lines.iter().map(|s| s.as_str()).collect();
        }

        // Support package prefix filter: package:<pkg_name>
        if let Some(target_pkg) = query.strip_prefix("package:") {
            let pkg = target_pkg.trim().to_lowercase();
            let pid_match = self.app_pid.map(|p| p.to_string());

            return self
                .lines
                .iter()
                .filter(|line| {
                    let lower = line.to_lowercase();
                    // Match package name in logcat line
                    if lower.contains(&pkg) {
                        return true;
                    }
                    // Match resolved PID if available
                    if let Some(ref pid) = pid_match {
                        // Check standard Android logcat PID patterns:
                        // 1. "time" format: "Tag(28083):" or "Tag( 1049):"
                        let tag_pid_tight = format!("({pid}):");
                        let tag_pid_space = format!("( {pid}):");
                        let tag_pid_open = format!("({pid})");
                        // 2. "threadtime" / "brief" format: " 28083 "
                        let space_pid = format!(" {pid} ");

                        if line.contains(&tag_pid_tight)
                            || line.contains(&tag_pid_space)
                            || line.contains(&tag_pid_open)
                            || line.contains(&space_pid)
                        {
                            return true;
                        }
                    }
                    false
                })
                .map(|s| s.as_str())
                .collect();
        }

        let q = query.to_lowercase();
        self.lines
            .iter()
            .filter(|line| line.to_lowercase().contains(&q))
            .map(|s| s.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogState {
    pub active_device_serial: Option<String>,
    pub sessions: HashMap<String, DeviceLogSession>,
    pub search_mode: SearchMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_filter_with_text() {
        let mut session = DeviceLogSession::new(
            "emulator-5554".into(),
            "Pixel".into(),
            PathBuf::from("/tmp/test.log"),
        );
        session.lines = vec![
            "09-17 23:00:00.001 1028 1028 D wtasd : sar_limit 1396".into(),
            "09-17 23:00:01.002 25333 25333 I ActivityTaskManager: Start proc com.example.flocky for activity".into(),
            "09-17 23:00:02.003 1191 1523 I SemWifi: scan done".into(),
        ];

        session.search_query = "package:com.example.flocky".into();
        let matches = session.filtered_lines();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].contains("com.example.flocky"));
    }

    #[test]
    fn test_package_filter_with_pid_time_format() {
        let mut session = DeviceLogSession::new(
            "emulator-5554".into(),
            "Pixel".into(),
            PathBuf::from("/tmp/test.log"),
        );
        session.app_pid = Some(28083);
        session.lines = vec![
            "09-17 23:22:28.344 D/BoundBrokerSvc( 8396): onUnbind".into(),
            "09-17 23:22:41.679 I/LocationManager(28083): >>> NEW GPS LOCK".into(),
            "09-17 23:22:42.000 D/UsbStatsMonitor( 1191): 0 0 0 0".into(),
        ];

        session.search_query = "package:com.example.flocky".into();
        let matches = session.filtered_lines();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].contains("LocationManager(28083)"));
    }
}
