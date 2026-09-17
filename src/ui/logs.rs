use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::{symbols, Frame};

use crate::app::AppState;
use crate::model::SearchMode;

pub fn render_logs_tab(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Device sub-tabs
        Constraint::Length(3), // Search & Filter bar
        Constraint::Fill(1),   // Main log viewport
        Constraint::Length(2), // Controls & Hotkeys footer
    ])
    .split(area);

    render_device_tabs(frame, chunks[0], app_state);
    render_search_bar(frame, chunks[1], app_state);
    render_log_viewport(frame, chunks[2], app_state);
    render_footer(frame, chunks[3], app_state);
}

fn render_device_tabs(frame: &mut Frame, area: Rect, app_state: &AppState) {
    if app_state.running_devices.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Device Log Streams (0 Active) ",
                Style::default().fg(Color::Yellow),
            ));
        let p = Paragraph::new("No active devices connected. Start an AVD (F1) or connect via USB.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(p, area);
        return;
    }

    let tab_titles: Vec<Line> = app_state
        .running_devices
        .iter()
        .enumerate()
        .map(|(idx, dev)| {
            let session = app_state.log_state.sessions.get(&dev.serial);
            let is_streaming = session.is_some_and(|s| s.is_streaming);
            let stream_dot = if is_streaming {
                Span::styled("●", Style::default().fg(Color::Green))
            } else {
                Span::styled("○", Style::default().fg(Color::DarkGray))
            };

            let name = dev.model.as_deref().unwrap_or(&dev.serial);
            Line::from(vec![
                Span::styled(format!("[{}] ", idx + 1), Style::default().fg(Color::Cyan).bold()),
                Span::from(format!("{name} (") ),
                Span::styled(&dev.serial, Style::default().fg(Color::Yellow)),
                Span::from(") "),
                stream_dot,
            ])
        })
        .collect();

    let active_idx = app_state.active_log_device_index();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Span::styled(
                    " Device Log Streams ",
                    Style::default().fg(Color::White).bold(),
                )),
        )
        .highlight_style(Style::default().fg(Color::Cyan).bold().underlined())
        .select(active_idx)
        .divider(symbols::DOT)
        .padding(" ", " ");

    frame.render_widget(tabs, area);
}

