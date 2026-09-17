use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::android::logcat::toggle_logcat_stream;
use crate::app::AppState;
use crate::events::AppEvent;
use crate::model::SearchMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogsAction {
    EnterSearch,
    ExitSearch,
    SearchChar(char),
    SearchBackspace,
    ClearSearch,
    NextDevice,
    PrevDevice,
    SelectDeviceIndex(usize),
    ToggleAutoScroll,
    ToggleStream,
    ClearScreen,
    ScrollUp(usize),
    ScrollDown(usize),
    ScrollTop,
    ScrollBottom,
}

/// Pure mapping from (KeyCode, SearchMode) to a Logs Tab Action
pub fn map_key(key: KeyCode, search_mode: SearchMode) -> Option<LogsAction> {
    match search_mode {
        SearchMode::Editing => match key {
            KeyCode::Esc | KeyCode::Enter => Some(LogsAction::ExitSearch),
            KeyCode::Backspace => Some(LogsAction::SearchBackspace),
            KeyCode::Char(c) => Some(LogsAction::SearchChar(c)),
            _ => None,
        },
        SearchMode::Normal => match key {
            KeyCode::Char('/') => Some(LogsAction::EnterSearch),
            KeyCode::Char('c') => Some(LogsAction::ClearSearch),
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => Some(LogsAction::NextDevice),
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => Some(LogsAction::PrevDevice),
            KeyCode::Char(d @ '1'..='9') => {
                let idx = d.to_digit(10).unwrap() as usize - 1;
                Some(LogsAction::SelectDeviceIndex(idx))
            }
            KeyCode::Char('a') => Some(LogsAction::ToggleAutoScroll),
            KeyCode::Char('k') => Some(LogsAction::ToggleStream),
            KeyCode::Char('x') => Some(LogsAction::ClearScreen),
            KeyCode::Up => Some(LogsAction::ScrollUp(1)),
            KeyCode::Down => Some(LogsAction::ScrollDown(1)),
            KeyCode::PageUp => Some(LogsAction::ScrollUp(15)),
            KeyCode::PageDown => Some(LogsAction::ScrollDown(15)),
            KeyCode::Home => Some(LogsAction::ScrollTop),
            KeyCode::End => Some(LogsAction::ScrollBottom),
            _ => None,
        },
    }
}

/// Dispatches execution of a Logs Tab Action
pub fn execute_action(
    action: LogsAction,
    state: &mut AppState,
    tx: &mpsc::Sender<AppEvent>,
) {
    match action {
        LogsAction::EnterSearch => {
            state.log_state.search_mode = SearchMode::Editing;
        }
        LogsAction::ExitSearch => {
            state.log_state.search_mode = SearchMode::Normal;
        }
        LogsAction::SearchChar(c) => {
            if let Some(session) = state.active_log_session_mut() {
                session.search_query.push(c);
                let total = session.filtered_lines().len();
                session.scroll_offset = total;
            }
        }
        LogsAction::SearchBackspace => {
            if let Some(session) = state.active_log_session_mut() {
                session.search_query.pop();
                let total = session.filtered_lines().len();
                session.scroll_offset = total;
            }
        }
        LogsAction::ClearSearch => {
            if let Some(session) = state.active_log_session_mut() {
                session.search_query.clear();
                let total = session.filtered_lines().len();
                session.scroll_offset = total;
            }
        }
        LogsAction::NextDevice => {
            state.select_next_log_device();
        }
        LogsAction::PrevDevice => {
            state.select_prev_log_device();
        }
        LogsAction::SelectDeviceIndex(idx) => {
            state.select_log_device_by_index(idx);
        }
        LogsAction::ToggleAutoScroll => {
            state.toggle_logs_auto_scroll();
        }
        LogsAction::ToggleStream => {
            if let Some(session) = state.active_log_session() {
                let serial = session.serial.clone();
                let tx_clone = tx.clone();
                toggle_logcat_stream(&serial, tx_clone);
            }
        }
        LogsAction::ClearScreen => {
            state.clear_active_logs();
        }
        LogsAction::ScrollUp(count) => {
            state.scroll_logs_up(count);
        }
        LogsAction::ScrollDown(count) => {
            state.scroll_logs_down(count);
        }
        LogsAction::ScrollTop => {
            state.scroll_logs_top();
        }
        LogsAction::ScrollBottom => {
            state.scroll_logs_bottom();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_key_editing_mode() {
        assert_eq!(
            map_key(KeyCode::Char('a'), SearchMode::Editing),
            Some(LogsAction::SearchChar('a'))
        );
        assert_eq!(
            map_key(KeyCode::Backspace, SearchMode::Editing),
            Some(LogsAction::SearchBackspace)
        );
        assert_eq!(
            map_key(KeyCode::Enter, SearchMode::Editing),
            Some(LogsAction::ExitSearch)
        );
        assert_eq!(
            map_key(KeyCode::Esc, SearchMode::Editing),
            Some(LogsAction::ExitSearch)
        );
    }

    #[test]
    fn test_map_key_normal_mode() {
        assert_eq!(
            map_key(KeyCode::Char('/'), SearchMode::Normal),
            Some(LogsAction::EnterSearch)
        );
        assert_eq!(
            map_key(KeyCode::Char('c'), SearchMode::Normal),
            Some(LogsAction::ClearSearch)
        );
        assert_eq!(
            map_key(KeyCode::Tab, SearchMode::Normal),
            Some(LogsAction::NextDevice)
        );
        assert_eq!(
            map_key(KeyCode::Char('1'), SearchMode::Normal),
            Some(LogsAction::SelectDeviceIndex(0))
        );
        assert_eq!(
            map_key(KeyCode::Char('a'), SearchMode::Normal),
            Some(LogsAction::ToggleAutoScroll)
        );
        assert_eq!(
            map_key(KeyCode::Char('k'), SearchMode::Normal),
            Some(LogsAction::ToggleStream)
        );
    }
}
