use std::time::Duration;

use crate::model::{AvdInfo, DeviceTarget};

#[derive(Debug)]
pub enum AppEvent {
    DevicesRefreshed {
        avds: Vec<AvdInfo>,
        devices: Vec<DeviceTarget>,
    },
    StatusLog(String),
    ActionStarted(String),
    ActionFinished {
        action: String,
        success: bool,
        message: String,
    },
    GradleLogLine(String),
    GradleTaskStarted(String),
    GradleTaskFinished {
        task: String,
        success: bool,
        exit_code: Option<i32>,
        duration: Duration,
        apk_path: Option<String>,
    },
    GradleTaskCancelled,
    ApkDeployed {
        serial: String,
    },
    LogcatLine {
        serial: String,
        line: String,
    },
    LogcatStreamStatus {
        serial: String,
        is_streaming: bool,
    },
    Tick,
}
