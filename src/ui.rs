use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::app::{ActiveTab, App};
use crate::theme;
use crate::widgets;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(0),   // Main Content
            Constraint::Length(1), // Footer
        ])
        .split(size);

    draw_header(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    if app.show_help {
        draw_help(f, size);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<_> = ActiveTab::ALL
        .iter()
        .map(|t| format!(" [{}] {} ", t.index() + 1, t.label()))
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" 7EMPEST MONITOR v{} ", env!("CARGO_PKG_VERSION")))
                .title_style(theme::style_title())
                .border_style(theme::style_border()),
        )
        .select(app.active_tab.index())
        .highlight_style(theme::style_tab_active())
        .style(Style::default().fg(theme::FG_MUTED));

    f.render_widget(tabs, area);
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    // If focus mode is active, render the focus dashboard instead
    if app.focus_pid.is_some() {
        widgets::focus::render(f, app, area);
        return;
    }

    match app.active_tab {
        ActiveTab::Overview  => widgets::overview::render(f, app, area),
        ActiveTab::Cpu       => widgets::cpu::render(f, app, area),
        ActiveTab::Memory    => widgets::memory::render(f, app, area),
        ActiveTab::Disks     => widgets::disk::render(f, app, area),
        ActiveTab::Network   => widgets::network::render(f, app, area),
        ActiveTab::Processes => widgets::processes::render(f, app, area),
        ActiveTab::Gpu       => widgets::gpu::render(f, app, area),
        ActiveTab::Services  => widgets::services::render(f, app, area),
        ActiveTab::Sockets   => widgets::sockets::render(f, app, area),
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.focus_pid.is_some() {
        format!(" [Esc] Exit Focus │ [Space] {} │ [+/-] Rate: {}",
            if app.paused { "Resume" } else { "Pause" },
            app.tick_rate_label())
    } else {
        format!(" [q] Quit │ [?] Help │ [Space] {} │ [+/-] Rate: {} │ Tabs 1-9",
            if app.paused { "Resume" } else { "Pause" },
            app.tick_rate_label())
    };

    let p = Paragraph::new(help_text).style(theme::style_footer());
    f.render_widget(p, area);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let help_area = centered_rect(60, 60, area);
    f.render_widget(ratatui::widgets::Clear, help_area);

    let text = "
  NAVIGATION
  [1-9]       Switch Tabs (9 tabs total)
  [Tab]       Next Tab
  [q]         Quit App
  [Space]     Pause/Resume
  [+ / -]     Adjust Refresh Rate
  [?]         Toggle Help
  
  PROCESSES (Tab 6)
  [c/m/v/p/n] Sort by CPU/Mem/Virt/Pid/Name
  [F1-F6]     Advanced Sort Controls
  [t]         Toggle Tree View
  [d]         Toggle Detail Panel
  [Enter]     FOCUS: Full-screen process view
  [Esc]       Exit Focus mode
  [/]         Filter Processes
  [k]         Open Signal Menu
  [r]         Toggle Regex Search
  
  SERVICES (Tab 8)
  [↑↓/j/k]   Navigate services
  [Enter]     Start service
  [s]         Stop service
  [r]         Restart service

  SOCKETS (Tab 9)
  [↑↓/j/k]   Navigate connections
  
  SIGNALS (Signal Menu)
  [Esc]       Close Menu
  [j/k]       Move Selection
  [Enter]     Send Signal
";

    let p = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Help / Keybindings ")
                .title_style(theme::style_title())
                .borders(Borders::ALL)
                .border_style(theme::style_border())
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(p, help_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
