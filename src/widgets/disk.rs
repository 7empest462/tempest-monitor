use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["Device", "Mount", "FS", "Total", "Used", "Free", "Usage"]
        .iter()
        .map(|h| Cell::from(*h).style(theme::style_table_header()));
    let header = Row::new(header_cells)
        .style(Style::default().bg(theme::HEADER_BG))
        .height(1);

    let rows: Vec<Row> = app.disks.iter().map(|disk| {
        let total = disk.total_space();
        let avail = disk.available_space();
        let used = total.saturating_sub(avail);
        let pct = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };
        
        // Progress bar string
        let bar_width = 10;
        let filled = (pct / 100.0 * bar_width as f64) as usize;
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_width - filled));

        Row::new(vec![
            Cell::from(disk.name().to_string_lossy().to_string()),
            Cell::from(disk.mount_point().to_string_lossy().to_string()),
            Cell::from(disk.file_system().to_string_lossy().to_string()),
            Cell::from(format_size(total)),
            Cell::from(format_size(used)),
            Cell::from(format_size(avail)),
            Cell::from(format!("{} {:.1}%", bar, pct)).style(Style::default().fg(theme::usage_color(pct))),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(10),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(" Mounted Disks ")
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );

    f.render_widget(table, area);
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_099_511_627_776 {
        format!("{:.1} TiB", bytes as f64 / 1_099_511_627_776.0)
    } else if bytes >= 1_073_741_824 {
        format!("{:.1} GiB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
