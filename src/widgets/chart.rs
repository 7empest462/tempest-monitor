use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame,
};
use crate::app::App;
use crate::theme;

pub fn render_focus_chart(f: &mut Frame, app: &App, area: Rect) {
    let cpu_data: Vec<(f64, f64)> = app.focus_cpu_history
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v as f64))
        .collect();

    let mem_data: Vec<(f64, f64)> = app.focus_mem_history
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v as f64))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name("CPU %")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&cpu_data),
        Dataset::default()
            .name("MEM %")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Magenta))
            .data(&mem_data),
    ];

    let x_axis = Axis::default()
        .title(Span::styled("Time", Style::default().fg(theme::FG_MUTED)))
        .style(Style::default().fg(theme::FG_MUTED))
        .bounds([0.0, 120.0])
        .labels(vec![
            Span::raw("-120s"),
            Span::raw("-60s"),
            Span::raw("Now"),
        ]);

    let y_axis = Axis::default()
        .title(Span::styled("Usage %", Style::default().fg(theme::FG_MUTED)))
        .style(Style::default().fg(theme::FG_MUTED))
        .bounds([0.0, 100.0])
        .labels(vec![
            Span::raw("0%"),
            Span::raw("50%"),
            Span::raw("100%"),
        ]);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(" Process Focus Timeline ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border())
        )
        .x_axis(x_axis)
        .y_axis(y_axis);

    f.render_widget(chart, area);
}
