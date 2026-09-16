use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::AppState;
use crate::model::TaskStatus;

pub fn render_build_tab(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Status banner
        Constraint::Length(2), // Toolbar & Hotkey hints
        Constraint::Fill(1),   // Main output viewport
    ])
    .split(area);

    render_status_banner(frame, chunks[0], app_state);
    render_toolbar(frame, chunks[1], app_state);
    render_output_viewport(frame, chunks[2], app_state);
}

fn render_status_banner(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let state = &app_state.gradle_state;

    let (badge_text, badge_style, details_text) = match &state.status {
        TaskStatus::Idle => (
            " IDLE ",
            Style::default().bg(Color::DarkGray).fg(Color::White).bold(),
            "Ready for build tasks. Press [s] to Sync or [b] to Build Debug APK.".to_string(),
        ),
        TaskStatus::Running { started_at } => {
            let elapsed = started_at.elapsed().as_secs();
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            let task_name = state
                .active_task_name
                .as_deref()
                .unwrap_or("Running task");
            (
                " RUNNING ",
                Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                format!("{task_name} | Elapsed: {mins:02}:{secs:02} | Press [x] to cancel"),
            )
        }
        TaskStatus::Success { duration } => {
            let task_name = state
                .active_task_name
                .as_deref()
                .unwrap_or("Task");
            (
                " SUCCESS ",
                Style::default().bg(Color::Green).fg(Color::Black).bold(),
                format!(
                    "{task_name} finished in {:.2}s",
                    duration.as_secs_f64()
                ),
            )
        }
        TaskStatus::Failed { exit_code, duration } => {
            let task_name = state
                .active_task_name
                .as_deref()
                .unwrap_or("Task");
            let code_str = exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string());
            (
                " FAILED ",
                Style::default().bg(Color::Red).fg(Color::White).bold(),
                format!(
                    "{task_name} failed (exit code: {code_str}) in {:.2}s",
                    duration.as_secs_f64()
                ),
            )
        }
        TaskStatus::Cancelled => (
            " CANCELLED ",
            Style::default().bg(Color::LightRed).fg(Color::Black).bold(),
            "Task was cancelled by user.".to_string(),
        ),
    };

    let mut line_spans = vec![
        Span::from(" Status: "),
        Span::styled(badge_text, badge_style),
        Span::from(format!("  {details_text}")),
    ];

    if let Some(ref apk) = state.apk_path {
        line_spans.push(Span::from("  |  "));
        line_spans.push(Span::styled(format!("APK: {apk}"), Style::default().fg(Color::Green).bold()));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Gradle Lifecycle ",
            Style::default().fg(Color::Cyan).bold(),
        ));

    let paragraph = Paragraph::new(Line::from(line_spans)).block(block);
    frame.render_widget(paragraph, area);
}

fn render_toolbar(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let state = &app_state.gradle_state;
    let auto_scroll_str = if state.auto_scroll { "ON" } else { "OFF" };
    let auto_scroll_color = if state.auto_scroll { Color::Green } else { Color::DarkGray };

    let line = Line::from(vec![
        Span::styled("[s] ", Style::default().fg(Color::Cyan).bold()),
        Span::from("Sync  "),
        Span::styled("[b] ", Style::default().fg(Color::Cyan).bold()),
        Span::from("Assemble Debug  "),
        Span::styled("[c] ", Style::default().fg(Color::Cyan).bold()),
        Span::from("Clean  "),
        Span::styled("[x] ", Style::default().fg(Color::Red).bold()),
        Span::from("Cancel  "),
        Span::styled("[l] ", Style::default().fg(Color::Yellow).bold()),
        Span::from("Clear  "),
        Span::styled("[a] ", Style::default().fg(Color::Magenta).bold()),
        Span::from("Auto-Scroll: "),
        Span::styled(format!("[{auto_scroll_str}]  "), Style::default().fg(auto_scroll_color).bold()),
        Span::styled("[↑/↓ / PgUp/PgDn] ", Style::default().fg(Color::DarkGray)),
        Span::styled("Scroll Viewport", Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

fn render_output_viewport(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let state = &app_state.gradle_state;
    let total_lines = state.output_lines.len();

    let visible_height = area.height.saturating_sub(2) as usize;

    let (start_idx, end_idx) = if total_lines == 0 {
        (0, 0)
    } else {
        let end = state.scroll_offset.clamp(1, total_lines);
        let start = end.saturating_sub(visible_height);
        (start, end)
    };

    let title_text = format!(
        " Gradle Output (Lines: {end_idx}/{total_lines} | Auto-scroll: {}) ",
        if state.auto_scroll { "ON" } else { "OFF" }
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(title_text, Style::default().fg(Color::White).bold()));

    if total_lines == 0 {
        let empty_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No Gradle build output yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(vec![
                Span::from("Press "),
                Span::styled("[s]", Style::default().fg(Color::Cyan).bold()),
                Span::from(" to Sync Dependencies, "),
                Span::styled("[b]", Style::default().fg(Color::Cyan).bold()),
                Span::from(" to Assemble Debug APK, or "),
                Span::styled("[c]", Style::default().fg(Color::Cyan).bold()),
                Span::from(" to Clean."),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Note: Built APKs are generated in './app/build/outputs/apk/debug/app-debug.apk'.",
                Style::default().fg(Color::DarkGray).italic(),
            )),
        ];
        let paragraph = Paragraph::new(empty_msg).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    let slice = &state.output_lines[start_idx..end_idx];
    let formatted_lines: Vec<Line> = slice.iter().map(|line| highlight_gradle_line(line)).collect();

    let paragraph = Paragraph::new(formatted_lines).block(block);
    frame.render_widget(paragraph, area);
}

fn highlight_gradle_line(line: &str) -> Line<'static> {
    let trimmed = line.trim();

    if trimmed.starts_with("> Task :") {
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
    } else if trimmed.contains("BUILD SUCCESSFUL") || trimmed.starts_with('✔') {
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ))
    } else if trimmed.contains("BUILD FAILED") || trimmed.starts_with("FAILURE:") || trimmed.starts_with("ERROR:") || trimmed.starts_with('✘') {
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))
    } else if trimmed.starts_with("WARNING:") || trimmed.contains("warning:") {
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::Yellow),
        ))
    } else if trimmed.starts_with("$ ./gradlew") {
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ))
    } else if trimmed.starts_with(">>>") {
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::Yellow),
        ))
    } else {
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::White),
        ))
    }
}
