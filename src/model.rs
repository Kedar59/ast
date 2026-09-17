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
        }
    }

    pub fn filtered_lines(&self) -> Vec<&str> {
        if self.search_query.is_empty() {
            self.lines.iter().map(|s| s.as_str()).collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.lines
                .iter()
                .filter(|line| line.to_lowercase().contains(&q))
                .map(|s| s.as_str())
                .collect()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogState {
    pub active_device_serial: Option<String>,
    pub sessions: HashMap<String, DeviceLogSession>,
    pub search_mode: SearchMode,
}
