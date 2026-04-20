use sysinfo::Process;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, Clear},
    Frame,
};
use crate::app::{App, SortMode, SortDirection, ProcessViewMode, SIGNALS};
use crate::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let mut processes: Vec<_> = app.sys.processes().values().collect();
    
    // Sort and filter
    if app.process_view == ProcessViewMode::List {
        apply_sort(&mut processes, app.sort_mode, app.sort_direction, app);
        let filter = app.filter_text_area.lines()[0].to_string();
        if !filter.is_empty() {
            if app.filter_regex {
                if let Ok(re) = regex::Regex::new(&filter) {
                    processes.retain(|p| {
                        re.is_match(&p.name().to_string_lossy()) ||
                        re.is_match(&p.pid().to_string())
                    });
                }
            } else {
                let filter_str = filter.to_lowercase();
                processes.retain(|p| {
                    p.name().to_string_lossy().to_lowercase().contains(&filter_str) ||
                    p.pid().to_string().contains(&filter_str)
                });
            }
        }
    }

    let total_procs = processes.len();
    if app.selected >= total_procs && total_procs > 0 {
        app.selected = total_procs - 1;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            if app.filter_active { Constraint::Length(3) } else { Constraint::Length(0) },
            Constraint::Min(0),
            if app.show_detail_panel { Constraint::Length(10) } else { Constraint::Length(0) },
        ])
        .split(area);

    let filter_idx = 0;
    let content_idx = 1;
    let detail_idx = 2;

    if app.filter_active {
        app.filter_text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Filter Processes (Enter/Esc to exit) ")
                .border_style(theme::style_tab_active())
        );
        f.render_widget(&app.filter_text_area, chunks[filter_idx]);
    }

    let header_cells = ["PID", "Name", "CPU%", "MEM%", "TOTAL", "VIR", "Disk R/W", "User", "CPU Time", "State"]
        .iter()
        .map(|h| Cell::from(*h).style(theme::style_table_header()));
    
    let header = Row::new(header_cells)
        .style(Style::default().bg(theme::HEADER_BG))
        .height(1);

    let mut rows: Vec<Row> = Vec::new();
    let mut displayed_processes: Vec<&Process> = Vec::new();

    if app.process_view == ProcessViewMode::Tree {
        // Build tree
        let mut children_map: std::collections::HashMap<Option<sysinfo::Pid>, Vec<&Process>> = std::collections::HashMap::new();
        for p in processes.iter() {
            children_map.entry(p.parent()).or_default().push(p);
        }

        // Sort children in each node
        for children in children_map.values_mut() {
            apply_sort(children, app.sort_mode, app.sort_direction, app);
        }

        flatten_tree(&children_map, None, 0, app, &mut rows, &mut displayed_processes);
    } else {
        for (i, p) in processes.iter().enumerate() {
            displayed_processes.push(p);
            rows.push(process_to_row(p, i == app.selected, app, "".to_string()));
        }
    }

    let total_rows = rows.len();
    if app.selected >= total_rows && total_rows > 0 {
        app.selected = total_rows - 1;
    }

    let block_title = format!(
        " Processes ({}) │ Sort: {} │ Filter: {}{} ",
        total_rows,
        app.sort_mode.label(),
        app.filter_text_area.lines()[0],
        if app.filter_regex { " [REGEX]" } else { "" }
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Percentage(25),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Min(8),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(block_title)
            .title_style(theme::style_title())
            .borders(Borders::ALL)
            .border_style(theme::style_border()),
    );

    app.process_table_state.select(Some(app.selected));
    f.render_stateful_widget(table, chunks[content_idx], &mut app.process_table_state);
    if app.show_detail_panel && total_rows > 0 {
        if let Some(p) = displayed_processes.get(app.selected) {
            render_detail_panel(f, p, chunks[detail_idx], app);
        }
    }

    if app.signal_menu_open {
        render_signal_menu(f, area, app.selected_signal);
    }
}

