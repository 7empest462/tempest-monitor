use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use crate::app::{App, ServiceInspectorMode};
use crate::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if app.services.inspector_open {
        render_inspector(f, app, area);
    } else {
        render_service_list(f, app, area);
    }
}

// ── Service List (original view) ─────────────────────────────────────────────

fn render_service_list(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    render_service_table(f, app, chunks[0]);
    render_action_bar(f, app, chunks[1]);
}

fn render_service_table(f: &mut Frame, app: &mut App, area: Rect) {
    let total = app.services.list.len();
    let header = Row::new([
        Cell::from("PID").style(theme::style_table_header()),
        Cell::from("Status").style(theme::style_table_header()),
        Cell::from("Label").style(theme::style_table_header()),
    ])
    .style(Style::default().bg(theme::header_bg()))
    .height(1);

    let rows: Vec<Row> = app.services.list.iter().map(|svc| {
        let pid_str    = svc.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into());
        let status_str = if svc.pid.is_some() { "running".to_string() } else { svc.status.to_string() };
        let status_style = if svc.pid.is_some() {
            Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::fg_muted())
        };

        Row::new(vec![
            Cell::from(pid_str),
            Cell::from(status_str).style(status_style),
            Cell::from(svc.label.clone()),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [Constraint::Length(8), Constraint::Length(10), Constraint::Min(40)],
    )
    .header(header)
    .row_highlight_style(theme::style_selected())
    .block(
        Block::default()
            .title(format!(
                " Services ({}) │ [Enter] Inspect │ [↑↓/jk] Navigate │ [s] Stop │ [r] Restart ",
                total
            ))
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );

    let mut state = TableState::default().with_selected(Some(app.services.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn render_action_bar(f: &mut Frame, app: &App, area: Rect) {
    let msg = if let Some(ref feedback) = app.services.action_pending {
        Line::from(vec![
            Span::styled(" ► ", Style::default().fg(theme::accent())),
            Span::raw(feedback.clone()),
        ])
    } else if let Some(svc) = app.services.list.get(app.services.selected) {
        Line::from(vec![
            Span::styled(" Selected: ", Style::default().fg(theme::fg_muted())),
            Span::styled(svc.label.clone(), Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::styled(
                if svc.pid.is_some() { "  [RUNNING]" } else { "  [STOPPED]" },
                Style::default().fg(if svc.pid.is_some() { Color::Rgb(166, 227, 161) } else { Color::Rgb(243, 139, 168) })
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(Span::raw(" No service selected"))
    };

    let p = Paragraph::new(msg).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(p, area);
}

// ── Service Inspector Panel ──────────────────────────────────────────────────

fn render_inspector(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Header: service info
            Constraint::Min(0),    // Body: file contents or logs
            Constraint::Length(3), // Footer: action bar
        ])
        .split(area);

    render_inspector_header(f, app, chunks[0]);
    render_inspector_body(f, app, chunks[1]);
    render_inspector_actions(f, app, chunks[2]);
}

fn render_inspector_header(f: &mut Frame, app: &App, area: Rect) {
    let svc = match app.services.list.get(app.services.selected) {
        Some(s) => s,
        None => return,
    };

    let status_color = if svc.pid.is_some() {
        Color::Rgb(166, 227, 161)
    } else {
        Color::Rgb(243, 139, 168)
    };
    let status_text = if let Some(pid) = svc.pid {
        format!("RUNNING (PID {})", pid)
    } else {
        format!("STOPPED (exit {})", svc.status)
    };

    let file_path = app.services.file_path.as_deref().unwrap_or("(not found)");
    let config_text = match &app.services.config_path {
        Some(p) => p.to_string(),
        None => "(none detected)".into(),
    };

    let sip_badge = if app.services.is_sip_protected {
        Span::styled(" [SIP PROTECTED] ", Style::default()
            .fg(Color::Rgb(249, 226, 175))
            .add_modifier(Modifier::BOLD))
    } else {
        Span::raw("")
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(" Status: ", Style::default().fg(theme::fg_muted())),
            Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            sip_badge,
        ]),
        Line::from(vec![
            Span::styled(" File:   ", Style::default().fg(theme::fg_muted())),
            Span::styled(file_path, Style::default().fg(theme::accent())),
        ]),
        Line::from(vec![
            Span::styled(" Config: ", Style::default().fg(theme::fg_muted())),
            Span::styled(config_text, Style::default().fg(
                if app.services.config_path.is_some() { theme::accent() } else { theme::fg_muted() }
            )),
        ]),
    ];

    let title = format!(" {} ", app.services.list.get(app.services.selected)
        .map(|s| s.label.as_str())
        .unwrap_or("Service"));

    let p = Paragraph::new(lines).block(
        Block::default()
            .title(title)
            .title_style(Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(p, area);
}

fn render_inspector_body(f: &mut Frame, app: &mut App, area: Rect) {
    let (title, content_lines) = match app.services.inspector_mode {
        ServiceInspectorMode::View => {
            let title = " Service File Contents ";
            let lines: Vec<Line> = if let Some(ref contents) = app.services.file_contents {
                contents.lines().enumerate().map(|(i, line)| {
                    Line::from(vec![
                        Span::styled(
                            format!(" {:4} │ ", i + 1),
                            Style::default().fg(theme::fg_muted()),
                        ),
                        Span::raw(line.to_string()),
                    ])
                }).collect()
            } else {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Service file not found on disk.",
                        Style::default().fg(Color::Rgb(249, 226, 175)),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  This may be a system-managed service without a standalone plist/unit file.",
                        Style::default().fg(theme::fg_muted()),
                    )),
                ]
            };
            (title, lines)
        }
        ServiceInspectorMode::Logs => {
            let title = " Service Logs ";
            let lines: Vec<Line> = if app.services.log_lines.is_empty() {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  No logs available.",
                        Style::default().fg(theme::fg_muted()),
                    )),
                ]
            } else {
                app.services.log_lines.iter().map(|l| {
                    if l.starts_with("──") {
                        Line::from(Span::styled(
                            format!(" {}", l),
                             Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        Line::from(Span::raw(format!(" {}", l)))
                    }
                }).collect()
            };
            (title, lines)
        }
    };

    // Clamp scroll
    let max_scroll = content_lines.len().saturating_sub(area.height as usize) as u16;
    if app.services.inspector_scroll > max_scroll {
        app.services.inspector_scroll = max_scroll;
    }

    let mode_indicator = match app.services.inspector_mode {
        ServiceInspectorMode::View => "[File View]",
        ServiceInspectorMode::Logs => "[Log View]",
    };

    let p = Paragraph::new(content_lines)
        .scroll((app.services.inspector_scroll, 0))
        .block(
            Block::default()
                .title(format!("{} {} ", title, mode_indicator))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_inspector_actions(f: &mut Frame, app: &App, area: Rect) {
    let edit_style = if app.services.is_sip_protected || app.services.file_path.is_none() {
        Style::default().fg(theme::fg_muted()) // greyed out
    } else {
        Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)
    };

    let config_style = if app.services.config_path.is_some() {
        Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::fg_muted()) // greyed out
    };

    let log_style = Style::default().fg(
        if app.services.inspector_mode == ServiceInspectorMode::Logs {
            Color::Rgb(166, 227, 161) // active highlight
        } else {
            theme::accent()
        }
    ).add_modifier(Modifier::BOLD);

    let action_style = Style::default().fg(theme::accent2()).add_modifier(Modifier::BOLD);

    let mut spans = vec![
        Span::raw(" "),
        Span::styled("[e]", edit_style),
        Span::styled(" Edit File ", if app.services.is_sip_protected {
            Style::default().fg(theme::fg_muted())
        } else {
            Style::default()
        }),
        Span::raw("│ "),
        Span::styled("[c]", config_style),
        Span::styled(" Config ", if app.services.config_path.is_none() {
            Style::default().fg(theme::fg_muted())
        } else {
            Style::default()
        }),
        Span::raw("│ "),
        Span::styled("[l]", log_style),
        Span::raw(" Logs "),
        Span::raw("│ "),
        Span::styled("[Enter]", action_style),
        Span::raw(" Start "),
        Span::styled("[s]", action_style),
        Span::raw(" Stop "),
        Span::styled("[r]", action_style),
        Span::raw(" Restart "),
        Span::raw("│ "),
        Span::styled("[Esc]", Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD)),
        Span::raw(" Back"),
    ];

    // Append feedback if any
    if let Some(ref feedback) = app.services.action_pending {
        spans.push(Span::raw(" │ "));
        spans.push(Span::styled(feedback.clone(), Style::default().fg(Color::Rgb(249, 226, 175))));
    }

    let p = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(p, area);
}

/// Called from input.rs to run service management actions.
pub fn run_service_action(app: &mut App, action: &str) {
    if let Some(svc) = app.services.list.get(app.services.selected) {
        let label = svc.label.clone();

        #[cfg(target_os = "macos")]
        let result = {
            if action == "restart" {
                let uid = crate::platform::get_current_uid();
                std::process::Command::new("launchctl")
                    .args(["kickstart", "-k", &format!("gui/{}/{}", uid, label)])
                    .status()
            } else {
                std::process::Command::new("launchctl")
                    .args([action, &label])
                    .status()
            }
        };

        #[cfg(target_os = "linux")]
        let result = {
            std::process::Command::new("systemctl")
                .args([action, &label])
                .status()
        };

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let result: Result<std::process::ExitStatus, std::io::Error> = {
            Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "unsupported platform"))
        };

        app.services.action_pending = Some(match result {
            Ok(s) if s.success() => format!("✓ {} '{}' succeeded", action, label),
            Ok(s) => format!("✗ {} '{}' exited {}", action, label, s.code().unwrap_or(-1)),
            Err(e) => format!("✗ {} '{}' failed: {}", action, label, e),
        });
    }
}
