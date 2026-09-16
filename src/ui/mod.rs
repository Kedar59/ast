pub mod emulator;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Tabs};
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

    let title = Line::from(vec![
        Span::from("Android Studio TUI (ast)").bold().cyan(),
        Span::from("  [F1] Emulator  [F2] Build  [F3] Logs  |  [q/Esc] Quit"),
    ]);
    frame.render_widget(title.centered(), top);

    render_tabs(frame, tabs_area, app_state);

    match app_state.screen_type {
        ScreenType::Emu => emulator::render_emulator_tab(frame, main_content, app_state),
        ScreenType::Build => render_placeholder(frame, main_content, "Build and Gradle Screen (F2)"),
        ScreenType::Logs => render_placeholder(frame, main_content, "Logs and Logcat Screen (F3)"),
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let selected_tab = app_state.screen_type as usize;

    let tabs = Tabs::new(vec!["[F1] Emulator & Devices", "[F2] Build & Gradle", "[F3] Logs & Logcat"])
        .style(Color::DarkGray)
        .highlight_style(Style::default().cyan().bold().underlined())
        .select(selected_tab)
        .divider(symbols::DOT)
        .padding(" ", " ");
    frame.render_widget(tabs, area);
}

fn render_placeholder(frame: &mut Frame, area: Rect, text: &str) {
    let block = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::bordered());
    frame.render_widget(block, area);
}
