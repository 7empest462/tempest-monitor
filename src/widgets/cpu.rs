use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Load averages
            Constraint::Length(5), // Overall CPU sparkline
            Constraint::Min(0),    // Per-core bars
            Constraint::Length(6), // Temperature sensors
        ])
        .split(area);

    render_load_averages(f, app, chunks[0]);
    render_overall_sparkline(f, app, chunks[1]);
    render_core_bars(f, app, chunks[2]);
    render_temperatures(f, app, chunks[3]);
}

fn render_load_averages(f: &mut Frame, app: &App, area: Rect) {
    let (one, five, fifteen) = app.load_avg;
    let text = format!(
        " 1 min: {one:.2} │ 5 min: {five:.2} │ 15 min: {fifteen:.2} ",
    );
    let p = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Load Averages ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );
    f.render_widget(p, area);
}

fn render_overall_sparkline(f: &mut Frame, app: &App, area: Rect) {
    let data: Vec<u64> = app.cpu_history.iter().copied().collect();
    let current = data.last().copied().unwrap_or(0);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(area);

    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" CPU History (avg {current}%) "))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&data)
        .max(100)
        .style(Style::default().fg(theme::usage_color(current as f64)));
    f.render_widget(sparkline, cols[0]);

    // Overall gauge
    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Avg ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio((current as f64 / 100.0).clamp(0.0, 1.0))
        .label(format!("{current}%"))
        .gauge_style(theme::style_gauge(current as f64));
    f.render_widget(gauge, cols[1]);
}

fn render_core_bars(f: &mut Frame, app: &App, area: Rect) {
    let cpus = app.sys.cpus();
    let items: Vec<ListItem> = cpus
        .iter()
        .enumerate()
        .map(|(i, cpu)| {
            let usage = cpu.cpu_usage();
            let freq = cpu.frequency();
            let bar_width: usize = 20;
            let filled = ((usage / 100.0) * bar_width as f32) as usize;
            let empty = bar_width.saturating_sub(filled);
            let bar: String = "█".repeat(filled) + &"░".repeat(empty);

            ListItem::new(format!(
                " Core {:02} [{bar}] {:5.1}% @ {freq} MHz",
                i, usage,
            ))
            .style(Style::default().fg(theme::usage_color(usage as f64)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(" CPU Cores ({} total) ", cpus.len()))
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(list, area);
}

fn render_temperatures(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .components
        .iter()
        .filter(|c| c.temperature().map(|t| t > 0.0).unwrap_or(false))
        .map(|c| {
            let temp = c.temperature().unwrap_or(0.0);
            let max = c.max().unwrap_or(100.0);
            let label = c.label();
            // Color based on proximity to max
            let pct = if max > 0.0 { (temp / max * 100.0) as f64 } else { 0.0 };
            ListItem::new(format!(" {label}: {temp:.1}°C (max {max:.1}°C)"))
                .style(Style::default().fg(theme::usage_color(pct)))
        })
        .collect();

    let count = items.len();
    let list = List::new(items).block(
        Block::default()
            .title(format!(" Temperature Sensors ({count}) "))
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(list, area);
}