fn render_search_bar(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let is_editing = app_state.log_state.search_mode == SearchMode::Editing;
    let session = app_state.active_log_session();

    let (border_style, title_span) = if is_editing {
        (
            Style::default().fg(Color::Yellow).bold(),
            Span::styled(
                " Search & Filter [INPUT MODE: Type query | Enter: Apply | Esc: Cancel] ",
                Style::default().fg(Color::Yellow).bold(),
            ),
        )
    } else {
        let has_filter = session.is_some_and(|s| !s.search_query.is_empty());
        let color = if has_filter { Color::Green } else { Color::DarkGray };
        (
            Style::default().fg(color),
            Span::styled(
                " Search & Filter [/ to Edit | c to Clear] ",
                Style::default().fg(Color::White),
            ),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title_span);

    let content = if let Some(sess) = session {
        let total_lines = sess.lines.len();
        let filtered_count = sess.filtered_lines().len();

        if is_editing {
            Line::from(vec![
                Span::styled("Query: ", Style::default().fg(Color::Cyan).bold()),
                Span::styled(&sess.search_query, Style::default().fg(Color::Yellow).bold()),
                Span::styled("█", Style::default().fg(Color::Yellow)),
            ])
        } else if sess.search_query.is_empty() {
            Line::from(Span::styled(
                "No active filter (showing all logs). Press [/] to search or filter.",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Line::from(vec![
                Span::styled("Active Filter: \"", Style::default().fg(Color::White)),
                Span::styled(&sess.search_query, Style::default().fg(Color::Yellow).bold()),
                Span::styled("\" ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("(Matched {filtered_count} / {total_lines} lines)"),
                    Style::default().fg(Color::Green).bold(),
                ),
            ])
        }
    } else {
        Line::from(Span::styled(
            "Select a running device to enable log search.",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_log_viewport(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let session = app_state.active_log_session();

    if session.is_none() || app_state.running_devices.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Logcat Stream ",
                Style::default().fg(Color::White).bold(),
            ));

        let empty_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No device selected or no running devices available.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::from("Go to "),
                Span::styled("[F1] Emulator & Devices", Style::default().fg(Color::Cyan).bold()),
                Span::from(" to launch an AVD or connect a physical phone via USB."),
            ]),
            Line::from(vec![
                Span::from("Logs will automatically stream here when an APK is deployed via "),
                Span::styled("[b]", Style::default().fg(Color::Green).bold()),
                Span::from("."),
            ]),
        ];
        let p = Paragraph::new(empty_msg)
            .alignment(Alignment::Center)
            .block(block);
        frame.render_widget(p, area);
        return;
    }

    let sess = session.unwrap();
    let filtered_lines = sess.filtered_lines();
    let total_lines = filtered_lines.len();

    let visible_height = area.height.saturating_sub(2) as usize;

    let (start_idx, end_idx) = if total_lines == 0 {
        (0, 0)
    } else {
        let end = sess.scroll_offset.clamp(1, total_lines);
        let start = end.saturating_sub(visible_height);
        (start, end)
    };

    let stream_status = if sess.is_streaming { "STREAMING" } else { "PAUSED" };
    let stream_color = if sess.is_streaming { Color::Green } else { Color::Yellow };
    let auto_scroll_status = if sess.auto_scroll { "ON" } else { "OFF" };

    let file_display = sess.log_file_path.display().to_string();

    let title_spans = vec![
        Span::styled(format!(" Logcat: {} ", sess.serial), Style::default().fg(Color::Cyan).bold()),
        Span::from(format!("| View: {end_idx}/{total_lines} | Stream: ")),
        Span::styled(format!("[{stream_status}] "), Style::default().fg(stream_color).bold()),
        Span::from("Auto-Scroll: "),
        Span::styled(
            format!("[{auto_scroll_status}] "),
            Style::default().fg(if sess.auto_scroll { Color::Green } else { Color::DarkGray }).bold(),
        ),
        Span::styled(format!("| Saved: {file_display} "), Style::default().fg(Color::DarkGray)),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Line::from(title_spans));

    if total_lines == 0 {
        let empty_lines = if sess.lines.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No logcat entries received yet for this device.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::from("Press "),
                    Span::styled("[k]", Style::default().fg(Color::Cyan).bold()),
                    Span::from(" to start/resume logcat streaming, or deploy an APK with "),
                    Span::styled("[b]", Style::default().fg(Color::Green).bold()),
                    Span::from(" in [F1]."),
                ]),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("No log lines matched the search filter \"{}\".", sess.search_query),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from("Press [c] to clear filter or [/] to edit search query."),
            ]
        };

        let p = Paragraph::new(empty_lines).block(block).alignment(Alignment::Center);
        frame.render_widget(p, area);
        return;
    }

    let slice = &filtered_lines[start_idx..end_idx];
    let formatted_lines: Vec<Line> = slice.iter().map(|line| highlight_logcat_line(line)).collect();

    let paragraph = Paragraph::new(formatted_lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let line = if app_state.log_state.search_mode == SearchMode::Editing {
        Line::from(vec![
            Span::styled("[Type] ", Style::default().fg(Color::Yellow).bold()),
            Span::from("Enter text  "),
            Span::styled("[Backspace] ", Style::default().fg(Color::Yellow).bold()),
            Span::from("Delete  "),
            Span::styled("[Enter] ", Style::default().fg(Color::Green).bold()),
            Span::from("Apply Filter  "),
            Span::styled("[Esc] ", Style::default().fg(Color::Red).bold()),
            Span::from("Cancel  "),
        ])
    } else {
        Line::from(vec![
            Span::styled("[Tab/←/→] ", Style::default().fg(Color::Cyan).bold()),
            Span::from("Switch Device  "),
            Span::styled("[1-9] ", Style::default().fg(Color::Cyan).bold()),
            Span::from("Direct Tab  "),
            Span::styled("[/] ", Style::default().fg(Color::Cyan).bold()),
            Span::from("Search  "),
            Span::styled("[c] ", Style::default().fg(Color::Yellow).bold()),
            Span::from("Clear Filter  "),
            Span::styled("[a] ", Style::default().fg(Color::Magenta).bold()),
            Span::from("Auto-Scroll  "),
            Span::styled("[k] ", Style::default().fg(Color::Green).bold()),
            Span::from("Toggle Stream  "),
            Span::styled("[x] ", Style::default().fg(Color::Red).bold()),
            Span::from("Clear View  "),
            Span::styled("[↑/↓/PgUp/PgDn] ", Style::default().fg(Color::DarkGray)),
            Span::styled("Scroll", Style::default().fg(Color::DarkGray)),
        ])
    };

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

pub fn highlight_logcat_line(line: &str) -> Line<'static> {
    let line_str = line.to_string();

    // Check for standard logcat level indicators
    if line.contains(" E ") || line.contains("E/") || line.contains("Fatal") || line.contains("FATAL") {
        Line::from(Span::styled(line_str, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))
    } else if line.contains(" F ") || line.contains("F/") {
        Line::from(Span::styled(line_str, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD).underlined()))
    } else if line.contains(" W ") || line.contains("W/") || line.contains("Warn") || line.contains("WARN") {
        Line::from(Span::styled(line_str, Style::default().fg(Color::Yellow)))
    } else if line.contains(" I ") || line.contains("I/") {
        Line::from(Span::styled(line_str, Style::default().fg(Color::Green)))
    } else if line.contains(" D ") || line.contains("D/") {
        Line::from(Span::styled(line_str, Style::default().fg(Color::Cyan)))
    } else if line.contains(" V ") || line.contains("V/") {
        Line::from(Span::styled(line_str, Style::default().fg(Color::DarkGray)))
    } else {
        Line::from(Span::styled(line_str, Style::default().fg(Color::White)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_logcat_line() {
        let error_line = highlight_logcat_line("09-17 12:00:00.001 123 456 E AndroidRuntime: FATAL EXCEPTION");
        assert_eq!(error_line.spans[0].style.fg, Some(Color::Red));

        let warn_line = highlight_logcat_line("09-17 12:00:00.001 123 456 W System: Low memory warning");
        assert_eq!(warn_line.spans[0].style.fg, Some(Color::Yellow));

        let debug_line = highlight_logcat_line("09-17 12:00:00.001 123 456 D App: Initialized");
        assert_eq!(debug_line.spans[0].style.fg, Some(Color::Cyan));
    }
}
