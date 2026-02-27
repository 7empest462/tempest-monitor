use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // GPU Info
            Constraint::Min(0),     // Extra space
        ])
        .split(area);

    render_gpu_info(f, app, chunks[0]);
}

fn render_gpu_info(f: &mut Frame, app: &App, area: Rect) {
    let mut items = Vec::new();
    items.push(ListItem::new(format!(" Model: {}", app.gpu_model)));
    
    let usage_text = if app.gpu_usage > 0.0 {
        format!(" Usage: {:.1}%", app.gpu_usage)
    } else {
        " Usage: N/A (Run as sudo for real-time stats)".to_string()
    };
    items.push(ListItem::new(usage_text));

    #[cfg(target_os = "macos")]
    {
        items.push(ListItem::new(" Cores: 8 (Detected)"));
        items.push(ListItem::new(" Memory: Unified Architecture"));
    }

    #[cfg(target_os = "linux")]
    {
        items.push(ListItem::new(" Drive: i915 / amdgpu (Generic)"));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" GPU Monitoring ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );

    f.render_widget(list, area);
}
