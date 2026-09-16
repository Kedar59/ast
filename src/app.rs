use std::collections::VecDeque;

use crate::model::{AvdInfo, DeviceTarget, GradleState, PaneFocus, ScreenType};

#[derive(Debug)]
pub struct AppState {
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
}

impl Default for AppState {
    fn default() -> Self {
        let mut logs = VecDeque::with_capacity(200);
        logs.push_back("Welcome to ast (Android Studio TUI).".to_string());
        logs.push_back("Discovering installed AVDs and connected devices...".to_string());

        Self {
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
        }
    }
}

impl AppState {
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
}
