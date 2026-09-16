pub mod android;
pub mod app;
pub mod events;
pub mod model;
pub mod ui;

use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::android::deploy::build_and_deploy_apk;
use crate::android::discovery::{list_active_devices, list_installed_avds};
use crate::android::lifecycle::{launch_emulator_headless, launch_scrcpy_for_emulator, stop_emulator};
use crate::app::AppState;
use crate::events::AppEvent;
use crate::model::{PaneFocus, ScreenType, TargetType};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run_app(terminal).await;
    ratatui::restore();
    result
}

async fn run_app(mut terminal: DefaultTerminal) -> Result<()> {
    let mut app_state = AppState::default();
    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(100);

    // Spawn background poller to discover AVDs and ADB targets
    let poller_tx = event_tx.clone();
    tokio::spawn(async move {
        // Initial fetch immediately
        refresh_devices(&poller_tx).await;

        // Periodic polling loop (every 2.5 seconds)
        let mut interval = tokio::time::interval(Duration::from_millis(2500));
        loop {
            interval.tick().await;
            refresh_devices(&poller_tx).await;
        }
    });

    let mut event_reader = crossterm::event::EventStream::new();

    loop {
        // Render UI
        terminal.draw(|frame| ui::render_ui(frame, &app_state))?;

        tokio::select! {
            // Handle background app events
            Some(event) = event_rx.recv() => {
                match event {
                    AppEvent::DevicesRefreshed { avds, devices } => {
                        app_state.update_devices(avds, devices);
                    }
                    AppEvent::StatusLog(msg) => {
                        app_state.add_log(msg);
                    }
                    AppEvent::ActionStarted(msg) => {
                        app_state.is_busy = true;
                        app_state.current_action = Some(msg.clone());
                        app_state.add_log(format!(">>> {msg}"));
                    }
                    AppEvent::ActionFinished { action, success, message } => {
                        app_state.is_busy = false;
                        app_state.current_action = None;
                        if success {
                            app_state.add_log(format!("✔ [{action}] {message}"));
                        } else {
                            app_state.add_log(format!("✘ [{action}] {message}"));
                        }
                    }
                    AppEvent::Tick => {}
                }
            }

            // Handle terminal keyboard events
            Some(Ok(event)) = event_reader.next() => {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        // Navigation & screens
                        KeyCode::Char('q') | KeyCode::Esc => {
                            break;
                        }
                        KeyCode::F(1) => {
                            app_state.screen_type = ScreenType::Emu;
                        }
                        KeyCode::F(2) => {
                            app_state.screen_type = ScreenType::Build;
                        }
                        KeyCode::F(3) => {
                            app_state.screen_type = ScreenType::Logs;
                        }
                        KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                            if app_state.screen_type == ScreenType::Emu {
                                app_state.toggle_focus();
                            }
                        }
                        KeyCode::Up => {
                            if app_state.screen_type == ScreenType::Emu {
                                app_state.select_prev();
                            }
                        }
                        KeyCode::Down => {
                            if app_state.screen_type == ScreenType::Emu {
                                app_state.select_next();
                            }
                        }

                        // Refresh
                        KeyCode::Char('R') => {
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                refresh_devices(&tx).await;
                            });
                            app_state.add_log("Refreshing devices...".to_string());
                        }

                        // Run AVD (Headless)
                        KeyCode::Enter | KeyCode::Char('r') => {
                            if app_state.screen_type == ScreenType::Emu {
                                match app_state.pane_focus {
                                    PaneFocus::InstalledAvds => {
                                        if let Some(avd) = app_state.get_selected_avd() {
                                            let avd_name = avd.name.clone();
                                            let tx = event_tx.clone();
                                            app_state.add_log(format!("Starting headless emulator '{avd_name}' with host GPU..."));
                                            tokio::spawn(async move {
                                                match launch_emulator_headless(&avd_name) {
                                                    Ok(()) => {
                                                        let _ = tx.send(AppEvent::StatusLog(format!("Emulator '{avd_name}' process spawned."))).await;
                                                        tokio::time::sleep(Duration::from_millis(1500)).await;
                                                        refresh_devices(&tx).await;
                                                    }
                                                    Err(err) => {
                                                        let _ = tx.send(AppEvent::StatusLog(format!("Error: {err}"))).await;
                                                    }
                                                }
                                            });
                                        } else {
                                            app_state.add_log("No AVD selected to run.".to_string());
                                        }
                                    }
                                    PaneFocus::RunningTargets => {
                                        app_state.add_log("Selected device is already running. Use [b] to build & deploy or [s] for scrcpy.".to_string());
                                    }
                                }
                            }
                        }

                        // Scrcpy Display: ONLY for Emulator devices!
                        KeyCode::Char('s') => {
                            if app_state.screen_type == ScreenType::Emu {
                                if let Some(target) = app_state.get_selected_device() {
                                    match target.target_type {
                                        TargetType::Emulator => {
                                            let serial = target.serial.clone();
                                            let tx = event_tx.clone();
                                            app_state.add_log(format!("Opening scrcpy display window for emulator '{serial}'..."));
                                            tokio::spawn(async move {
                                                match launch_scrcpy_for_emulator(&serial) {
                                                    Ok(()) => {
                                                        let _ = tx.send(AppEvent::StatusLog(format!("Scrcpy launched for emulator '{serial}'."))).await;
                                                    }
                                                    Err(err) => {
                                                        let _ = tx.send(AppEvent::StatusLog(format!("Failed to launch scrcpy: {err}"))).await;
                                                    }
                                                }
                                            });
                                        }
                                        TargetType::UsbPhone => {
                                            app_state.add_log("Note: Scrcpy is disabled for physical devices (operate device by hand).".to_string());
                                        }
                                    }
                                } else {
                                    app_state.add_log("No target device selected for scrcpy.".to_string());
                                }
                            }
                        }

                        // Stop Emulator: Only for emulators
                        KeyCode::Char('k') => {
                            if app_state.screen_type == ScreenType::Emu {
                                if let Some(target) = app_state.get_selected_device() {
                                    match target.target_type {
                                        TargetType::Emulator => {
                                            let serial = target.serial.clone();
                                            let tx = event_tx.clone();
                                            app_state.add_log(format!("Stopping emulator '{serial}'..."));
                                            tokio::spawn(async move {
                                                match stop_emulator(&serial).await {
                                                    Ok(msg) => {
                                                        let _ = tx.send(AppEvent::StatusLog(msg)).await;
                                                        tokio::time::sleep(Duration::from_millis(1000)).await;
                                                        refresh_devices(&tx).await;
                                                    }
                                                    Err(err) => {
                                                        let _ = tx.send(AppEvent::StatusLog(format!("Error: {err}"))).await;
                                                    }
                                                }
                                            });
                                        }
                                        TargetType::UsbPhone => {
                                            app_state.add_log("Cannot stop physical USB device via emu kill.".to_string());
                                        }
                                    }
                                } else {
                                    app_state.add_log("No target selected to stop.".to_string());
                                }
                            }
                        }

                        // Build and Deploy Debug APK to running target
                        KeyCode::Char('b')
                            if app_state.screen_type == ScreenType::Emu => {
                                if let Some(target) = app_state.get_selected_device() {
                                    let serial = target.serial.clone();
                                    let tx = event_tx.clone();
                                    tokio::spawn(async move {
                                        build_and_deploy_apk(serial, tx).await;
                                    });
                                } else {
                                    app_state.add_log("No running target selected for build & deploy.".to_string());
                                }
                            }

                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

async fn refresh_devices(tx: &mpsc::Sender<AppEvent>) {
    let avds = list_installed_avds().await.unwrap_or_else(|err| {
        let _ = tx.try_send(AppEvent::StatusLog(err));
        Vec::new()
    });

    let devices = list_active_devices().await.unwrap_or_else(|err| {
        let _ = tx.try_send(AppEvent::StatusLog(err));
        Vec::new()
    });

    let _ = tx
        .send(AppEvent::DevicesRefreshed { avds, devices })
        .await;
}
