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
