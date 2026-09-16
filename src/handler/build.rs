use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::android::gradle::{cancel_current_gradle_task, run_gradle_task};
use crate::app::AppState;
use crate::events::AppEvent;
use crate::model::TaskStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildAction {
    Sync,
    AssembleDebug,
    Clean,
    Cancel,
    ClearLog,
    ToggleAutoScroll,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
}

/// Pure mapping from KeyCode to Build Tab Action
pub fn map_key(key: KeyCode) -> Option<BuildAction> {
    match key {
        KeyCode::Char('s') => Some(BuildAction::Sync),
        KeyCode::Char('b') => Some(BuildAction::AssembleDebug),
        KeyCode::Char('c') => Some(BuildAction::Clean),
        KeyCode::Char('x') => Some(BuildAction::Cancel),
        KeyCode::Char('l') => Some(BuildAction::ClearLog),
        KeyCode::Char('a') => Some(BuildAction::ToggleAutoScroll),
        KeyCode::Up => Some(BuildAction::ScrollUp),
        KeyCode::Down => Some(BuildAction::ScrollDown),
        KeyCode::PageUp => Some(BuildAction::ScrollPageUp),
        KeyCode::PageDown => Some(BuildAction::ScrollPageDown),
        _ => None,
    }
}

/// Dispatches execution of a Build Tab Action
pub fn execute_action(
    action: BuildAction,
    state: &mut AppState,
    tx: &mpsc::Sender<AppEvent>,
) {
    match action {
        BuildAction::Sync => {
            if matches!(state.gradle_state.status, TaskStatus::Running { .. }) {
                state.append_gradle_line(
                    "A Gradle task is already running. Press [x] to cancel before starting a new one."
                        .to_string(),
                );
                return;
            }
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                run_gradle_task(
                    vec!["--refresh-dependencies".into(), "help".into()],
                    "Sync Dependencies".into(),
                    tx_clone,
                )
                .await;
            });
        }

        BuildAction::AssembleDebug => {
            if matches!(state.gradle_state.status, TaskStatus::Running { .. }) {
                state.append_gradle_line(
                    "A Gradle task is already running. Press [x] to cancel before starting a new one."
                        .to_string(),
                );
                return;
            }
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                run_gradle_task(
                    vec!["assembleDebug".into(), "--console=plain".into()],
                    "Assemble Debug".into(),
                    tx_clone,
                )
                .await;
            });
        }

        BuildAction::Clean => {
            if matches!(state.gradle_state.status, TaskStatus::Running { .. }) {
                state.append_gradle_line(
                    "A Gradle task is already running. Press [x] to cancel before starting a new one."
                        .to_string(),
                );
                return;
            }
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                run_gradle_task(
                    vec!["clean".into(), "--console=plain".into()],
                    "Clean Project".into(),
                    tx_clone,
                )
                .await;
            });
        }

        BuildAction::Cancel => {
            if matches!(state.gradle_state.status, TaskStatus::Running { .. }) {
                if cancel_current_gradle_task() {
                    state.append_gradle_line(">>> Cancellation signal sent to running Gradle task...".to_string());
                } else {
                    state.append_gradle_line("No active task could be cancelled.".to_string());
                }
            } else {
                state.append_gradle_line("No Gradle task is currently running.".to_string());
            }
        }

        BuildAction::ClearLog => {
            state.clear_gradle_output();
        }

        BuildAction::ToggleAutoScroll => {
            state.toggle_gradle_auto_scroll();
        }

        BuildAction::ScrollUp => {
            state.scroll_gradle_up(1);
        }

        BuildAction::ScrollDown => {
            state.scroll_gradle_down(1);
        }

        BuildAction::ScrollPageUp => {
            state.scroll_gradle_up(15);
        }

        BuildAction::ScrollPageDown => {
            state.scroll_gradle_down(15);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_key_build_actions() {
        assert_eq!(map_key(KeyCode::Char('s')), Some(BuildAction::Sync));
        assert_eq!(map_key(KeyCode::Char('b')), Some(BuildAction::AssembleDebug));
        assert_eq!(map_key(KeyCode::Char('c')), Some(BuildAction::Clean));
        assert_eq!(map_key(KeyCode::Char('x')), Some(BuildAction::Cancel));
        assert_eq!(map_key(KeyCode::Char('l')), Some(BuildAction::ClearLog));
        assert_eq!(map_key(KeyCode::Char('a')), Some(BuildAction::ToggleAutoScroll));
        assert_eq!(map_key(KeyCode::Up), Some(BuildAction::ScrollUp));
        assert_eq!(map_key(KeyCode::Down), Some(BuildAction::ScrollDown));
        assert_eq!(map_key(KeyCode::PageUp), Some(BuildAction::ScrollPageUp));
        assert_eq!(map_key(KeyCode::PageDown), Some(BuildAction::ScrollPageDown));
        assert_eq!(map_key(KeyCode::Char('z')), None);
    }
}
