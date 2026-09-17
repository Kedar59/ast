pub mod build;
pub mod emulator;
pub mod logs;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Tabs;
use ratatui::{symbols, Frame};

use crate::app::AppState;
use crate::model::ScreenType;

pub fn render_ui(frame: &mut Frame, app_state: &AppState) {
    let layout = Layout::vertical([
        Constraint::Length(1), // Top header title
        Constraint::Length(1), // Tabs bar
        Constraint::Fill(1),   // Main content area
    ])
    .spacing(0);

    let [top, tabs_area, main_content] = frame.area().layout(&layout);

    let project_name = app_state
        .project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".");

    let title = Line::from(vec![
        Span::from("Android Studio TUI (ast)").bold().cyan(),
        Span::from("  📁 "),
        Span::from(project_name).yellow().bold(),
        Span::from("  |  [F1] Emulator  [F2] Build  [F3] Logs  |  [q/Esc] Quit"),
    ]);
    frame.render_widget(title.centered(), top);

    render_tabs(frame, tabs_area, app_state);

    match app_state.screen_type {
        ScreenType::Emu => emulator::render_emulator_tab(frame, main_content, app_state),
        ScreenType::Build => build::render_build_tab(frame, main_content, app_state),
        ScreenType::Logs => logs::render_logs_tab(frame, main_content, app_state),
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let selected_tab = app_state.screen_type as usize;

    let tabs = Tabs::new(vec![
        "[F1] Emulator & Devices",
        "[F2] Build & Gradle",
        "[F3] Logs & Logcat",
    ])
    .style(Color::DarkGray)
    .highlight_style(Style::default().cyan().bold().underlined())
    .select(selected_tab)
    .divider(symbols::DOT)
    .padding(" ", " ");
    frame.render_widget(tabs, area);
}
