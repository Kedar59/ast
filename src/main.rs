pub mod android;
pub mod app;
pub mod events;
pub mod handler;
pub mod model;
pub mod ui;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::android::discovery::refresh_all_devices;
use crate::android::logcat::start_logcat_stream;
use crate::app::AppState;
use crate::events::AppEvent;
use crate::model::{ScreenType, SearchMode, TaskStatus};

#[derive(Parser, Debug)]
#[command(name = "ast", version, about = "Android Studio TUI")]
pub struct Cli {
    /// Path to Android project root containing gradlew (defaults to current directory)
    #[arg(short = 'p', long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let project_dir = std::fs::canonicalize(&cli.project_path)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to access project path '{}': {}", cli.project_path.display(), e))?;

    if !project_dir.is_dir() {
        return Err(color_eyre::eyre::eyre!("Specified project path '{}' is not a directory", project_dir.display()));
    }

    std::env::set_current_dir(&project_dir)?;

    let terminal = ratatui::init();
    let result = run_app(terminal, project_dir).await;
    ratatui::restore();
    result
}

async fn run_app(mut terminal: DefaultTerminal, project_dir: PathBuf) -> Result<()> {
    let mut app_state = AppState::new(project_dir);
    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(2000);

    // Spawn background poller to discover AVDs and ADB targets
    let poller_tx = event_tx.clone();
    tokio::spawn(async move {
        // Initial fetch immediately
        refresh_all_devices(&poller_tx).await;

        // Periodic polling loop (every 2.5 seconds)
        let mut interval = tokio::time::interval(Duration::from_millis(2500));
        loop {
            interval.tick().await;
            refresh_all_devices(&poller_tx).await;
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
                    AppEvent::GradleLogLine(line) => {
                        app_state.append_gradle_line(line);
                    }
                    AppEvent::GradleTaskStarted(name) => {
                        app_state.gradle_state.active_task_name = Some(name);
                        app_state.gradle_state.status = TaskStatus::Running {
                            started_at: std::time::Instant::now(),
                        };
                    }
                    AppEvent::GradleTaskFinished {
                        task: _,
                        success,
                        exit_code,
                        duration,
                        apk_path,
                    } => {
                        if success {
                            app_state.gradle_state.status = TaskStatus::Success { duration };
                        } else {
                            app_state.gradle_state.status = TaskStatus::Failed {
                                exit_code,
                                duration,
                            };
                        }
                        if let Some(path) = apk_path {
                            app_state.gradle_state.apk_path = Some(path);
                        }
                    }
                    AppEvent::GradleTaskCancelled => {
                        app_state.gradle_state.status = TaskStatus::Cancelled;
                    }
                    AppEvent::ApkDeployed {
                        serial,
                        package_name,
                        version_name,
                        version_code,
                    } => {
                        app_state.screen_type = ScreenType::Logs;
                        app_state.select_log_device(&serial);
                        let dev_name = app_state
                            .running_devices
                            .iter()
                            .find(|d| d.serial == serial)
                            .map(|d| d.display_name())
                            .unwrap_or_else(|| serial.clone());
                        let session = app_state.get_or_create_log_session(&serial, &dev_name);

                        // If package was detected, set filter to package:<pkg> if query is currently empty
                        if let Some(ref pkg) = package_name {
                            session.app_package = Some(pkg.clone());
                            if session.search_query.is_empty() {
                                session.search_query = format!("package:{pkg}");
                            }
                        }

                        // Polling task to resolve app PID after launch so logs can filter by PID
                        if let Some(ref pkg) = package_name {
                            let tx = event_tx.clone();
                            let s = serial.clone();
                            let p = pkg.clone();
                            tokio::spawn(async move {
                                for attempt in 0..10 {
                                    tokio::time::sleep(Duration::from_millis(if attempt == 0 { 200 } else { 500 })).await;
                                    if let Some(pid) = crate::android::deploy::get_package_pid(&s, &p).await {
                                        let _ = tx
                                            .send(AppEvent::AppPidResolved {
                                                serial: s.clone(),
                                                package_name: p.clone(),
                                                pid,
                                            })
                                            .await;
                                        break;
                                    }
                                }
                            });
                        }

                        start_logcat_stream(serial.clone(), event_tx.clone());

                        let ver_label = match (version_name, version_code) {
                            (Some(v), Some(c)) => format!(" (v{v}, code {c})"),
                            (Some(v), None) => format!(" (v{v})"),
                            _ => String::new(),
                        };
                        app_state.add_log(format!(
                            "APK deployed{ver_label} to '{serial}'. Streaming logcat in Logs tab."
                        ));
                    }
                    AppEvent::AppPidResolved { serial, package_name, pid } => {
                        if let Some(session) = app_state.log_state.sessions.get_mut(&serial) {
                            session.app_pid = Some(pid);
                            // Auto-scroll to bottom of newly matched lines
                            if session.auto_scroll {
                                session.scroll_offset = session.filtered_lines().len();
                            }
                        }
                        app_state.add_log(format!("Active process for '{package_name}' attached [PID: {pid}]."));
                    }
                    AppEvent::LogcatLine { serial, line } => {
                        let is_active_device = app_state
                            .log_state
                            .active_device_serial
                            .as_deref()
                            == Some(&serial);

                        app_state.append_logcat_line(&serial, line);

                        // Batch drain up to 200 pending lines so we don't redraw per individual line
                        let mut drained = 0;
                        while drained < 200 {
                            match event_rx.try_recv() {
                                Ok(AppEvent::LogcatLine { serial: s, line: l }) => {
                                    app_state.append_logcat_line(&s, l);
                                    drained += 1;
                                }
                                Ok(other_evt) => {
                                    match other_evt {
                                        AppEvent::DevicesRefreshed { avds, devices } => {
                                            app_state.update_devices(avds, devices);
                                        }
                                        AppEvent::StatusLog(msg) => {
                                            app_state.add_log(msg);
                                        }
                                        AppEvent::AppPidResolved { serial: s, package_name, pid } => {
                                            if let Some(session) = app_state.log_state.sessions.get_mut(&s) {
                                                session.app_pid = Some(pid);
                                                if session.auto_scroll {
                                                    session.scroll_offset = session.filtered_lines().len();
                                                }
                                            }
                                            app_state.add_log(format!("Active process for '{package_name}' attached [PID: {pid}]."));
                                        }
                                        AppEvent::LogcatStreamStatus { serial: s, is_streaming } => {
                                            if let Some(session) = app_state.log_state.sessions.get_mut(&s) {
                                                session.is_streaming = is_streaming;
                                            }
                                        }
                                        _ => {}
                                    }
                                    break;
                                }
                                Err(_) => break,
                            }
                        }

                        // Only redraw if this is the active log tab and screen is Logs
                        if !is_active_device || app_state.screen_type != ScreenType::Logs {
                            continue;
                        }
                    }
                    AppEvent::LogcatStreamStatus { serial, is_streaming } => {
                        if let Some(session) = app_state.log_state.sessions.get_mut(&serial) {
                            session.is_streaming = is_streaming;
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

                    // In Logs tab search editing mode, capture all keystrokes into search input
                    if app_state.screen_type == ScreenType::Logs
                        && app_state.log_state.search_mode == SearchMode::Editing
                    {
                        if let Some(action) = handler::logs::map_key(key.code, SearchMode::Editing) {
                            handler::logs::execute_action(action, &mut app_state, &event_tx);
                        }
                        continue;
                    }

                    match key.code {
                        // 1. Global navigation & application exit
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::F(1) => app_state.screen_type = ScreenType::Emu,
                        KeyCode::F(2) => app_state.screen_type = ScreenType::Build,
                        KeyCode::F(3) => app_state.screen_type = ScreenType::Logs,

                        // 2. Delegate to active tab handler
                        other => match app_state.screen_type {
                            ScreenType::Emu => {
                                if let Some(action) = handler::emulator::map_key(other, app_state.pane_focus) {
                                    handler::emulator::execute_action(action, &mut app_state, &event_tx);
                                }
                            }
                            ScreenType::Build => {
                                if let Some(action) = handler::build::map_key(other) {
                                    handler::build::execute_action(action, &mut app_state, &event_tx);
                                }
                            }
                            ScreenType::Logs => {
                                if let Some(action) = handler::logs::map_key(other, app_state.log_state.search_mode) {
                                    handler::logs::execute_action(action, &mut app_state, &event_tx);
                                }
                            }
                        },
                    }
                }
            }
        }
    }

    Ok(())
}
