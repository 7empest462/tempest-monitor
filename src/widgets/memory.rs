use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Sparkline},
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
            Constraint::Length(5), // RAM gauge
            Constraint::Length(5), // SWAP gauge
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

    // Stacked RAM bar calculation
    let inner_width = (chunks[2].width as usize).saturating_sub(2);
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
            " {:.2} GiB used ({:.0}%) / {:.2} GiB total │ Cache: {:.2} GiB │ Free: {:.2} GiB",
            used_gib,
            pct.clamp(0.0, 100.0),
            total_gib,
            cache_gib,
            free_gib,
        ))
    ]);

    let ram_legend_line = ratatui::text::Line::from(cfg_select! {
        target_os = "macos" => {
            vec![
                ratatui::text::Span::raw(" Legend: "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::usage_color(0.0))),
                ratatui::text::Span::raw(" App Memory   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::accent())),
                ratatui::text::Span::raw(" Wired   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::usage_color(50.0))),
                ratatui::text::Span::raw(" Cached Files   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::fg_muted())),
                ratatui::text::Span::raw(" Free"),
            ]
        },
        target_os = "windows" => {
            vec![
                ratatui::text::Span::raw(" Legend: "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::usage_color(0.0))),
                ratatui::text::Span::raw(" Working Set   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::accent())),
                ratatui::text::Span::raw(" Non-Paged Pool   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::usage_color(50.0))),
                ratatui::text::Span::raw(" Paged Pool   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::fg_muted())),
                ratatui::text::Span::raw(" Free"),
            ]
        },
        _ => {
            vec![
                ratatui::text::Span::raw(" Legend: "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::usage_color(0.0))),
                ratatui::text::Span::raw(" Apps   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::accent())),
                ratatui::text::Span::raw(" Buffers   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::usage_color(50.0))),
                ratatui::text::Span::raw(" Cache   "),
                ratatui::text::Span::styled("█", Style::default().fg(theme::fg_muted())),
                ratatui::text::Span::raw(" Free"),
            ]
        }
    });

    let ram_paragraph = ratatui::widgets::Paragraph::new(vec![
        ratatui::text::Line::from(ram_spans),
        ram_label_line,
        ram_legend_line,
    ])
    .block(
        Block::default()
            .title(" RAM ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(ram_paragraph, chunks[2]);

    // SWAP gauge
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
            " {:.2} GiB used ({:.0}%) / {:.2} GiB total │ {:.2} GiB free",
            used_sw as f64 / 1_073_741_824.0,
            pct_sw.clamp(0.0, 100.0),
            total_sw as f64 / 1_073_741_824.0,
            free_sw as f64 / 1_073_741_824.0,
        ))
    ]);

    let swap_legend_line = ratatui::text::Line::from(vec![
        ratatui::text::Span::raw(" Legend: "),
        ratatui::text::Span::styled("█", Style::default().fg(theme::accent2())),
        ratatui::text::Span::raw(" Used   "),
        ratatui::text::Span::styled("█", Style::default().fg(theme::fg_muted())),
        ratatui::text::Span::raw(" Free"),
    ]);

    let swap_paragraph = ratatui::widgets::Paragraph::new(vec![
        ratatui::text::Line::from(swap_spans),
        swap_label_line,
        swap_legend_line,
    ])
    .block(
        Block::default()
            .title(" SWAP ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );
    f.render_widget(swap_paragraph, chunks[3]);

    // Memory details
    let sys_used = app.sys.used_memory();
    let sys_avail = app.sys.available_memory();
    let sys_free = app.mem_segments.free;

    let detail_text = format!(
        " Total: {:.2} GiB │ Used: {:.2} GiB │ Free: {:.2} GiB │ Available: {:.2} GiB\n Swap Total: {:.2} GiB │ Swap Used: {:.2} GiB │ Swap Free: {:.2} GiB",
        total as f64 / 1_073_741_824.0,
        sys_used as f64 / 1_073_741_824.0,
        sys_free as f64 / 1_073_741_824.0,
        sys_avail as f64 / 1_073_741_824.0,
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
