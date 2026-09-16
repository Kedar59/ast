use std::time::Duration;

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::android::deploy::build_and_deploy_apk;
use crate::android::discovery::refresh_all_devices;
use crate::android::lifecycle::{
    launch_emulator_headless, launch_scrcpy_for_emulator, stop_emulator,
};
use crate::app::AppState;
use crate::events::AppEvent;
use crate::model::{PaneFocus, TargetType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmuAction {
    TogglePane,
    SelectNext,
    SelectPrev,
    RefreshDevices,
    RunSelectedAvd,
    LaunchScrcpy,
    StopEmulator,
    BuildAndDeploy,
}

/// Pure mapping from (KeyCode, PaneFocus) to an Emulator Tab Action
pub fn map_key(key: KeyCode, pane: PaneFocus) -> Option<EmuAction> {
    match (pane, key) {
        // Shared navigation
        (_, KeyCode::Tab | KeyCode::Left | KeyCode::Right) => Some(EmuAction::TogglePane),
        (_, KeyCode::Up) => Some(EmuAction::SelectPrev),
        (_, KeyCode::Down) => Some(EmuAction::SelectNext),
        (_, KeyCode::Char('R')) => Some(EmuAction::RefreshDevices),

        // Installed AVDs pane shortcuts
        (PaneFocus::InstalledAvds, KeyCode::Enter | KeyCode::Char('r')) => {
            Some(EmuAction::RunSelectedAvd)
        }

        // Running Targets pane shortcuts
        (PaneFocus::RunningTargets, KeyCode::Char('s')) => Some(EmuAction::LaunchScrcpy),
        (PaneFocus::RunningTargets, KeyCode::Char('k')) => Some(EmuAction::StopEmulator),
        (PaneFocus::RunningTargets, KeyCode::Char('b')) => Some(EmuAction::BuildAndDeploy),

        _ => None,
    }
}

/// Dispatches execution of an Emulator Tab Action
pub fn execute_action(
    action: EmuAction,
    state: &mut AppState,
    tx: &mpsc::Sender<AppEvent>,
) {
    match action {
        EmuAction::TogglePane => state.toggle_focus(),
        EmuAction::SelectNext => state.select_next(),
        EmuAction::SelectPrev => state.select_prev(),

        EmuAction::RefreshDevices => {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                refresh_all_devices(&tx_clone).await;
            });
            state.add_log("Refreshing devices...".to_string());
        }

        EmuAction::RunSelectedAvd => {
            if let Some(avd) = state.get_selected_avd() {
                let avd_name = avd.name.clone();
                let tx_clone = tx.clone();
                state.add_log(format!(
                    "Starting headless emulator '{avd_name}' with host GPU..."
                ));
                tokio::spawn(async move {
                    match launch_emulator_headless(&avd_name) {
                        Ok(()) => {
                            let _ = tx_clone
                                .send(AppEvent::StatusLog(format!(
                                    "Emulator '{avd_name}' process spawned."
                                )))
                                .await;
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                            refresh_all_devices(&tx_clone).await;
                        }
                        Err(err) => {
                            let _ = tx_clone
                                .send(AppEvent::StatusLog(format!("Error: {err}")))
                                .await;
                        }
                    }
                });
            } else {
                state.add_log("No AVD selected to run.".to_string());
            }
        }

        EmuAction::LaunchScrcpy => {
            if let Some(target) = state.get_selected_device() {
                match target.target_type {
                    TargetType::Emulator => {
                        let serial = target.serial.clone();
                        let tx_clone = tx.clone();
                        state.add_log(format!(
                            "Opening scrcpy display window for emulator '{serial}'..."
                        ));
                        tokio::spawn(async move {
                            match launch_scrcpy_for_emulator(&serial) {
                                Ok(()) => {
                                    let _ = tx_clone
                                        .send(AppEvent::StatusLog(format!(
                                            "Scrcpy launched for emulator '{serial}'."
                                        )))
                                        .await;
                                }
                                Err(err) => {
                                    let _ = tx_clone
                                        .send(AppEvent::StatusLog(format!(
                                            "Failed to launch scrcpy: {err}"
                                        )))
                                        .await;
                                }
                            }
                        });
                    }
                    TargetType::UsbPhone => {
                        state.add_log(
                            "Note: Scrcpy is disabled for physical devices (operate device by hand)."
                                .to_string(),
                        );
                    }
                }
            } else {
                state.add_log("No target device selected for scrcpy.".to_string());
            }
        }

        EmuAction::StopEmulator => {
            if let Some(target) = state.get_selected_device() {
                match target.target_type {
                    TargetType::Emulator => {
                        let serial = target.serial.clone();
                        let tx_clone = tx.clone();
                        state.add_log(format!("Stopping emulator '{serial}'..."));
                        tokio::spawn(async move {
                            match stop_emulator(&serial).await {
                                Ok(msg) => {
                                    let _ = tx_clone.send(AppEvent::StatusLog(msg)).await;
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                    refresh_all_devices(&tx_clone).await;
                                }
                                Err(err) => {
                                    let _ = tx_clone
                                        .send(AppEvent::StatusLog(format!("Error: {err}")))
                                        .await;
                                }
                            }
                        });
                    }
                    TargetType::UsbPhone => {
                        state.add_log("Cannot stop physical USB device via emu kill.".to_string());
                    }
                }
            } else {
                state.add_log("No target selected to stop.".to_string());
            }
        }

        EmuAction::BuildAndDeploy => {
            if let Some(target) = state.get_selected_device() {
                let serial = target.serial.clone();
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    build_and_deploy_apk(serial, tx_clone).await;
                });
            } else {
                state.add_log("No running target selected for build & deploy.".to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_key_installed_avds() {
        assert_eq!(
            map_key(KeyCode::Enter, PaneFocus::InstalledAvds),
            Some(EmuAction::RunSelectedAvd)
        );
        assert_eq!(
            map_key(KeyCode::Char('r'), PaneFocus::InstalledAvds),
            Some(EmuAction::RunSelectedAvd)
        );
        assert_eq!(
            map_key(KeyCode::Tab, PaneFocus::InstalledAvds),
            Some(EmuAction::TogglePane)
        );
        assert_eq!(
            map_key(KeyCode::Char('s'), PaneFocus::InstalledAvds),
            None
        );
    }

    #[test]
    fn test_map_key_running_targets() {
        assert_eq!(
            map_key(KeyCode::Char('s'), PaneFocus::RunningTargets),
            Some(EmuAction::LaunchScrcpy)
        );
        assert_eq!(
            map_key(KeyCode::Char('k'), PaneFocus::RunningTargets),
            Some(EmuAction::StopEmulator)
        );
        assert_eq!(
            map_key(KeyCode::Char('b'), PaneFocus::RunningTargets),
            Some(EmuAction::BuildAndDeploy)
        );
        assert_eq!(
            map_key(KeyCode::Enter, PaneFocus::RunningTargets),
            None
        );
    }
}
