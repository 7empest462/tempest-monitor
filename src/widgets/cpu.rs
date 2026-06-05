use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Sparkline, Table},
    Frame,
};

use crate::app::App;
use crate::power_mode::CpuPowerMode;
use crate::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(11), // Load avg + mode + sparkline
            Constraint::Min(0),     // Cores and sensors
        ])
        .split(area);

    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Load averages
            Constraint::Length(3), // Performance mode
            Constraint::Length(5), // Overall CPU sparkline
        ])
        .split(main_chunks[0]);

    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // CPU Cores
            Constraint::Min(0),         // Temperature Sensors
        ])
        .split(main_chunks[1]);

    render_load_averages(f, app, header_chunks[0]);
    render_power_mode(f, app, header_chunks[1]);
    render_overall_sparkline(f, app, header_chunks[2]);
    render_core_bars(f, app, list_chunks[0]);
    render_temperatures(f, app, list_chunks[1]);
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

fn render_power_mode(f: &mut Frame, app: &App, area: Rect) {
    let mode = app.cpu_power.mode;
    let detail = crate::power_mode::get_mode_detail();

    // Build the current mode display
    let mut spans: Vec<Span> = vec![
        Span::styled(" Current: ", Style::default().fg(theme::fg_muted())),
        Span::styled(
            format!("{} {} ", mode.icon(), mode.label()),
            Style::default()
                .fg(mode_color(mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("({}) ", detail),
            Style::default().fg(theme::fg_muted()),
        ),
        Span::raw("│ "),
    ];

    // Show available mode keys
    for (i, available_mode) in app.cpu_power.available_modes.iter().enumerate() {
        let f_key = format!("[F{}]", 7 + i);
        let is_active = *available_mode == mode;

        if is_active {
            spans.push(Span::styled(
                format!("{} {} {} ", f_key, available_mode.icon(), available_mode.label()),
                Style::default()
                    .fg(mode_color(*available_mode))
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            spans.push(Span::styled(
                format!("{} ", f_key),
                Style::default().fg(theme::fg_muted()),
            ));
            spans.push(Span::styled(
                format!("{} {} ", available_mode.icon(), available_mode.label()),
                Style::default().fg(theme::fg_muted()),
            ));
        }
    }

    // Show feedback if recent
    if let Some(ref feedback) = app.cpu_power.feedback {
        spans.push(Span::raw("│ "));
        let color = if feedback.starts_with('✓') {
            Color::Rgb(166, 227, 161)
        } else if feedback.starts_with('⚠') {
            Color::Rgb(249, 226, 175)
        } else {
            Color::Rgb(243, 139, 168)
        };
        spans.push(Span::styled(feedback.clone(), Style::default().fg(color)));
    }

    let p = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .title(" Performance Mode ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(p, area);
}

fn mode_color(mode: CpuPowerMode) -> Color {
    match mode {
        CpuPowerMode::LowPower    => Color::Rgb(148, 226, 213), // teal
        CpuPowerMode::Normal      => Color::Rgb(166, 227, 161), // green
        CpuPowerMode::Balanced    => Color::Rgb(249, 226, 175), // yellow
        CpuPowerMode::Performance => Color::Rgb(243, 139, 168), // red/peach
        CpuPowerMode::Unknown     => Color::Rgb(108, 112, 124), // muted
    }
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
    let count = cpus.len();

    // Core bar item width
    let col_width = 48;
    let usable_width = area.width.saturating_sub(2) as usize;
    let num_cols = (usable_width / col_width).max(1);

    let num_rows = count.div_ceil(num_cols);
    let mut rows = Vec::new();

    for r in 0..num_rows {
        let mut row_cells = Vec::new();
        for c in 0..num_cols {
            let idx = r + c * num_rows; // Column-major order
            if idx < count {
                let cpu = &cpus[idx];
                let usage = cpu.cpu_usage();
                let freq = cpu.frequency();
                let bar_width: usize = 20;
                let filled = ((usage / 100.0) * bar_width as f32) as usize;
                let empty = bar_width.saturating_sub(filled);
                let bar: String = "█".repeat(filled) + &"░".repeat(empty);

                let text = format!(" Core {:02} [{bar}] {:5.1}% @ {freq} MHz", idx, usage);
                row_cells.push(Cell::from(text).style(Style::default().fg(theme::usage_color(usage as f64))));
            } else {
                row_cells.push(Cell::from(""));
            }
        }
        rows.push(Row::new(row_cells));
    }

    let constraints: Vec<Constraint> = (0..num_cols)
        .map(|_| Constraint::Length(col_width as u16))
        .collect();

    let table = Table::new(rows, constraints)
        .block(
            Block::default()
                .title(format!(" CPU Cores ({count} total) "))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );

    f.render_widget(table, area);
}

fn render_temperatures(f: &mut Frame, app: &App, area: Rect) {
    let sensors: Vec<_> = app
        .components
        .iter()
        .filter(|c| c.temperature().map(|t| t > 0.0).unwrap_or(false))
        .map(|c| {
            let temp = c.temperature().unwrap_or(0.0) as f64;
            let max = c.max().unwrap_or(100.0) as f64;
            let label = c.label();
            // Scale 35°C (cool / 0% gradient) to 80°C (hot / 100% gradient)
            let pct = ((temp - 35.0) / (80.0 - 35.0) * 100.0).clamp(0.0, 100.0);
            (label.to_string(), temp, max, pct)
        })
        .collect();

    let count = sensors.len();
    if count == 0 {
        let empty = Paragraph::new(" No temperature sensors detected ")
            .block(
                Block::default()
                    .title(" Temperature Sensors (0) ")
                    .title_style(theme::style_title())
                    .borders(Borders::ALL)
                    .border_style(theme::style_border()),
            )
            .style(theme::style_muted());
        f.render_widget(empty, area);
        return;
    }

    // Determine how many columns we can display based on panel width.
    let col_width = 34;
    let usable_width = area.width.saturating_sub(2) as usize;
    let num_cols = (usable_width / col_width).max(1);

    // Group sensors into rows of size num_cols
    let mut rows = Vec::new();
    let num_rows = count.div_ceil(num_cols);

    for r in 0..num_rows {
        let mut row_cells = Vec::new();
        for c in 0..num_cols {
            let idx = r + c * num_rows; // Column-major order
            if idx < count {
                let (label, temp, max, pct) = &sensors[idx];
                let text = format!(" {label}: {temp:.1}°C (max {max:.1}°C)");
                row_cells.push(Cell::from(text).style(Style::default().fg(theme::usage_color(*pct))));
            } else {
                row_cells.push(Cell::from(""));
            }
        }
        rows.push(Row::new(row_cells));
    }

    let constraints: Vec<Constraint> = (0..num_cols)
        .map(|_| Constraint::Length(col_width as u16))
        .collect();

    let table = Table::new(rows, constraints)
        .block(
            Block::default()
                .title(format!(" Temperature Sensors ({count}) "))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );

    f.render_widget(table, area);
}
