use std::collections::VecDeque;
use std::path::PathBuf;

use crate::model::{
    AvdInfo, DeviceLogSession, DeviceTarget, GradleState, LogState, PaneFocus, ScreenType,
};

#[derive(Debug)]
pub struct AppState {
    pub project_dir: PathBuf,
    pub screen_type: ScreenType,
    pub pane_focus: PaneFocus,
    pub installed_avds: Vec<AvdInfo>,
    pub running_devices: Vec<DeviceTarget>,
    pub avd_selected_index: usize,
    pub device_selected_index: usize,
    pub status_logs: VecDeque<String>,
    pub is_busy: bool,
    pub current_action: Option<String>,
    pub gradle_state: GradleState,
    pub log_state: LogState,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl AppState {
    pub fn new(project_dir: PathBuf) -> Self {
        let mut logs = VecDeque::with_capacity(200);
        logs.push_back("Welcome to ast (Android Studio TUI).".to_string());
        logs.push_back(format!("Project root: {}", project_dir.display()));
        logs.push_back("Discovering installed AVDs and connected devices...".to_string());

        Self {
            project_dir,
            screen_type: ScreenType::Emu,
            pane_focus: PaneFocus::InstalledAvds,
            installed_avds: Vec::new(),
            running_devices: Vec::new(),
            avd_selected_index: 0,
            device_selected_index: 0,
            status_logs: logs,
            is_busy: false,
            current_action: None,
            gradle_state: GradleState::default(),
            log_state: LogState::default(),
        }
    }

    pub fn add_log(&mut self, line: String) {
        if self.status_logs.len() >= 200 {
            self.status_logs.pop_front();
        }
        self.status_logs.push_back(line);
    }

    pub fn append_gradle_line(&mut self, line: String) {
        self.gradle_state.output_lines.push(line);
        // If auto-scroll is enabled, keep offset at the bottom
        if self.gradle_state.auto_scroll {
            self.gradle_state.scroll_offset = self.gradle_state.output_lines.len();
        }
    }

    pub fn scroll_gradle_up(&mut self, count: usize) {
        self.gradle_state.auto_scroll = false;
        self.gradle_state.scroll_offset = self.gradle_state.scroll_offset.saturating_sub(count);
    }

    pub fn scroll_gradle_down(&mut self, count: usize) {
        let total = self.gradle_state.output_lines.len();
        self.gradle_state.scroll_offset = (self.gradle_state.scroll_offset + count).min(total);
        if self.gradle_state.scroll_offset >= total {
            self.gradle_state.auto_scroll = true;
        }
    }

    pub fn toggle_gradle_auto_scroll(&mut self) {
        self.gradle_state.auto_scroll = !self.gradle_state.auto_scroll;
        if self.gradle_state.auto_scroll {
            self.gradle_state.scroll_offset = self.gradle_state.output_lines.len();
        }
    }

    pub fn clear_gradle_output(&mut self) {
        self.gradle_state.output_lines.clear();
        self.gradle_state.scroll_offset = 0;
        self.gradle_state.auto_scroll = true;
        self.gradle_state.apk_path = None;
    }

    // --- Logcat State Management ---

    pub fn log_file_path_for_serial(serial: &str) -> PathBuf {
        crate::android::logcat::log_file_path(serial)
    }

    pub fn get_or_create_log_session(
        &mut self,
        serial: &str,
        display_name: &str,
    ) -> &mut DeviceLogSession {
        if !self.log_state.sessions.contains_key(serial) {
            let path = Self::log_file_path_for_serial(serial);
            let mut session = DeviceLogSession::new(serial.to_string(), display_name.to_string(), path.clone());
            // Pre-hydrate existing log lines from disk if file exists
            if let Ok(content) = std::fs::read_to_string(&path) {
                let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                let start_idx = lines.len().saturating_sub(1000);
                session.lines = lines[start_idx..].to_vec();
                session.scroll_offset = session.lines.len();
            }
            self.log_state.sessions.insert(serial.to_string(), session);
        }

        let session = self.log_state.sessions.get_mut(serial).unwrap();
        if !display_name.is_empty() {
            session.display_name = display_name.to_string();
        }
        session
    }

    pub fn active_log_session(&self) -> Option<&DeviceLogSession> {
        self.log_state
            .active_device_serial
            .as_ref()
            .and_then(|s| self.log_state.sessions.get(s))
    }

    pub fn active_log_session_mut(&mut self) -> Option<&mut DeviceLogSession> {
        let serial = self.log_state.active_device_serial.clone()?;
        self.log_state.sessions.get_mut(&serial)
    }

    pub fn append_logcat_line(&mut self, serial: &str, line: String) {
        let session = self.get_or_create_log_session(serial, "");
        session.lines.push(line);
        if session.lines.len() > 10_000 {
            session.lines.drain(0..1_000);
        }
        if session.auto_scroll {
            let total_filtered = session.filtered_lines().len();
            session.scroll_offset = total_filtered;
        }
    }

    pub fn select_log_device(&mut self, serial: &str) {
        self.log_state.active_device_serial = Some(serial.to_string());
    }

    pub fn active_log_device_index(&self) -> usize {
        if let Some(ref active) = self.log_state.active_device_serial {
            self.running_devices
                .iter()
                .position(|d| &d.serial == active)
                .unwrap_or(0)
        } else {
            0
        }
    }

    pub fn select_next_log_device(&mut self) {
        if self.running_devices.is_empty() {
            return;
        }
        let current_idx = self.active_log_device_index();
        let next_idx = (current_idx + 1) % self.running_devices.len();
        let next_serial = self.running_devices[next_idx].serial.clone();
        let next_name = self.running_devices[next_idx].display_name();
        self.get_or_create_log_session(&next_serial, &next_name);
        self.select_log_device(&next_serial);
    }

    pub fn select_prev_log_device(&mut self) {
        if self.running_devices.is_empty() {
            return;
        }
        let current_idx = self.active_log_device_index();
        let prev_idx = if current_idx == 0 {
            self.running_devices.len().saturating_sub(1)
        } else {
            current_idx - 1
        };
        let prev_serial = self.running_devices[prev_idx].serial.clone();
        let prev_name = self.running_devices[prev_idx].display_name();
        self.get_or_create_log_session(&prev_serial, &prev_name);
        self.select_log_device(&prev_serial);
    }

    pub fn select_log_device_by_index(&mut self, idx: usize) {
        if let Some(dev) = self.running_devices.get(idx) {
            let serial = dev.serial.clone();
            let name = dev.display_name();
            self.get_or_create_log_session(&serial, &name);
            self.select_log_device(&serial);
        }
    }

    pub fn scroll_logs_up(&mut self, count: usize) {
        if let Some(session) = self.active_log_session_mut() {
            session.auto_scroll = false;
            session.scroll_offset = session.scroll_offset.saturating_sub(count);
        }
    }

    pub fn scroll_logs_down(&mut self, count: usize) {
        if let Some(session) = self.active_log_session_mut() {
            let total = session.filtered_lines().len();
            session.scroll_offset = (session.scroll_offset + count).min(total);
            if session.scroll_offset >= total {
                session.auto_scroll = true;
            }
        }
    }

    pub fn scroll_logs_top(&mut self) {
        if let Some(session) = self.active_log_session_mut() {
            session.auto_scroll = false;
            session.scroll_offset = 0;
        }
    }

    pub fn scroll_logs_bottom(&mut self) {
        if let Some(session) = self.active_log_session_mut() {
            session.auto_scroll = true;
            session.scroll_offset = session.filtered_lines().len();
        }
    }

    pub fn toggle_logs_auto_scroll(&mut self) {
        if let Some(session) = self.active_log_session_mut() {
            session.auto_scroll = !session.auto_scroll;
            if session.auto_scroll {
                session.scroll_offset = session.filtered_lines().len();
            }
        }
    }

    pub fn clear_active_logs(&mut self) {
        if let Some(session) = self.active_log_session_mut() {
            session.lines.clear();
            session.scroll_offset = 0;
            session.auto_scroll = true;
        }
    }

    pub fn update_search_query(&mut self, query: String) {
        if let Some(session) = self.active_log_session_mut() {
            session.search_query = query;
            let total = session.filtered_lines().len();
            session.scroll_offset = total;
        }
    }

    // --- Emulator & Device Navigation ---

    pub fn select_next(&mut self) {
        match self.pane_focus {
            PaneFocus::InstalledAvds => {
                if !self.installed_avds.is_empty() {
                    self.avd_selected_index =
                        (self.avd_selected_index + 1) % self.installed_avds.len();
                }
            }
            PaneFocus::RunningTargets => {
                if !self.running_devices.is_empty() {
                    self.device_selected_index =
                        (self.device_selected_index + 1) % self.running_devices.len();
                }
            }
        }
    }

    pub fn select_prev(&mut self) {
        match self.pane_focus {
            PaneFocus::InstalledAvds => {
                if !self.installed_avds.is_empty() {
                    if self.avd_selected_index == 0 {
                        self.avd_selected_index = self.installed_avds.len().saturating_sub(1);
                    } else {
                        self.avd_selected_index -= 1;
                    }
                }
            }
            PaneFocus::RunningTargets => {
                if !self.running_devices.is_empty() {
                    if self.device_selected_index == 0 {
                        self.device_selected_index = self.running_devices.len().saturating_sub(1);
                    } else {
                        self.device_selected_index -= 1;
                    }
                }
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        self.pane_focus = match self.pane_focus {
            PaneFocus::InstalledAvds => PaneFocus::RunningTargets,
            PaneFocus::RunningTargets => PaneFocus::InstalledAvds,
        };
    }

    pub fn get_selected_avd(&self) -> Option<&AvdInfo> {
        self.installed_avds.get(self.avd_selected_index)
    }

    pub fn get_selected_device(&self) -> Option<&DeviceTarget> {
        self.running_devices.get(self.device_selected_index)
    }

    pub fn update_devices(&mut self, mut avds: Vec<AvdInfo>, devices: Vec<DeviceTarget>) {
        // Mark AVDs as running if their name matches any device model or serial
        for avd in &mut avds {
            let is_running = devices.iter().any(|d| {
                d.serial.contains(&avd.name)
                    || d.model.as_ref().is_some_and(|m| m.contains(&avd.name))
                    || d.product.as_ref().is_some_and(|p| p.contains(&avd.name))
            });
            avd.is_running = is_running;
        }

        // Initialize log sessions for newly discovered devices
        for dev in &devices {
            let serial = &dev.serial;
            let display_name = dev.display_name();
            self.get_or_create_log_session(serial, &display_name);
        }

        // If active log device is None or no longer valid, default to the first available device
        if (self.log_state.active_device_serial.is_none()
            || !devices.iter().any(|d| Some(&d.serial) == self.log_state.active_device_serial.as_ref()))
            && !devices.is_empty()
        {
            self.log_state.active_device_serial = Some(devices[0].serial.clone());
        }

        self.installed_avds = avds;
        self.running_devices = devices;

        // Keep bounds safe
        if self.avd_selected_index >= self.installed_avds.len() && !self.installed_avds.is_empty() {
            self.avd_selected_index = self.installed_avds.len() - 1;
        }
        if self.device_selected_index >= self.running_devices.len()
            && !self.running_devices.is_empty()
        {
            self.device_selected_index = self.running_devices.len() - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetType;

    #[test]
    fn test_toggle_focus() {
        let mut state = AppState::default();
        assert_eq!(state.pane_focus, PaneFocus::InstalledAvds);
        state.toggle_focus();
        assert_eq!(state.pane_focus, PaneFocus::RunningTargets);
        state.toggle_focus();
        assert_eq!(state.pane_focus, PaneFocus::InstalledAvds);
    }

    #[test]
    fn test_navigation_and_clamping() {
        let mut state = AppState::default();
        let avds = vec![
            AvdInfo {
                name: "Pixel_6a".into(),
                is_running: false,
            },
            AvdInfo {
                name: "medium_phone".into(),
                is_running: false,
            },
        ];
        state.update_devices(avds, vec![]);
        assert_eq!(state.avd_selected_index, 0);

        state.select_next();
        assert_eq!(state.avd_selected_index, 1);
        state.select_next();
        assert_eq!(state.avd_selected_index, 0);

        state.select_prev();
        assert_eq!(state.avd_selected_index, 1);
        state.select_prev();
        assert_eq!(state.avd_selected_index, 0);
    }

    #[test]
    fn test_device_matching_running() {
        let mut state = AppState::default();
        let avds = vec![AvdInfo {
            name: "Pixel_6a".into(),
            is_running: false,
        }];
        let devices = vec![DeviceTarget {
            serial: "emulator-5554".into(),
            state: "device".into(),
            target_type: TargetType::Emulator,
            product: Some("sdk_gphone64_x86_64".into()),
            model: Some("Pixel_6a".into()),
            device: Some("emulator".into()),
            transport_id: Some("1".into()),
            boot_completed: true,
        }];
        state.update_devices(avds, devices);
        assert!(state.installed_avds[0].is_running);
    }

    #[test]
    fn test_gradle_scrolling_and_auto_scroll() {
        let mut state = AppState::default();
        assert!(state.gradle_state.auto_scroll);

        for i in 0..50 {
            state.append_gradle_line(format!("Line {i}"));
        }
        assert_eq!(state.gradle_state.scroll_offset, 50);

        state.scroll_gradle_up(10);
        assert_eq!(state.gradle_state.scroll_offset, 40);
        assert!(!state.gradle_state.auto_scroll);

        state.scroll_gradle_down(10);
        assert_eq!(state.gradle_state.scroll_offset, 50);
        assert!(state.gradle_state.auto_scroll);
    }

    #[test]
    fn test_log_sessions_and_persistence() {
        let mut state = AppState::default();
        let devices = vec![
            DeviceTarget {
                serial: "mock-emu-001".into(),
                state: "device".into(),
                target_type: TargetType::Emulator,
                product: None,
                model: Some("Pixel_6a".into()),
                device: None,
                transport_id: None,
                boot_completed: true,
            },
            DeviceTarget {
                serial: "mock-phone-002".into(),
                state: "device".into(),
                target_type: TargetType::UsbPhone,
                product: None,
                model: Some("Galaxy S21".into()),
                device: None,
                transport_id: None,
                boot_completed: true,
            },
        ];
        state.update_devices(vec![], devices);

        assert_eq!(state.log_state.active_device_serial.as_deref(), Some("mock-emu-001"));

        // Append logs to emulator
        state.append_logcat_line("mock-emu-001", "E/Test: Emulator log 1".into());
        state.append_logcat_line("mock-emu-001", "I/Test: Emulator log 2".into());

        // Append logs to physical phone
        state.append_logcat_line("mock-phone-002", "D/Test: Phone log 1".into());

        // Switch to physical phone
        state.select_next_log_device();
        assert_eq!(state.log_state.active_device_serial.as_deref(), Some("mock-phone-002"));
        assert_eq!(state.active_log_session().unwrap().lines.len(), 1);

        // Switch back to emulator - verify preservation
        state.select_prev_log_device();
        assert_eq!(state.log_state.active_device_serial.as_deref(), Some("mock-emu-001"));
        assert_eq!(state.active_log_session().unwrap().lines.len(), 2);
    }

    #[test]
    fn test_log_search_filter() {
        let mut state = AppState::default();
        state.get_or_create_log_session("mock-device-filter", "Pixel");
        state.select_log_device("mock-device-filter");

        state.append_logcat_line("mock-device-filter", "09-17 12:00:00.001 123 456 D App: Starting...".into());
        state.append_logcat_line("mock-device-filter", "09-17 12:00:01.002 123 456 E Crash: FatalException occurred".into());
        state.append_logcat_line("mock-device-filter", "09-17 12:00:02.003 123 456 I App: Shutdown".into());

        let session = state.active_log_session().unwrap();
        assert_eq!(session.filtered_lines().len(), 3);

        state.update_search_query("Crash".into());
        let session = state.active_log_session().unwrap();
        assert_eq!(session.filtered_lines().len(), 1);
        assert!(session.filtered_lines()[0].contains("FatalException"));
    }
}
