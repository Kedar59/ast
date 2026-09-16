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
use crate::app::AppState;
use crate::events::AppEvent;
use crate::model::{ScreenType, TaskStatus};

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
    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(100);

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
                                // Delegated to handler::logs when implemented
                            }
                        },
                    }
                }
            }
        }
    }

    Ok(())
}
