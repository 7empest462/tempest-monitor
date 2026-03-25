use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
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
    let total = app.services.len();
    let header = Row::new([
        Cell::from("PID").style(theme::style_table_header()),
        Cell::from("Status").style(theme::style_table_header()),
        Cell::from("Label").style(theme::style_table_header()),
    ])
    .style(Style::default().bg(theme::HEADER_BG))
    .height(1);

    let rows: Vec<Row> = app.services.iter().map(|svc| {
        let pid_str    = svc.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into());
        let status_str = if svc.pid.is_some() { "running".to_string() } else { svc.status.to_string() };
        let status_style = if svc.pid.is_some() {
            Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::FG_MUTED)
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
                " Services ({}) │ [↑↓/jk] Navigate │ [Enter] Start │ [s] Stop │ [r] Restart ",
                total
            ))
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );

    let mut state = TableState::default().with_selected(Some(app.service_selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn render_action_bar(f: &mut Frame, app: &App, area: Rect) {
    let msg = if let Some(ref feedback) = app.service_action_pending {
        Line::from(vec![
            Span::styled(" ► ", Style::default().fg(theme::ACCENT)),
            Span::raw(feedback.clone()),
        ])
    } else if let Some(svc) = app.services.get(app.service_selected) {
        Line::from(vec![
            Span::styled(" Selected: ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(svc.label.clone(), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
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

/// Called from input.rs to run service management actions.
pub fn run_service_action(app: &mut App, action: &str) {
    if let Some(svc) = app.services.get(app.service_selected) {
        let label = svc.label.clone();

        #[cfg(target_os = "macos")]
        let result = {
            if action == "restart" {
                let uid = unsafe { libc::getuid() };
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

        app.service_action_pending = Some(match result {
            Ok(s) if s.success() => format!("✓ {} '{}' succeeded", action, label),
            Ok(s) => format!("✗ {} '{}' exited {}", action, label, s.code().unwrap_or(-1)),
            Err(e) => format!("✗ {} '{}' failed: {}", action, label, e),
        });
    }
}
