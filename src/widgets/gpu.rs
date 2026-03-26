use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // GPU Header (Model + usage)
            Constraint::Length(5), // GPU Usage Sparkline
            Constraint::Length(5), // GPU Usage Gauge
            Constraint::Min(0),    // Power panel + details
        ])
        .split(area);

    render_gpu_header(f, app, chunks[0]);
    render_gpu_sparkline(f, app, chunks[1]);
    render_gpu_gauge(f, app, chunks[2]);
    render_power_and_details(f, app, chunks[3]);
}

fn render_gpu_header(f: &mut Frame, app: &App, area: Rect) {
    let usage_str = if app.gpu_usage >= 0.0 {
        format!("{:.1}%", app.gpu_usage)
    } else {
        "N/A (run with sudo)".to_string()
    };

    let pkg_str = app.pkg_power_mw
        .map(|mw| format!(" │ Pkg Power: {:.2} W", mw / 1000.0))
        .unwrap_or_default();

    let text = format!(" Model: {} │ GPU: {}{} ", app.gpu_model, usage_str, pkg_str);
    let p = Paragraph::new(text)
        .style(Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .title(" GPU Overview ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );
    f.render_widget(p, area);
}

fn render_gpu_sparkline(f: &mut Frame, app: &App, area: Rect) {
    let data: Vec<u64> = app.gpu_history.iter().copied().collect();
    let current = app.gpu_usage.max(0.0);

    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" GPU Utilization ({}%) ", current as u64))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .data(&data)
        .max(100)
        .style(Style::default().fg(theme::usage_color(current)));
    f.render_widget(sparkline, area);
}

fn render_gpu_gauge(f: &mut Frame, app: &App, area: Rect) {
    let current = app.gpu_usage.max(0.0);
    let ratio = (current / 100.0).clamp(0.0, 1.0);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Load ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio(ratio)
        .label(format!("{:.1}%", current))
        .gauge_style(theme::style_gauge(current));
    f.render_widget(gauge, area);
}

fn render_power_and_details(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    #[cfg(target_os = "macos")]
    {
        render_macos_power_panel(f, app, cols[0]);
        render_hw_details(f, app, cols[1]);
    }

    #[cfg(target_os = "linux")]
    {
        if !app.nvidia_gpus.is_empty() {
            render_nvidia_panel(f, app, area);
        } else {
            render_linux_gpu_stats(f, app, cols[0]);
            render_hw_details(f, app, cols[1]);
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        render_hw_details(f, app, area);
    }
}

// ── macOS: powermetrics power draw panel ─────────────────────────────────────

#[cfg(target_os = "macos")]
fn power_gauge(label: &str, mw: Option<f64>, max_w: f64) -> Line<'static> {
    match mw {
        Some(mw_val) => {
            let watts = mw_val / 1000.0;
            let bar_len = 20usize;
            let filled = ((watts / max_w) * bar_len as f64).round().clamp(0.0, bar_len as f64) as usize;
            let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_len - filled));
            let color = if watts / max_w > 0.75 { Color::Rgb(243, 139, 168) }
                        else if watts / max_w > 0.45 { Color::Rgb(249, 226, 175) }
                        else { Color::Rgb(166, 227, 161) };
            Line::from(vec![
                Span::styled(format!(" {:<18} ", label), Style::default().fg(theme::FG_MUTED)),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(format!("  {:.2} W", watts), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ])
        }
        None => Line::from(Span::styled(
            format!(" {:<18} ---", label),
            Style::default().fg(theme::FG_MUTED),
        )),
    }
}

