use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Wrap},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // system info bar
            Constraint::Min(0),   // main content
        ])
        .split(area);

    render_system_bar(f, app, chunks[0]);

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),       // CPU sparkline
            Constraint::Length(5),       // RAM / SWAP gauges
            Constraint::Length(5),       // GPU summary
            Constraint::Length(5),       // Network sparkline
            Constraint::Length(5),       // Disk summary
            Constraint::Min(0),         // Sensors + Battery
        ])
        .split(chunks[1]);

    render_cpu_mini(f, app, main[0]);
    render_mem_gauges(f, app, main[1]);
    render_gpu_mini(f, app, main[2]);
    render_net_mini(f, app, main[3]);
    render_disk_mini(f, app, main[4]);
    render_sensors_battery(f, app, main[5]);
}

fn render_system_bar(f: &mut Frame, #[allow(unused_variables)] app: &App, area: Rect) {
    let hostname = System::host_name().unwrap_or_else(|| "unknown".into());
    let os_name = System::long_os_version().unwrap_or_else(|| "unknown".into());
    let kernel = System::kernel_version().unwrap_or_else(|| "unknown".into());
    let uptime_secs = System::uptime();

    let hours = uptime_secs / 3600;
    let mins = (uptime_secs % 3600) / 60;

    let load = System::load_average();
    
    let is_root = crate::platform::is_running_as_admin();
    
    let mut spans = vec![
        Span::styled(format!(" {} ", hostname), Style::default().fg(theme::accent()).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(theme::fg_muted())),
        Span::styled(format!("{} ", os_name), Style::default().fg(theme::title_fg())),
        Span::styled("│ ", Style::default().fg(theme::fg_muted())),
        Span::styled("Kernel: ", Style::default().fg(theme::fg_muted())),
        Span::styled(format!("{} ", kernel), Style::default().fg(theme::title_fg())),
        Span::styled("│ ", Style::default().fg(theme::fg_muted())),
        Span::styled("Uptime: ", Style::default().fg(theme::fg_muted())),
        Span::styled(format!("{}h {:02}m ", hours, mins), Style::default().fg(theme::accent2())),
        Span::styled("│ ", Style::default().fg(theme::fg_muted())),
        cfg_select! {
            target_os = "windows" => {
                Span::styled("CPU: ", Style::default().fg(theme::fg_muted()))
            },
            _ => {
                Span::styled("Load: ", Style::default().fg(theme::fg_muted()))
            }
        },
        cfg_select! {
            target_os = "windows" => {
                Span::styled(
                    format!("{:.1}%", app.sys.global_cpu_usage()),
                    Style::default().fg(theme::usage_color(app.sys.global_cpu_usage() as f64)),
                )
            },
            _ => {
                Span::styled(
                    format!("{:.2} {:.2} {:.2}", load.one, load.five, load.fifteen),
                    Style::default().fg(theme::usage_color(load.five * 10.0)),
                )
            }
        },
    ];

    if is_root {
        spans.push(Span::styled(" │ ", Style::default().fg(theme::fg_muted())));
        spans.push(Span::styled("[ROOT]", theme::style_root_badge()));
    }

    let p = Paragraph::new(Line::from(spans))
        .style(theme::style_header())
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::style_border()),
        );
    f.render_widget(p, area);
}

fn render_cpu_mini(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Sparkline
    let data: Vec<u64> = app.cpu_history.iter().copied().collect();
    let current = data.last().copied().unwrap_or(0);
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" CPU {current}% "))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&data)
        .max(100)
        .style(Style::default().fg(theme::usage_color(current as f64)));
    f.render_widget(sparkline, cols[0]);

    // Overall gauge
    let ratio = (current as f64 / 100.0).clamp(0.0, 1.0);
    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Overall ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio(ratio)
        .label(format!("{current}%"))
        .gauge_style(theme::style_gauge(current as f64));
    f.render_widget(gauge, cols[1]);
}

