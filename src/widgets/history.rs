use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::app::App;
use crate::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    // If the database feature is disabled, render a clean stub message
    #[cfg(not(feature = "database"))]
    {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Database feature is disabled in this build.",
                Style::default().fg(Color::Rgb(249, 226, 175)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  To enable, recompile with `--features database`.",
                Style::default().fg(theme::fg_muted()),
            )),
        ])
        .block(
            Block::default()
                .title(" Historical Performance Metrics ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );
        f.render_widget(p, area);
        return;
    }

    #[cfg(feature = "database")]
    {
        let snapshots = &app.history.snapshots;

        if snapshots.is_empty() {
            let p = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No historical snapshots recorded yet.",
                    Style::default().fg(theme::fg_muted()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Wait a few seconds for the telemetry loop to populate the database.",
                    Style::default().fg(theme::fg_muted()),
                )),
            ])
            .block(
                Block::default()
                    .title(" Historical Performance Metrics ")
                    .title_style(theme::style_title())
                    .borders(Borders::ALL)
                    .border_style(theme::style_border()),
            );
            f.render_widget(p, area);
            return;
        }

        // Calculate summary statistics
        let count = snapshots.len() as f64;
        let mut cpu_min = f64::MAX;
        let mut cpu_max = f64::MIN;
        let mut cpu_sum = 0.0;

        let mut mem_min = f64::MAX;
        let mut mem_max = f64::MIN;
        let mut mem_sum = 0.0;

        let mut gpu_min = f64::MAX;
        let mut gpu_max = f64::MIN;
        let mut gpu_sum = 0.0;

        let mut rx_min = f64::MAX;
        let mut rx_max = f64::MIN;
        let mut rx_sum = 0.0;

        let mut tx_min = f64::MAX;
        let mut tx_max = f64::MIN;
        let mut tx_sum = 0.0;

        for s in snapshots {
            cpu_min = cpu_min.min(s.cpu_usage);
            cpu_max = cpu_max.max(s.cpu_usage);
            cpu_sum += s.cpu_usage;

            mem_min = mem_min.min(s.mem_used_gb);
            mem_max = mem_max.max(s.mem_used_gb);
            mem_sum += s.mem_used_gb;

            gpu_min = gpu_min.min(s.gpu_usage);
            gpu_max = gpu_max.max(s.gpu_usage);
            gpu_sum += s.gpu_usage;

            rx_min = rx_min.min(s.net_rx_kbps);
            rx_max = rx_max.max(s.net_rx_kbps);
            rx_sum += s.net_rx_kbps;

            tx_min = tx_min.min(s.net_tx_kbps);
            tx_max = tx_max.max(s.net_tx_kbps);
            tx_sum += s.net_tx_kbps;
        }

        let cpu_avg = cpu_sum / count;
        let mem_avg = mem_sum / count;
        let gpu_avg = gpu_sum / count;
        let rx_avg = rx_sum / count;
        let tx_avg = tx_sum / count;

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Summary panels
                Constraint::Min(0),    // Table of recent records
            ])
            .split(area);

        // Render Summary Panels (5 columns)
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ])
            .split(main_chunks[0]);

        // CPU Summary Panel
        let cpu_lines = vec![
            Line::from(vec![
                Span::styled("Avg: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1}%", cpu_avg),
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Min: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1}%", cpu_min),
                    Style::default().fg(Color::Rgb(166, 227, 161)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Max: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1}%", cpu_max),
                    Style::default().fg(Color::Rgb(243, 139, 168)),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(cpu_lines).block(
                Block::default()
                    .title(" CPU History ")
                    .title_style(theme::style_title())
                    .borders(Borders::ALL)
                    .border_style(theme::style_border()),
            ),
            cols[0],
        );

        // Memory Summary Panel
        let mem_lines = vec![
            Line::from(vec![
                Span::styled("Avg: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.2} GB", mem_avg),
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Min: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.2} GB", mem_min),
                    Style::default().fg(Color::Rgb(166, 227, 161)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Max: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.2} GB", mem_max),
                    Style::default().fg(Color::Rgb(243, 139, 168)),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(mem_lines).block(
                Block::default()
                    .title(" Memory History ")
                    .title_style(theme::style_title())
                    .borders(Borders::ALL)
                    .border_style(theme::style_border()),
            ),
            cols[1],
        );

        // GPU Summary Panel
        let gpu_lines = vec![
            Line::from(vec![
                Span::styled("Avg: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1}%", gpu_avg),
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Min: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1}%", gpu_min),
                    Style::default().fg(Color::Rgb(166, 227, 161)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Max: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1}%", gpu_max),
                    Style::default().fg(Color::Rgb(243, 139, 168)),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(gpu_lines).block(
                Block::default()
                    .title(" GPU History ")
                    .title_style(theme::style_title())
                    .borders(Borders::ALL)
                    .border_style(theme::style_border()),
            ),
            cols[2],
        );

        // Net Rx Summary Panel
        let rx_lines = vec![
            Line::from(vec![
                Span::styled("Avg: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1} KB/s", rx_avg),
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Min: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1} KB/s", rx_min),
                    Style::default().fg(Color::Rgb(166, 227, 161)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Max: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1} KB/s", rx_max),
                    Style::default().fg(Color::Rgb(243, 139, 168)),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(rx_lines).block(
                Block::default()
                    .title(" Net Down History ")
                    .title_style(theme::style_title())
                    .borders(Borders::ALL)
                    .border_style(theme::style_border()),
            ),
            cols[3],
        );

        // Net Tx Summary Panel
        let tx_lines = vec![
            Line::from(vec![
                Span::styled("Avg: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1} KB/s", tx_avg),
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Min: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1} KB/s", tx_min),
                    Style::default().fg(Color::Rgb(166, 227, 161)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Max: ", Style::default().fg(theme::fg_muted())),
                Span::styled(
                    format!("{:.1} KB/s", tx_max),
                    Style::default().fg(Color::Rgb(243, 139, 168)),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(tx_lines).block(
                Block::default()
                    .title(" Net Up History ")
                    .title_style(theme::style_title())
                    .borders(Borders::ALL)
                    .border_style(theme::style_border()),
            ),
            cols[4],
        );

        // Render raw record table below
        let header = Row::new([
            Cell::from("ID").style(theme::style_table_header()),
            Cell::from("Timestamp").style(theme::style_table_header()),
            Cell::from("CPU Usage").style(theme::style_table_header()),
            Cell::from("RAM Used").style(theme::style_table_header()),
            Cell::from("GPU Usage").style(theme::style_table_header()),
            Cell::from("Net Download").style(theme::style_table_header()),
            Cell::from("Net Upload").style(theme::style_table_header()),
        ])
        .style(Style::default().bg(theme::header_bg()))
        .height(1);

        let rows: Vec<Row> = snapshots
            .iter()
            .map(|s| {
                Row::new(vec![
                    Cell::from(s.id.to_string()),
                    Cell::from(s.timestamp.clone()),
                    Cell::from(format!("{:.1}%", s.cpu_usage)),
                    Cell::from(format!("{:.2} GB", s.mem_used_gb)),
                    Cell::from(if s.gpu_usage >= 0.0 {
                        format!("{:.1}%", s.gpu_usage)
                    } else {
                        "N/A".to_string()
                    }),
                    Cell::from(format!("{:.1} KB/s", s.net_rx_kbps)),
                    Cell::from(format!("{:.1} KB/s", s.net_tx_kbps)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(22),
                Constraint::Length(14),
                Constraint::Length(14),
                Constraint::Length(14),
                Constraint::Length(16),
                Constraint::Min(16),
            ],
        )
        .header(header)
        .row_highlight_style(theme::style_selected())
        .block(
            Block::default()
                .title(format!(
                    " Historical Metric Records (showing last {}) │ [↑↓/jk/PgUp/PgDn] Scroll ",
                    snapshots.len()
                ))
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border()),
        );

        let mut state =
            ratatui::widgets::TableState::default().with_selected(Some(app.history.selected));
        f.render_stateful_widget(table, main_chunks[1], &mut state);
    }
}
