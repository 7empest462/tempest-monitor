use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Gauge, Sparkline},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // RAM sparkline
            Constraint::Length(5), // SWAP sparkline
            Constraint::Length(4), // RAM gauge
            Constraint::Length(4), // SWAP gauge
            Constraint::Min(0),   // Details
        ])
        .split(area);

    // RAM sparkline
    let ram_data: Vec<u64> = app.ram_history.iter().copied().collect();
    let ram_current = ram_data.last().copied().unwrap_or(0);
    let ram_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" RAM History ({ram_current}%) "))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&ram_data)
        .max(100)
        .style(Style::default().fg(theme::usage_color(ram_current as f64)));
    f.render_widget(ram_sparkline, chunks[0]);

    // SWAP sparkline
    let swap_data: Vec<u64> = app.swap_history.iter().copied().collect();
    let swap_current = swap_data.last().copied().unwrap_or(0);
    let swap_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" SWAP History ({swap_current}%) "))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&swap_data)
        .max(100)
        .style(Style::default().fg(theme::usage_color(swap_current as f64)));
    f.render_widget(swap_sparkline, chunks[1]);

    // RAM gauge
    let total = app.sys.total_memory();
    let used = app.sys.used_memory();
    let avail = app.sys.available_memory();
    let pct = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };
    let ram_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" RAM ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio((pct / 100.0).clamp(0.0, 1.0))
        .label(format!(
            "{:.2} GiB used / {:.2} GiB total │ {:.2} GiB available ({:.0}%)",
            used as f64 / 1_073_741_824.0,
            total as f64 / 1_073_741_824.0,
            avail as f64 / 1_073_741_824.0,
            pct,
        ))
        .gauge_style(theme::style_gauge(pct));
    f.render_widget(ram_gauge, chunks[2]);

    // SWAP gauge
    let total_sw = app.sys.total_swap();
    let used_sw = app.sys.used_swap();
    let free_sw = total_sw.saturating_sub(used_sw);
    let pct_sw = if total_sw > 0 { used_sw as f64 / total_sw as f64 * 100.0 } else { 0.0 };
    let swap_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" SWAP ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio((pct_sw / 100.0).clamp(0.0, 1.0))
        .label(format!(
            "{:.2} GiB used / {:.2} GiB total │ {:.2} GiB free ({:.0}%)",
            used_sw as f64 / 1_073_741_824.0,
            total_sw as f64 / 1_073_741_824.0,
            free_sw as f64 / 1_073_741_824.0,
            pct_sw,
        ))
        .gauge_style(theme::style_gauge(pct_sw));
    f.render_widget(swap_gauge, chunks[3]);

    // Memory details
    let detail_text = format!(
        " Total: {:.2} GiB │ Used: {:.2} GiB │ Available: {:.2} GiB\n Swap Total: {:.2} GiB │ Swap Used: {:.2} GiB │ Swap Free: {:.2} GiB",
        total as f64 / 1_073_741_824.0,
        used as f64 / 1_073_741_824.0,
        avail as f64 / 1_073_741_824.0,
        total_sw as f64 / 1_073_741_824.0,
        used_sw as f64 / 1_073_741_824.0,
        free_sw as f64 / 1_073_741_824.0,
    );
    let detail = ratatui::widgets::Paragraph::new(detail_text)
        .block(
            Block::default()
                .title(" Details ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .style(theme::style_muted());
    f.render_widget(detail, chunks[4]);
}