fn render_mem_gauges(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let inner_width = (cols[0].width as usize).saturating_sub(2);

    // RAM
    let total = app.mem_segments.total;
    let active = app.mem_segments.active;
    let wired = app.mem_segments.wired;
    let cache = app.mem_segments.cache;
    let free = total.saturating_sub(active).saturating_sub(wired).saturating_sub(cache);

    let active_chars = if total > 0 {
        ((active as f64 / total as f64) * inner_width as f64).round() as usize
    } else {
        0
    };
    let wired_chars = if total > 0 {
        ((wired as f64 / total as f64) * inner_width as f64).round() as usize
    } else {
        0
    };
    let cache_chars = if total > 0 {
        ((cache as f64 / total as f64) * inner_width as f64).round() as usize
    } else {
        0
    };
    let total_used_chars = active_chars + wired_chars + cache_chars;
    let free_chars = inner_width.saturating_sub(total_used_chars);

    let mut ram_spans = Vec::new();
    if active_chars > 0 {
        ram_spans.push(ratatui::text::Span::styled(
            "█".repeat(active_chars),
            Style::default().fg(theme::usage_color(0.0)),
        ));
    }
    if wired_chars > 0 {
        ram_spans.push(ratatui::text::Span::styled(
            "█".repeat(wired_chars),
            Style::default().fg(theme::accent()),
        ));
    }
    if cache_chars > 0 {
        ram_spans.push(ratatui::text::Span::styled(
            "█".repeat(cache_chars),
            Style::default().fg(theme::usage_color(50.0)),
        ));
    }
    if free_chars > 0 {
        ram_spans.push(ratatui::text::Span::styled(
            "█".repeat(free_chars),
            Style::default().fg(theme::fg_muted()),
        ));
    }

    let used_gib = (active + wired) as f64 / 1_073_741_824.0;
    let total_gib = total as f64 / 1_073_741_824.0;
    let cache_gib = cache as f64 / 1_073_741_824.0;
    let free_gib = free as f64 / 1_073_741_824.0;
    let pct = if total > 0 { (active + wired) as f64 / total as f64 * 100.0 } else { 0.0 };

    let ram_label_line = ratatui::text::Line::from(vec![
        ratatui::text::Span::raw(format!(
            " {:.1} / {:.1} GiB ({:.0}%) │ Cache: {:.1}G │ Free: {:.1}G",
            used_gib,
            total_gib,
            pct.clamp(0.0, 100.0),
            cache_gib,
            free_gib,
        ))
    ]);

    let ram_paragraph = ratatui::widgets::Paragraph::new(vec![
        ratatui::text::Line::from(ram_spans),
        ram_label_line,
    ])
    .block(
        Block::default()
            .title(" RAM ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(ram_paragraph, cols[0]);

    // SWAP
    let total_sw = app.sys.total_swap();
    let used_sw = app.sys.used_swap();
    let free_sw = total_sw.saturating_sub(used_sw);
    let pct_sw = if total_sw > 0 { used_sw as f64 / total_sw as f64 * 100.0 } else { 0.0 };

    let used_sw_chars = if total_sw > 0 {
        ((used_sw as f64 / total_sw as f64) * inner_width as f64).round() as usize
    } else {
        0
    };
    let free_sw_chars = inner_width.saturating_sub(used_sw_chars);

    let mut swap_spans = Vec::new();
    if used_sw_chars > 0 {
        swap_spans.push(ratatui::text::Span::styled(
            "█".repeat(used_sw_chars),
            Style::default().fg(theme::accent2()),
        ));
    }
    if free_sw_chars > 0 {
        swap_spans.push(ratatui::text::Span::styled(
            "█".repeat(free_sw_chars),
            Style::default().fg(theme::fg_muted()),
        ));
    }

    let swap_label_line = ratatui::text::Line::from(vec![
        ratatui::text::Span::raw(format!(
            " {:.1} / {:.1} GiB ({:.0}%) │ Free: {:.1}G",
            used_sw as f64 / 1_073_741_824.0,
            total_sw as f64 / 1_073_741_824.0,
            pct_sw.clamp(0.0, 100.0),
            free_sw as f64 / 1_073_741_824.0,
        ))
    ]);

    let swap_paragraph = ratatui::widgets::Paragraph::new(vec![
        ratatui::text::Line::from(swap_spans),
        swap_label_line,
    ])
    .block(
        Block::default()
            .title(" SWAP ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(swap_paragraph, cols[1]);
}

fn render_gpu_mini(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let gpu_data: Vec<u64> = app.gpu_history.iter().copied().collect();
    let current = gpu_data.last().copied().unwrap_or(0);
    
    let model = if !app.gpu_model.is_empty() {
        format!(" GPU ({}) ", app.gpu_model)
    } else {
        " GPU ".into()
    };

    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" {model} {current}% "))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&gpu_data)
        .max(100)
        .style(Style::default().fg(theme::usage_color(current as f64)));
    f.render_widget(sparkline, cols[0]);

    let mut spans = vec![
        Span::styled(" Usage: ", Style::default().fg(theme::fg_muted())),
        Span::styled(format!("{current}%"), Style::default().fg(theme::usage_color(current as f64))),
    ];
    if let Some(pwr) = app.gpu_power_mw
        && pwr > 0.0 {
            spans.push(Span::styled(" │ Power: ", Style::default().fg(theme::fg_muted())));
            spans.push(Span::styled(format!("{:.1}W", pwr / 1000.0), Style::default().fg(theme::accent2())));
    }
    
    let p = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .title(" Info ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, cols[1]);
}

fn render_net_mini(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let rx_data: Vec<u64> = app.net_rx_history.iter().copied().collect();
    let rx_current = rx_data.last().copied().unwrap_or(0);
    let rx_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" ↓ RX {} ", format_bytes_rate(rx_current)))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&rx_data)
        .style(Style::default().fg(theme::accent()));
    f.render_widget(rx_sparkline, cols[0]);

    let tx_data: Vec<u64> = app.net_tx_history.iter().copied().collect();
    let tx_current = tx_data.last().copied().unwrap_or(0);
    let tx_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" ↑ TX {} ", format_bytes_rate(tx_current)))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&tx_data)
        .style(Style::default().fg(theme::accent2()));
    f.render_widget(tx_sparkline, cols[1]);
}