fn render_detail_panel(f: &mut Frame, p: &Process, area: Rect, app: &App) {
    let compressed = app.get_compressed_mem(p.pid());
    let total_footprint = p.memory() + compressed;

    let detail_text = format!(
        "Command: {:?}\nCPU Usage: {:.1}% │ Total: {} │ Resident: {} │ Compressed: {} │ Virtual: {}\nDisk Read: {} │ Disk Write: {}\nStatus: {:?} │ Parent: {:?} │ Uptime: {}s",
        p.cmd(),
        p.cpu_usage(),
        format_size(total_footprint),
        format_size(p.memory()),
        format_size(compressed),
        format_size(p.virtual_memory()),
        format_size(p.disk_usage().total_read_bytes),
        format_size(p.disk_usage().total_written_bytes),
        p.status(),
        p.parent(),
        p.run_time(),
    );

    let p_widget = Paragraph::new(detail_text)
        .block(
            Block::default()
                .title(format!(" Details: {} (PID {}) ", p.name().to_string_lossy(), p.pid()))
                .borders(Borders::ALL)
                .border_style(theme::style_border())
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(p_widget, area);
}

fn render_signal_menu(f: &mut Frame, area: Rect, selected: usize) {
    let menu_area = centered_rect(30, 40, area);
    f.render_widget(Clear, menu_area);

    let items: Vec<ratatui::widgets::ListItem> = SIGNALS.iter().enumerate().map(|(i, sig)| {
        let style = if i == selected {
            theme::style_selected()
        } else {
            Style::default()
        };
        ratatui::widgets::ListItem::new(sig.name).style(style)
    }).collect();

    let list = ratatui::widgets::List::new(items)
        .block(
            Block::default()
                .title(" Send Signal ")
                .borders(Borders::ALL)
                .border_style(theme::style_border())
        );

    f.render_widget(list, menu_area);
}

pub fn apply_sort(processes: &mut Vec<&Process>, mode: SortMode, direction: SortDirection, app: &App) {
    processes.sort_by(|a, b| {
        let ord = match mode {
            SortMode::Cpu => a.cpu_usage().partial_cmp(&b.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal),
            SortMode::Memory => {
                let a_total = a.memory() + app.get_compressed_mem(a.pid());
                let b_total = b.memory() + app.get_compressed_mem(b.pid());
                a_total.cmp(&b_total)
            },
            SortMode::Pid => a.pid().cmp(&b.pid()),
            SortMode::Name => a.name().cmp(&b.name()),
            SortMode::DiskIo => (a.disk_usage().read_bytes + a.disk_usage().written_bytes)
                .cmp(&(b.disk_usage().read_bytes + b.disk_usage().written_bytes)),
            SortMode::Virt => a.virtual_memory().cmp(&b.virtual_memory()),
        };
        if direction == SortDirection::Desc { ord.reverse() } else { ord }
    });
}

fn process_to_row<'a>(p: &'a Process, selected: bool, app: &App, name_prefix: String) -> Row<'a> {
    let style = if selected {
        theme::style_selected()
    } else {
        Style::default()
    };

    let compressed = app.get_compressed_mem(p.pid());
    let total_footprint = p.memory() + compressed;
    let mem_pct = total_footprint as f64 / app.sys.total_memory() as f64 * 100.0;
    let disk_usage = p.disk_usage();
    let acc_time = p.accumulated_cpu_time();
    let h = acc_time / 3600;
    let m = (acc_time % 3600) / 60;
    let s = acc_time % 60;
    let cpu_time_str = if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    };
    
    Row::new(vec![
        Cell::from(p.pid().to_string()),
        Cell::from(name_prefix + &p.name().to_string_lossy()),
        Cell::from(format!("{:.1}%", p.cpu_usage())),
        Cell::from(format!("{:.1}%", mem_pct)),
        Cell::from(format_size(total_footprint)),
        Cell::from(format_size(p.virtual_memory())),
        Cell::from(format!("{} / {}", format_short_size(disk_usage.read_bytes), format_short_size(disk_usage.written_bytes))),
        Cell::from(p.user_id().map(|id| id.to_string()).unwrap_or_else(|| "n/a".into())),
        Cell::from(cpu_time_str),
        Cell::from(format!("{:?}", p.status())),
    ]).style(style)
}

fn flatten_tree<'a>(
    children_map: &std::collections::HashMap<Option<sysinfo::Pid>, Vec<&'a Process>>,
    parent_pid: Option<sysinfo::Pid>,
    depth: usize,
    app: &App,
    rows: &mut Vec<Row<'a>>,
    displayed_processes: &mut Vec<&'a Process>,
) {
    if let Some(children) = children_map.get(&parent_pid) {
        for p in children {
            let is_selected = rows.len() == app.selected;
            displayed_processes.push(p);

            let indent = if depth > 0 {
                "  ".repeat(depth) + "└─ "
            } else {
                "".to_string()
            };
            
            rows.push(process_to_row(p, is_selected, app, indent));
            flatten_tree(children_map, Some(p.pid()), depth + 1, app, rows, displayed_processes);
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GiB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_short_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
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
