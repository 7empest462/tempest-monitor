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
    let usage_str = {
        #[cfg(windows)]
        {
            if let Some(u) = windows_gpu::gpu_utilization() {
                format!("{:.1}%", u)
            } else if app.gpu_usage >= 0.0 {
                format!("{:.1}%", app.gpu_usage)
            } else {
                "N/A".to_string()
            }
        }
        #[cfg(not(windows))]
        {
            if app.gpu_usage >= 0.0 {
                format!("{:.1}%", app.gpu_usage)
            } else {
                "N/A (run with sudo)".to_string()
            }
        }
    };

    let freq_str = app.gpu_freq_mhz
        .map(|f| format!(" │ Freq: {:.0} MHz", f))
        .unwrap_or_default();

    let pkg_str = app.pkg_power_mw
        .map(|mw| format!(" │ Pkg Power: {:.2} W", mw / 1000.0))
        .unwrap_or_default();

    let vram_str = {
        #[cfg(windows)]
        {
            if let Some((used_mb, limit_mb, pct)) = windows_gpu::vram_usage_mb() {
                format!(" │ VRAM: {:.0}/{:.0} MB ({:.1}%)", used_mb, limit_mb, pct)
            } else {
                String::new()
            }
        }
        #[cfg(not(windows))]
        {
            String::new()
        }
    };

    let model_str = {
        #[cfg(windows)]
        {
            let names = windows_gpu::gpu_adapter_names();
            if names.is_empty() {
                if app.gpu_model.is_empty() {
                    "Unknown GPU".to_string()
                } else {
                    app.gpu_model.clone()
                }
            } else {
                names.join(", ")
            }
        }
        #[cfg(not(windows))]
        { app.gpu_model.clone() }
    };

    let text = format!(" Model: {} │ GPU: {}{}{}{} ", model_str, usage_str, freq_str, pkg_str, vram_str);
    let p = Paragraph::new(text)
        .style(Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .title(" GPU Overview ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );
    f.render_widget(p, area);
}

fn render_gpu_sparkline(f: &mut Frame, #[allow(unused_variables)] app: &App, area: Rect) {
    #[cfg(windows)]
    let (data, current) = {
        let d = windows_gpu::history_tick_and_get(60);
        let c = d.last().copied().unwrap_or(0) as f64;
        (d, c)
    };

    #[cfg(not(windows))]
    let (data, current) = {
        (app.gpu_history.iter().copied().collect::<Vec<u64>>(), app.gpu_usage.max(0.0))
    };

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
    let current = {
        #[cfg(windows)]
        {
            windows_gpu::gpu_utilization().unwrap_or_else(|| app.gpu_usage.max(0.0))
        }
        #[cfg(not(windows))]
        {
            app.gpu_usage.max(0.0)
        }
    };
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
    #[allow(unused_variables)]
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    cfg_select! {
        target_os = "macos" => {
            let details_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(10), Constraint::Min(0)])
                .split(cols[1]);

            render_macos_power_panel(f, app, cols[0]);
            render_macos_unified_memory(f, app, details_chunks[0]);
            render_hw_details(f, app, details_chunks[1]);
        },
        target_os = "linux" => {
            if !app.nvidia_gpus.is_empty() {
                render_nvidia_panel(f, app, area);
            } else {
                render_linux_gpu_stats(f, app, cols[0]);
                render_hw_details(f, app, cols[1]);
            }
        },
        _ => {
            render_hw_details(f, app, area);
        }
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
                Span::styled(format!(" {:<18} ", label), Style::default().fg(theme::fg_muted())),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(format!("  {:.2} W", watts), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ])
        }
        None => Line::from(Span::styled(
            format!(" {:<18} ---", label),
            Style::default().fg(theme::fg_muted()),
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
            power_gauge("GPU Power",     app.gpu_power_mw, 30.0),
            power_gauge("ANE Power",     app.ane_power_mw, 10.0),
            Line::from(""),
            power_gauge("Package Power", app.pkg_power_mw, 60.0),
            Line::from(""),
            Line::from(Span::styled(
                " (from powermetrics — live)",
                Style::default().fg(theme::fg_muted()),
            )),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(" No power data.", Style::default().fg(theme::fg_muted()))),
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
            Span::styled(" Temperature      ", Style::default().fg(theme::fg_muted())),
            Span::styled(format!("{}°C", temp), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }

    // GPU Clock
    if let Some(clock) = app.gpu_clock_mhz {
        lines.push(Line::from(vec![
            Span::styled(" GPU Clock        ", Style::default().fg(theme::fg_muted())),
            Span::styled(format!("{} MHz", clock), Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
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
            Span::styled(" VRAM Used        ", Style::default().fg(theme::fg_muted())),
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
            Span::styled(" GPU Busy         ", Style::default().fg(theme::fg_muted())),
            Span::styled(format!("{:.1}%", usage), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }

    // Source attribution
    lines.push(Line::from(Span::styled(
        " (from sysfs / hwmon — live)",
        Style::default().fg(theme::fg_muted()),
    )));

    // If no data at all
    if lines.len() <= 2 {
        lines = vec![
            Line::from(""),
            Line::from(Span::styled(" No GPU telemetry available.", Style::default().fg(theme::fg_muted()))),
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

#[cfg(target_os = "macos")]
fn render_macos_unified_memory(f: &mut Frame, app: &App, area: Rect) {
    let total = app.sys.total_memory() as f64;
    let used = app.sys.used_memory() as f64;
    let ratio = (used / total).clamp(0.0, 1.0);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Unified Memory (System Shared) ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio(ratio)
        .label(format!("{:.1} / {:.1} GiB", used / 1024.0 / 1024.0 / 1024.0, total / 1024.0 / 1024.0 / 1024.0))
        .gauge_style(theme::style_gauge(ratio * 100.0));

    f.render_widget(gauge, area);
}

fn render_hw_details(f: &mut Frame, app: &App, area: Rect) {
    #[cfg(target_os = "macos")]
    let items: Vec<ListItem> = vec![
        ListItem::new(Line::from(vec![
            Span::styled(" Model:          ", Style::default().fg(theme::fg_muted())),
            Span::styled(app.gpu_model.clone(), Style::default().fg(theme::accent())),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled(" Architecture:   ", Style::default().fg(theme::fg_muted())),
            Span::styled("Apple Silicon (Unified Memory)", Style::default().fg(theme::title_fg())),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled(" Memory:         ", Style::default().fg(theme::fg_muted())),
            Span::styled("Shared with CPU (Unified)", Style::default().fg(theme::title_fg())),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled(" Encoder:        ", Style::default().fg(theme::fg_muted())),
            Span::styled("Apple Media Engine (HW)", Style::default().fg(theme::title_fg())),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled(" API Support:    ", Style::default().fg(theme::fg_muted())),
            Span::styled("Metal 3, CoreML, ANE", Style::default().fg(theme::accent2())),
        ])),
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
            ListItem::new(Line::from(vec![
                Span::styled(" Model:          ", Style::default().fg(theme::fg_muted())),
                Span::styled(app.gpu_model.clone(), Style::default().fg(theme::accent())),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled(" Architecture:   ", Style::default().fg(theme::fg_muted())),
                Span::styled(arch, Style::default().fg(theme::title_fg())),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled(" Driver:         ", Style::default().fg(theme::fg_muted())),
                Span::styled(app.gpu_driver.clone(), Style::default().fg(theme::title_fg())),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled(" API Support:    ", Style::default().fg(theme::fg_muted())),
                Span::styled(api_support, Style::default().fg(theme::accent2())),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled(" Vendor:         ", Style::default().fg(theme::fg_muted())),
                Span::styled(app.gpu_vendor.clone(), Style::default().fg(theme::accent())),
            ])),
            ListItem::new(""),
        ]
    };

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let items: Vec<ListItem> = vec![
        ListItem::new(Line::from(vec![
            Span::styled(" Model:          ", Style::default().fg(theme::fg_muted())),
            Span::styled(app.gpu_model.clone(), Style::default().fg(theme::accent())),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled(" Architecture:   ", Style::default().fg(theme::fg_muted())),
            Span::styled("Unknown", Style::default().fg(theme::title_fg())),
        ])),
        ListItem::new(""),
    ];

    let mut final_items = items;

    cfg_select! {
        target_os = "windows" => {
            // On Windows gpu_usage is always -1 (no NVML), use PDH instead.
            let pdh_ok = windows_gpu::gpu_utilization().is_some();
            if pdh_ok {
                let adapters = windows_gpu::gpu_adapter_names();
                if adapters.is_empty() {
                    final_items.push(ListItem::new(Span::styled(
                        " ✓ PDH metrics active (no discrete GPU detected)",
                        Style::default().fg(Color::Rgb(166, 227, 161)),
                    )));
                } else {
                    for adapter in &adapters {
                        final_items.push(ListItem::new(Span::styled(
                            format!(" ✓ {}", adapter),
                            Style::default().fg(Color::Rgb(166, 227, 161)),
                        )));
                    }
                }
            } else {
                final_items.push(ListItem::new(Span::styled(
                    " ⚠ PDH not available — GPU counters unavailable",
                    Style::default().fg(Color::Yellow),
                )));
            }
        },
        _ => {
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
        }
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

#[cfg(windows)]
mod windows_gpu {
    use std::cell::RefCell;
    use windows::core::PCWSTR;
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
    };

    // Simple per-thread history buffer for sparkline on Windows
    thread_local! {
        static GPU_HISTORY: RefCell<Vec<u64>> = RefCell::new(Vec::new());
    }

    pub fn history_tick_and_get(max_len: usize) -> Vec<u64> {
        GPU_HISTORY.with(|cell| {
            let mut v = cell.borrow_mut();
            let sample = gpu_utilization().unwrap_or(0.0).clamp(0.0, 100.0).round() as u64;
            v.push(sample);
            if v.len() > max_len { let drop_n = v.len() - max_len; v.drain(0..drop_n); }
            v.clone()
        })
    }

    struct PdhGpu {
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
    }

    // Cache PDH handles per-thread to avoid Sync/Send requirements on raw handles.
    thread_local! {
        static GPU_HANDLES: RefCell<Option<PdhGpu>> = RefCell::new(None);
    }

    pub fn gpu_utilization() -> Option<f64> {
        GPU_HANDLES.with(|cell| {
            unsafe {
                let mut opt = cell.borrow_mut();
                if opt.is_none() {
                    let mut query = PDH_HQUERY::default();
                    if PdhOpenQueryW(None, 0, &mut query) != 0 {
                        return None;
                    }

                    // PDH local counter path — single leading backslash, no machine prefix.
                    let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage"
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();

                    let mut counter = PDH_HCOUNTER::default();
                    if PdhAddEnglishCounterW(query, PCWSTR(path.as_ptr()), 0, &mut counter) != 0 {
                        return None;
                    }

                    // Prime the query once.
                    let _ = PdhCollectQueryData(query);

                    *opt = Some(PdhGpu { query, counter });
                }

                let h = opt.as_ref().unwrap();

                // Collect a new sample
                if PdhCollectQueryData(h.query) != 0 {
                    return None;
                }

                // First call to get required buffer size and item count
                let mut buf_size: u32 = 0;
                let mut item_count: u32 = 0;
                let mut status = PdhGetFormattedCounterArrayW(
                    h.counter,
                    PDH_FMT_DOUBLE,
                    &mut buf_size,
                    &mut item_count,
                    None,
                );

                if status != 0 && buf_size == 0 {
                    return None;
                }

                // Allocate the buffer and fetch the array
                let mut buffer: Vec<u8> = vec![0u8; buf_size as usize];
                status = PdhGetFormattedCounterArrayW(
                    h.counter,
                    PDH_FMT_DOUBLE,
                    &mut buf_size,
                    &mut item_count,
                    Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
                );
                if status != 0 {
                    return None;
                }

                // Interpret the buffer as an array of PDH_FMT_COUNTERVALUE_ITEM_W
                let items_ptr = buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W;
                let items = std::slice::from_raw_parts(items_ptr, item_count as usize);

                let mut sum_3d_like = 0.0f64;
                let mut sum_all_engines = 0.0f64;
                for item in items {
                    // Read instance name
                    let mut name = String::new();
                    if !item.szName.0.is_null() {
                        let mut len = 0usize;
                        while *item.szName.0.add(len) != 0 { len += 1; }
                        let slice = std::slice::from_raw_parts(item.szName.0, len);
                        name = String::from_utf16_lossy(slice);
                    }

                    let name_lc = name.to_ascii_lowercase();
                    let is_engine = name_lc.contains("engtype_");

                    // Read the double value
                    let val = item.FmtValue.Anonymous.doubleValue;
                    if !val.is_finite() || val < 0.0 { continue; }

                    if is_engine {
                        sum_all_engines += val;
                        // Include common engines users care about on integrated GPUs too
                        if name_lc.contains("engtype_3d")
                            || name_lc.contains("engtype_compute")
                            || name_lc.contains("engtype_copy")
                            || name_lc.contains("engtype_video") {
                            sum_3d_like += val;
                        }
                    }
                }

                // Prefer the subset of engines that represent user-visible workload.
                let mut total = if sum_3d_like > 0.0 { sum_3d_like } else { sum_all_engines };
                total = total.clamp(0.0, 100.0);
                Some(total)
            }
        })
    }

    // PDH-based VRAM usage (Dedicated Usage / Dedicated Limit) in MB
    struct PdhMem {
        query: PDH_HQUERY,
        usage: PDH_HCOUNTER,
        limit: PDH_HCOUNTER,
    }

    thread_local! {
        static MEM_HANDLES: RefCell<Option<PdhMem>> = RefCell::new(None);
    }

    pub fn vram_usage_mb() -> Option<(f64, f64, f64)> {
        MEM_HANDLES.with(|cell| {
            unsafe {
                let mut opt = cell.borrow_mut();
                if opt.is_none() {
                    let mut query = PDH_HQUERY::default();
                    if PdhOpenQueryW(None, 0, &mut query) != 0 { return None; }

                    // PDH local counter paths — single leading backslash.
                    let path_usage: Vec<u16> = "\\GPU Adapter Memory(*)\\Dedicated Usage"
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();
                    let path_limit: Vec<u16> = "\\GPU Adapter Memory(*)\\Dedicated Limit"
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();

                    let mut usage = PDH_HCOUNTER::default();
                    let mut limit = PDH_HCOUNTER::default();
                    if PdhAddEnglishCounterW(query, PCWSTR(path_usage.as_ptr()), 0, &mut usage) != 0 { return None; }
                    if PdhAddEnglishCounterW(query, PCWSTR(path_limit.as_ptr()), 0, &mut limit) != 0 { return None; }

                    // Prime once
                    let _ = PdhCollectQueryData(query);

                    *opt = Some(PdhMem { query, usage, limit });
                }

                let h = opt.as_ref().unwrap();

                // Collect a fresh sample
                if PdhCollectQueryData(h.query) != 0 { return None; }

                // Helper to read an array counter and sum values
                let sum_counter = |counter: PDH_HCOUNTER| -> Option<f64> {
                    let mut buf_size: u32 = 0;
                    let mut item_count: u32 = 0;
                    let mut status = PdhGetFormattedCounterArrayW(
                        counter,
                        PDH_FMT_DOUBLE,
                        &mut buf_size,
                        &mut item_count,
                        None,
                    );
                    if status != 0 && buf_size == 0 { return None; }
                    let mut buffer: Vec<u8> = vec![0u8; buf_size as usize];
                    status = PdhGetFormattedCounterArrayW(
                        counter,
                        PDH_FMT_DOUBLE,
                        &mut buf_size,
                        &mut item_count,
                        Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
                    );
                    if status != 0 { return None; }
                    let items_ptr = buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W;
                    let items = std::slice::from_raw_parts(items_ptr, item_count as usize);
                    let mut sum = 0.0f64;
                    for item in items {
                        let v = item.FmtValue.Anonymous.doubleValue;
                        if v.is_finite() && v >= 0.0 { sum += v; }
                    }
                    Some(sum)
                };

                let used_mb = sum_counter(h.usage)?;
                let limit_mb = sum_counter(h.limit)?;
                if limit_mb <= 0.0 { return None; }
                let pct = (used_mb / limit_mb * 100.0).clamp(0.0, 100.0);
                Some((used_mb, limit_mb, pct))
            }
        })
    }

    /// Enumerate GPU adapters via DXGI — returns display names with dedicated VRAM.
    /// No elevation needed, no extra installs. Same API Direct3D games use.
    pub fn gpu_adapter_names() -> Vec<String> {
        use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_DESC1};

        let mut names = Vec::new();
        unsafe {
            let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
                Ok(f) => f,
                Err(_) => return names,
            };

            let mut i = 0u32;
            loop {
                let adapter = match factory.EnumAdapters1(i) {
                    Ok(a) => a,
                    Err(_) => break, // No more adapters
                };

                let mut desc = DXGI_ADAPTER_DESC1::default();
                if adapter.GetDesc1(&mut desc).is_ok() {
                    // Description is [u16; 128] null-terminated
                    let end = desc.Description.iter().position(|&c| c == 0).unwrap_or(128);
                    let name = String::from_utf16_lossy(&desc.Description[..end]);
                    let vram_mb = desc.DedicatedVideoMemory / (1024 * 1024);
                    // Skip software adapters (WARP, Basic Render Driver, etc.)
                    if vram_mb > 0 {
                        names.push(format!("{} ({} MB VRAM)", name.trim(), vram_mb));
                    }
                }
                i += 1;
            }
        }
        names
    }
}