fn render_disk_mini(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();
    for disk in app.disks.iter() {
        let total = disk.total_space();
        let avail = disk.available_space();
        let used = total.saturating_sub(avail);
        let pct = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", disk.mount_point().to_string_lossy()), Style::default().fg(theme::accent())),
            Span::styled(format!("({}) ", disk.file_system().to_string_lossy()), Style::default().fg(theme::fg_muted())),
            Span::raw(format!("{:.1}/{:.1} GiB ", used as f64 / 1_073_741_824.0, total as f64 / 1_073_741_824.0)),
            Span::styled(format!("({:.0}%)", pct), Style::default().fg(theme::usage_color(pct))),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(" No disks detected", Style::default().fg(theme::fg_muted()))
        ]));
    }
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Disks ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_sensors_battery(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Temperature sensors
    let mut sensor_lines = Vec::new();
    for comp in app.components.iter() {
        let temp = comp.temperature().unwrap_or(0.0) as f64;
        if temp <= 0.0 {
            continue;
        }
        let label = comp.label();
        // Scale 35°C (cool / 0% gradient) to 80°C (hot / 100% gradient)
        let pct = ((temp - 35.0) / (80.0 - 35.0) * 100.0).clamp(0.0, 100.0);
        sensor_lines.push(Line::from(vec![
            Span::styled(format!(" {}: ", label), Style::default().fg(theme::fg_muted())),
            Span::styled(format!("{temp:.1}°C"), Style::default().fg(theme::usage_color(pct))),
        ]));
    }
    if sensor_lines.is_empty() {
        sensor_lines.push(Line::from(vec![
            Span::styled(" No temperature sensors detected", Style::default().fg(theme::fg_muted()))
        ]));
    }
    let p = Paragraph::new(sensor_lines)
        .block(
            Block::default()
                .title(" Temperatures ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, cols[0]);

    // Battery
    let bat_text = if let Some(ref bat) = app.battery_info {
        let time_str = bat
            .time_remaining
            .map(|d: std::time::Duration| {
                let h = d.as_secs() / 3600;
                let m = (d.as_secs() % 3600) / 60;
                format!("{h}h {m:02}m remaining")
            })
            .unwrap_or_default();
        format!(
            " {:.0}% │ {} │ {}",
            bat.percent, bat.state, time_str
        )
    } else {
        " No battery / AC Power".into()
    };

    let bat_pct = app.battery_info.as_ref().map(|b| b.percent).unwrap_or(100.0);
    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Battery ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio((bat_pct / 100.0).clamp(0.0, 1.0))
        .label(bat_text)
        .gauge_style(theme::style_gauge(100.0 - bat_pct)); // inverted: low battery = red
    f.render_widget(gauge, cols[1]);
}

use sysinfo::System;

pub fn format_bytes_rate(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB/s", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB/s", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.2} KB/s", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B/s")
    }
}
