use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, List, ListItem, Sparkline},
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

    // Interface list
    let interfaces: Vec<ListItem> = app.networks.iter().map(|(name, data): (&String, &sysinfo::NetworkData)| {
        ListItem::new(format!(
            " Interface: {:<12} │ ↓ {:10} │ ↑ {:10} │ Total RX: {:8} │ Total TX: {:8}",
            name,
            format_bytes_rate(data.received()),
            format_bytes_rate(data.transmitted()),
            format_total_bytes(data.total_received()),
            format_total_bytes(data.total_transmitted()),
        ))
    }).collect();

    let list = List::new(interfaces).block(
        Block::default()
            .title(" Network Interfaces ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(list, chunks[2]);
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
