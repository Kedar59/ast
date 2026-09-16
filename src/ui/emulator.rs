use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::AppState;
use crate::model::{PaneFocus, TargetType};

pub fn render_emulator_tab(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),     // Dual lists (AVDs vs Running targets)
            Constraint::Length(4),  // Selected Item Details
            Constraint::Length(3),  // Action bar / Hotkeys
            Constraint::Length(8),  // Activity Log Drawer
        ])
        .split(area);

    let lists_area = main_layout[0];
    let details_area = main_layout[1];
    let hotkeys_area = main_layout[2];
    let logs_area = main_layout[3];

    // Split dual list into Left (AVDs) and Right (Running Targets)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(lists_area);

    render_avds_pane(frame, columns[0], app_state);
    render_devices_pane(frame, columns[1], app_state);
    render_details_pane(frame, details_area, app_state);
    render_hotkeys_pane(frame, hotkeys_area, app_state);
    render_logs_pane(frame, logs_area, app_state);
}

fn render_avds_pane(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let is_focused = app_state.pane_focus == PaneFocus::InstalledAvds;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(
        " Installed Virtual Devices (AVDs) [{}] ",
        app_state.installed_avds.len()
    );

    let items: Vec<ListItem> = if app_state.installed_avds.is_empty() {
        vec![ListItem::new("  (No AVDs found via 'emulator -list-avds')").style(Style::default().fg(Color::DarkGray))]
    } else {
        app_state
            .installed_avds
            .iter()
            .enumerate()
            .map(|(idx, avd)| {
                let is_selected = is_focused && idx == app_state.avd_selected_index;
                let status = if avd.is_running {
                    Span::styled("[Running]", Style::default().fg(Color::Green))
                } else {
                    Span::styled("[Available]", Style::default().fg(Color::Blue))
                };

                let line = Line::from(vec![
                    Span::raw(if is_selected { " > " } else { "   " }),
                    Span::styled(&avd.name, if is_selected { Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow) } else { Style::default() }),
                    Span::raw("  "),
                    status,
                ]);

                let mut item = ListItem::new(line);
                if is_selected {
                    item = item.style(Style::default().bg(Color::Rgb(30, 40, 60)));
                }
                item
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    frame.render_widget(list, area);
}

fn render_devices_pane(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let is_focused = app_state.pane_focus == PaneFocus::RunningTargets;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(
        " Running Targets (Emulators & USB) [{}] ",
        app_state.running_devices.len()
    );

    let items: Vec<ListItem> = if app_state.running_devices.is_empty() {
        vec![ListItem::new("  (No active targets found via 'adb devices -l')").style(Style::default().fg(Color::DarkGray))]
    } else {
        app_state
            .running_devices
            .iter()
            .enumerate()
            .map(|(idx, dev)| {
                let is_selected = is_focused && idx == app_state.device_selected_index;
                let type_span = match dev.target_type {
                    TargetType::Emulator => Span::styled(" [Emulator] ", Style::default().fg(Color::Magenta)),
                    TargetType::UsbPhone => Span::styled(" [USB Phone] ", Style::default().fg(Color::Green)),
                };

                let boot_span = if dev.boot_completed {
                    Span::styled("● Booted", Style::default().fg(Color::Green))
                } else {
                    Span::styled("○ Booting...", Style::default().fg(Color::Yellow))
                };

                let name = dev.model.as_deref().unwrap_or(&dev.serial);

                let line = Line::from(vec![
                    Span::raw(if is_selected { " > " } else { "   " }),
                    Span::styled(name, if is_selected { Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow) } else { Style::default() }),
                    Span::styled(format!(" ({})", dev.serial), Style::default().fg(Color::DarkGray)),
                    type_span,
                    boot_span,
                ]);

                let mut item = ListItem::new(line);
                if is_selected {
                    item = item.style(Style::default().bg(Color::Rgb(30, 40, 60)));
                }
                item
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    frame.render_widget(list, area);
}

fn render_details_pane(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray))
        .title(" Selected Target Details ");

    let content = match app_state.pane_focus {
        PaneFocus::InstalledAvds => {
            if let Some(avd) = app_state.installed_avds.get(app_state.avd_selected_index) {
                Line::from(vec![
                    Span::styled("AVD Name: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(&avd.name, Style::default().fg(Color::Cyan)),
                    Span::raw(" | "),
                    Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(if avd.is_running { "Running" } else { "Stopped" }, Style::default().fg(if avd.is_running { Color::Green } else { Color::Yellow })),
                    Span::raw(" | "),
                    Span::styled("Execution: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("Headless with Host GPU (-no-window -gpu host)"),
                ])
            } else {
                Line::from(Span::styled("No AVD selected", Style::default().fg(Color::DarkGray)))
            }
        }
        PaneFocus::RunningTargets => {
            if let Some(dev) = app_state.running_devices.get(app_state.device_selected_index) {
                let note = match dev.target_type {
                    TargetType::Emulator => "Virtual device: press 's' for scrcpy window (mouse & keyboard control)",
                    TargetType::UsbPhone => "Physical device: operate directly by hand (no scrcpy needed)",
                };
                Line::from(vec![
                    Span::styled("Serial: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(&dev.serial, Style::default().fg(Color::Cyan)),
                    Span::raw(" | "),
                    Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{}", dev.target_type), Style::default().fg(Color::Yellow)),
                    Span::raw(" | "),
                    Span::styled("State: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&dev.state),
                    Span::raw(" | "),
                    Span::styled("Usage: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(note, Style::default().fg(Color::LightGreen)),
                ])
            } else {
                Line::from(Span::styled("No running device selected", Style::default().fg(Color::DarkGray)))
            }
        }
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_hotkeys_pane(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray))
        .title(" Controls & Shortcuts ");

    let spans = match app_state.pane_focus {
        PaneFocus::InstalledAvds => vec![
            Span::styled("[Enter/r] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
            Span::raw("Start Emulator (Headless)  "),
            Span::styled("[Tab/←/→] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
            Span::raw("Switch to Running Targets  "),
            Span::styled("[R] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
            Span::raw("Refresh  "),
            Span::styled("[↑/↓] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
            Span::raw("Navigate  "),
        ],
        PaneFocus::RunningTargets => {
            let is_emulator = app_state
                .running_devices
                .get(app_state.device_selected_index)
                .is_some_and(|d| d.target_type == TargetType::Emulator);

            let mut keys = vec![
                Span::styled("[b] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Green)),
                Span::raw("Build & Deploy Debug APK  "),
            ];

            if is_emulator {
                keys.extend(vec![
                    Span::styled("[s] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                    Span::raw("Launch Scrcpy (Display & Controls)  "),
                    Span::styled("[k] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Red)),
                    Span::raw("Stop Emulator  "),
                ]);
            } else {
                keys.extend(vec![
                    Span::styled("[Physical Phone: Use by hand] ", Style::default().fg(Color::DarkGray)),
                ]);
            }

            keys.extend(vec![
                Span::styled("[Tab/←/→] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                Span::raw("Switch to AVDs  "),
                Span::styled("[R] ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                Span::raw("Refresh  "),
            ]);

            keys
        }
    };

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(paragraph, area);
}

fn render_logs_pane(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let title = if let Some(ref action) = app_state.current_action {
        format!(" Activity & Command Log — [Running: {action}] ")
    } else {
        " Activity & Command Log ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if app_state.is_busy {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .title(title);

    let log_lines: Vec<Line> = if app_state.status_logs.is_empty() {
        vec![Line::from(Span::styled("No activity yet. Press [r] to launch AVD, [b] to build & deploy.", Style::default().fg(Color::DarkGray)))]
    } else {
        let max_lines = area.height.saturating_sub(2) as usize;
        let start_idx = app_state.status_logs.len().saturating_sub(max_lines);
        app_state
            .status_logs
            .iter()
            .skip(start_idx)
            .map(|l| {
                if l.contains("succeeded") || l.contains("successfully") {
                    Line::from(Span::styled(l, Style::default().fg(Color::Green)))
                } else if l.contains("error") || l.contains("failed") || l.contains("Error") {
                    Line::from(Span::styled(l, Style::default().fg(Color::Red)))
                } else if l.contains("[gradle]") {
                    Line::from(Span::styled(l, Style::default().fg(Color::LightBlue)))
                } else {
                    Line::from(Span::raw(l))
                }
            })
            .collect()
    };

    let paragraph = Paragraph::new(log_lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
