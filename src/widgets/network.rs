use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table, Sparkline},
    Frame,
};

use crate::app::App;
use crate::theme;
use crate::widgets::overview::format_bytes_rate;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // RX overall sparkline
            Constraint::Length(5), // TX overall sparkline
            Constraint::Min(0),   // Per-interface list
        ])
        .split(area);

    // RX sparkline
    let rx_data: Vec<u64> = app.net_rx_history.iter().copied().collect();
    let rx_current = rx_data.last().copied().unwrap_or(0);
    let rx_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" Total Download speed: {} ", format_bytes_rate(rx_current)))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&rx_data)
        .style(Style::default().fg(theme::ACCENT));
    f.render_widget(rx_sparkline, chunks[0]);

    // TX sparkline
    let tx_data: Vec<u64> = app.net_tx_history.iter().copied().collect();
    let tx_current = tx_data.last().copied().unwrap_or(0);
    let tx_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" Total Upload speed: {} ", format_bytes_rate(tx_current)))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&tx_data)
        .style(Style::default().fg(theme::ACCENT2));
    f.render_widget(tx_sparkline, chunks[1]);

    // Interface list (Table)
    let header = Row::new([
        Cell::from("Interface").style(theme::style_table_header()),
        Cell::from("MAC Address").style(theme::style_table_header()),
        Cell::from("MTU").style(theme::style_table_header()),
        Cell::from("Speed").style(theme::style_table_header()),
        Cell::from("Duplex").style(theme::style_table_header()),
        Cell::from("Driver").style(theme::style_table_header()),
        Cell::from("RX Rate").style(theme::style_table_header()),
        Cell::from("TX Rate").style(theme::style_table_header()),
        Cell::from("Total RX").style(theme::style_table_header()),
        Cell::from("Total TX").style(theme::style_table_header()),
    ])
    .style(Style::default().bg(theme::HEADER_BG))
    .height(1);

    let rows: Vec<Row> = app.networks.iter().map(|(name, data)| {
        let info = app.network_info.get(name);
        let mac = info.map(|i| i.mac.clone()).unwrap_or_else(|| "-".into());
        let mtu = info.map(|i| i.mtu.to_string()).unwrap_or_else(|| "-".into());
        let speed = info.and_then(|i| i.speed).map(|s| format!("{}Mb/s", s)).unwrap_or_else(|| "-".into());
        let duplex = info.and_then(|i| i.duplex.clone()).unwrap_or_else(|| "-".into());
        let driver = info.and_then(|i| i.driver.clone()).unwrap_or_else(|| "-".into());

        Row::new(vec![
            Cell::from(name.clone()),
            Cell::from(mac),
            Cell::from(mtu),
            Cell::from(speed),
            Cell::from(duplex),
            Cell::from(driver),
            Cell::from(format_bytes_rate(data.received())),
            Cell::from(format_bytes_rate(data.transmitted())),
            Cell::from(format_total_bytes(data.total_received())),
            Cell::from(format_total_bytes(data.total_transmitted())),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(18),
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(" Network Interfaces ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(table, chunks[2]);
}

fn format_total_bytes(bytes: u64) -> String {
    if bytes >= 1_099_511_627_776 {
        format!("{:.1} TiB", bytes as f64 / 1_099_511_627_776.0)
    } else if bytes >= 1_073_741_824 {
        format!("{:.1} GiB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    }
}
