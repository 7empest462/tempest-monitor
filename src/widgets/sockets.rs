use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    render_summary(f, app, chunks[0]);
    render_socket_table(f, app, chunks[1]);
}

fn render_summary(f: &mut Frame, app: &App, area: Rect) {
    let established = app.sockets.iter().filter(|s| s.state == "ESTABLISHED").count();
    let listening   = app.sockets.iter().filter(|s| s.state == "LISTEN").count();
    let closing     = app.sockets.iter().filter(|s| s.state == "CLOSE_WAIT" || s.state == "TIME_WAIT").count();

    let text = format!(
        " Total: {}  │  Established: {}  │  Listening: {}  │  Closing: {}  │  [↑↓/jk/PgUp/PgDn] Scroll ",
        app.sockets.len(), established, listening, closing
    );

    let p = Paragraph::new(text)
        .style(theme::style_header())
        .block(
            Block::default()
                .title(" Active Network Connections ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );
    f.render_widget(p, area);
}

fn render_socket_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header = Row::new([
        Cell::from("Proto").style(theme::style_table_header()),
        Cell::from("Local Address").style(theme::style_table_header()),
        Cell::from("Foreign Address").style(theme::style_table_header()),
        Cell::from("State").style(theme::style_table_header()),
        Cell::from("Process").style(theme::style_table_header()),
    ])
    .style(Style::default().bg(theme::HEADER_BG))
    .height(1);

    let state_color = |state: &str| -> Color {
        match state {
            "ESTABLISHED" => Color::Rgb(166, 227, 161),
            "LISTEN"      => Color::Cyan,
            "CLOSE_WAIT"  => Color::Rgb(250, 179, 135),
            "TIME_WAIT"   => Color::Rgb(243, 139, 168),
            _             => Color::White,
        }
    };

    let rows: Vec<Row> = app.sockets.iter().map(|sock| {
        let state_sty = Style::default().fg(state_color(&sock.state));
        let proc_str = if sock.process_name.is_empty() {
            sock.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into())
        } else {
            sock.process_name.clone()
        };
        Row::new(vec![
            Cell::from(sock.proto.clone()),
            Cell::from(sock.local_addr.clone()),
            Cell::from(sock.foreign_addr.clone()),
            Cell::from(sock.state.clone()).style(state_sty),
            Cell::from(proc_str),
        ])
    }).collect();

    let total = app.sockets.len();
    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Percentage(28),
            Constraint::Percentage(28),
            Constraint::Length(13),
            Constraint::Min(12),
        ],
    )
    .header(header)
    .row_highlight_style(theme::style_selected())
    .block(
        Block::default()
            .title(format!(" Active Sockets ({}) ", total))
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );

    let mut state = TableState::default().with_selected(Some(app.socket_selected));
    f.render_stateful_widget(table, area, &mut state);
}
