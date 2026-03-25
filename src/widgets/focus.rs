/// Process Focus Dashboard — full-screen drill-down for a single process.
/// Activated when the user presses Enter on a process in the Processes tab.
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pid = match app.focus_pid {
        Some(p) => p,
        None => return,
    };

    let proc = match app.sys.process(pid) {
        Some(p) => p,
        None => {
            let p = Paragraph::new(" Process no longer running.")
                .style(theme::style_muted());
            f.render_widget(p, area);
            return;
        }
    };

    // ── Layout ────────────────────────────────────────────────────────────────
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header: name + pid
            Constraint::Length(12), // Unified Chart (CPU + MEM)
            Constraint::Length(5),  // Gauges row
            Constraint::Min(0),     // Details block
        ])
        .split(area);

    // ── Header ────────────────────────────────────────────────────────────────
    let compressed = app.get_compressed_mem(pid);
    let total_mem  = proc.memory() + compressed;
    let header_line = Line::from(vec![
        Span::styled(" FOCUS: ", theme::style_muted()),
        Span::styled(
            proc.name().to_string_lossy().to_string(),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  PID {}", pid), theme::style_muted()),
        Span::styled(format!("  │  Status: {:?}", proc.status()), theme::style_muted()),
        Span::styled(format!("  │  Uptime: {}s", proc.run_time()), theme::style_muted()),
    ]);
    let header_para = Paragraph::new(header_line).block(
        Block::default()
            .title(" Process Focus  [Esc] to return ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(header_para, chunks[0]);

    // ── Unified Chart ─────────────────────────────────────────────────────────
    crate::widgets::chart::render_focus_chart(f, app, chunks[1]);

    // ── Gauges row ────────────────────────────────────────────────────────────
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);
    let cpu_cur = proc.cpu_usage() as u64;
    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" CPU Load ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio((cpu_cur as f64 / 100.0).clamp(0.0, 1.0))
        .label(format!("{:.1}%", proc.cpu_usage()))
        .gauge_style(theme::style_gauge(cpu_cur as f64));
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    let total_sys_mem = app.sys.total_memory();
    let mem_pct = if total_sys_mem > 0 { total_mem * 100 / total_sys_mem } else { 0 };
    let mem_ratio = (mem_pct as f64 / 100.0).clamp(0.0, 1.0);
    let mem_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Memory Footprint ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .ratio(mem_ratio)
        .label(format!("{} / {}  ({:.1}%)",
            fmt_bytes(total_mem),
            fmt_bytes(total_sys_mem),
            mem_pct
        ))
        .gauge_style(theme::style_gauge(mem_pct as f64));
    f.render_widget(mem_gauge, gauge_chunks[1]);

    // ── Details block ─────────────────────────────────────────────────────────
    let disk = proc.disk_usage();
    let detail = format!(
        " Command: {}\n\n Parent PID: {:?}\n User ID:    {}\n\n Disk Read:  {}  │  Total Read:  {}\n Disk Write: {}  │  Total Write: {}\n\n Resident:   {}\n Compressed: {}\n Virtual:    {}",
        proc.cmd().iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" "),
        proc.parent(),
        proc.user_id().map(|u| u.to_string()).unwrap_or_else(|| "n/a".into()),
        fmt_bytes(disk.read_bytes),
        fmt_bytes(disk.total_read_bytes),
        fmt_bytes(disk.written_bytes),
        fmt_bytes(disk.total_written_bytes),
        fmt_bytes(proc.memory()),
        fmt_bytes(compressed),
        fmt_bytes(proc.virtual_memory()),
    );

    let detail_para = Paragraph::new(detail)
        .block(
            Block::default()
                .title(" Details ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(detail_para, chunks[3]);
}

fn fmt_bytes(b: u64) -> String {
    if b >= 1_073_741_824 {
        format!("{:.1} GiB", b as f64 / 1_073_741_824.0)
    } else if b >= 1_048_576 {
        format!("{:.1} MiB", b as f64 / 1_048_576.0)
    } else if b >= 1024 {
        format!("{:.1} KiB", b as f64 / 1024.0)
    } else {
        format!("{} B", b)
    }
}