#[cfg(target_os = "macos")]
fn render_macos_power_panel(f: &mut Frame, app: &App, area: Rect) {
    let has_data = app.gpu_power_mw.is_some() || app.cpu_power_mw.is_some() || app.pkg_power_mw.is_some();

    let lines: Vec<Line> = if has_data {
        vec![
            Line::from(""),
            power_gauge("CPU Power",     app.cpu_power_mw, 50.0),
            Line::from(""),
            power_gauge("GPU Power",     app.gpu_power_mw, 50.0),
            Line::from(""),
            power_gauge("Package Total", app.pkg_power_mw, 100.0),
            Line::from(""),
            Line::from(Span::styled(
                " (from powermetrics — live)",
                Style::default().fg(theme::FG_MUTED),
            )),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(" No power data.", Style::default().fg(theme::FG_MUTED))),
            Line::from(""),
            Line::from(Span::styled(" Run with sudo to enable", Style::default().fg(Color::Yellow))),
            Line::from(Span::styled(" powermetrics readings.", Style::default().fg(Color::Yellow))),
        ]
    };

    let p = Paragraph::new(lines).block(
        Block::default()
            .title(" Live Power Draw ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(p, area);
}

// ── Linux: NVIDIA panel (via NVML) ──────────────────────────────────────────

#[cfg(target_os = "linux")]
fn render_nvidia_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let mut items = Vec::new();
    for gpu in &app.nvidia_gpus {
        items.push(ListItem::new(format!("─── {} ───", gpu.name)));
        items.push(ListItem::new(format!(" Temp:       {}°C", gpu.temperature)));
        items.push(ListItem::new(format!(" VRAM Used:  {:.1}%", gpu.memory_used_pct)));
        items.push(ListItem::new(format!(" Fan Speed:  {}%", gpu.fan_speed_pct)));
        items.push(ListItem::new(format!(" Power:      {:.2}W", gpu.power_usage_mw as f64 / 1000.0)));
        items.push(ListItem::new(format!(" Gfx Clock:  {}MHz", gpu.graphics_clock_mhz)));
        items.push(ListItem::new(format!(" Mem Clock:  {}MHz", gpu.memory_clock_mhz)));
        items.push(ListItem::new(""));
    }

    let p = List::new(items).block(
        Block::default()
            .title(" NVIDIA GPU Details ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border())
    );
    f.render_widget(p, chunks[0]);

    render_hw_details(f, app, chunks[1]);
}

// ── Linux: AMD/Intel GPU stats panel (via sysfs + hwmon) ────────────────────

#[cfg(target_os = "linux")]
fn render_linux_gpu_stats(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = vec![Line::from("")];

    // Temperature
    if let Some(temp) = app.gpu_temp {
        let color = if temp > 80 { Color::Rgb(243, 139, 168) }
                    else if temp > 60 { Color::Rgb(249, 226, 175) }
                    else { Color::Rgb(166, 227, 161) };
        lines.push(Line::from(vec![
            Span::styled(" Temperature      ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(format!("{}°C", temp), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }

    // GPU Clock
    if let Some(clock) = app.gpu_clock_mhz {
        lines.push(Line::from(vec![
            Span::styled(" GPU Clock        ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(format!("{} MHz", clock), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }

    // VRAM Usage
    if let (Some(used), Some(total)) = (app.gpu_vram_used, app.gpu_vram_total) {
        let used_mb = used as f64 / 1_048_576.0;
        let total_mb = total as f64 / 1_048_576.0;
        let pct = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };
        let color = if pct > 80.0 { Color::Rgb(243, 139, 168) }
                    else if pct > 50.0 { Color::Rgb(249, 226, 175) }
                    else { Color::Rgb(166, 227, 161) };
        lines.push(Line::from(vec![
            Span::styled(" VRAM Used        ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(
                format!("{:.0} / {:.0} MB ({:.1}%)", used_mb, total_mb, pct),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // GPU Usage
    let usage = app.gpu_usage;
    if usage >= 0.0 {
        let color = if usage > 80.0 { Color::Rgb(243, 139, 168) }
                    else if usage > 50.0 { Color::Rgb(249, 226, 175) }
                    else { Color::Rgb(166, 227, 161) };
        lines.push(Line::from(vec![
            Span::styled(" GPU Busy         ", Style::default().fg(theme::FG_MUTED)),
            Span::styled(format!("{:.1}%", usage), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }

    // Source attribution
    lines.push(Line::from(Span::styled(
        " (from sysfs / hwmon — live)",
        Style::default().fg(theme::FG_MUTED),
    )));

    // If no data at all
    if lines.len() <= 2 {
        lines = vec![
            Line::from(""),
            Line::from(Span::styled(" No GPU telemetry available.", Style::default().fg(theme::FG_MUTED))),
            Line::from(""),
            Line::from(Span::styled(" Check /sys/class/drm/ and", Style::default().fg(Color::Yellow))),
            Line::from(Span::styled(" /sys/class/hwmon/ for data.", Style::default().fg(Color::Yellow))),
        ];
    }

    let p = Paragraph::new(lines).block(
        Block::default()
            .title(format!(" {} GPU Stats ", app.gpu_vendor))
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(p, area);
}

// ── Hardware details (shared, but content varies by platform) ────────────────

fn render_hw_details(f: &mut Frame, app: &App, area: Rect) {
    #[cfg(target_os = "macos")]
    let items: Vec<ListItem> = vec![
        ListItem::new(format!(" Model:          {}", app.gpu_model)),
        ListItem::new(" Architecture:   Apple Silicon (Unified Memory)"),
        ListItem::new(" Memory:         Shared with CPU (Unified)"),
        ListItem::new(" Encoder:        Apple Media Engine (HW)"),
        ListItem::new(" API Support:    Metal 3, CoreML, ANE"),
        ListItem::new(""),
    ];

    #[cfg(target_os = "linux")]
    let items: Vec<ListItem> = {
        let api_support = match app.gpu_vendor.as_str() {
            "AMD" => "Vulkan, OpenGL, OpenCL, VA-API",
            "Intel" => "Vulkan, OpenGL, OpenCL, VA-API",
            "NVIDIA" => "CUDA, Vulkan, OpenGL, OpenCL",
            _ => "Unknown",
        };
        let arch = match app.gpu_vendor.as_str() {
            "AMD" => "RDNA / GCN (Discrete/APU)",
            "Intel" => "Xe / Gen (Integrated)",
            "NVIDIA" => "Ada / Ampere (Discrete)",
            _ => "Unknown Architecture",
        };
        vec![
            ListItem::new(format!(" Model:          {}", app.gpu_model)),
            ListItem::new(format!(" Architecture:   {}", arch)),
            ListItem::new(format!(" Driver:         {}", app.gpu_driver)),
            ListItem::new(format!(" API Support:    {}", api_support)),
            ListItem::new(format!(" Vendor:         {}", app.gpu_vendor)),
            ListItem::new(""),
        ]
    };

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let items: Vec<ListItem> = vec![
        ListItem::new(format!(" Model:          {}", app.gpu_model)),
        ListItem::new(" Architecture:   Unknown"),
        ListItem::new(""),
    ];

    let mut final_items = items;

    if app.gpu_usage < 0.0 {
        final_items.push(ListItem::new(Span::styled(
            " [sudo required for live stats]",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        final_items.push(ListItem::new(Span::styled(
            " ✓ Live metrics active",
            Style::default().fg(Color::Rgb(166, 227, 161)),
        )));
    }

    let list = List::new(final_items).block(
        Block::default()
            .title(" Hardware Details ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(list, area);
}
