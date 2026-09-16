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
    Tick,
}
