use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::app::{ActiveTab, App, ProcessViewMode, SortDirection, SortMode, SIGNALS};

/// Returns `true` if the app should quit.
pub fn handle_event(event: Event, app: &mut App) -> bool {
    match event {
        Event::Key(key) => handle_key(key, app),
        Event::Mouse(mouse) => {
            handle_mouse(mouse, app);
            false
        }
        _ => false,
    }
}

fn handle_key(key: KeyEvent, app: &mut App) -> bool {
    // ── Signal menu takes priority ───────────────────────────────────────
    if app.signal_menu_open {
        return handle_signal_menu_key(key, app);
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => return true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,

        // Tab switching (1–6)
        KeyCode::Char('1') => app.active_tab = ActiveTab::Overview,
        KeyCode::Char('2') => app.active_tab = ActiveTab::Cpu,
        KeyCode::Char('3') => app.active_tab = ActiveTab::Memory,
        KeyCode::Char('4') => app.active_tab = ActiveTab::Disks,
        KeyCode::Char('5') => app.active_tab = ActiveTab::Network,
        KeyCode::Char('6') => app.active_tab = ActiveTab::Processes,

        // Tab cycling
        KeyCode::Tab => {
            let idx = app.active_tab.index();
            let next = (idx + 1) % ActiveTab::ALL.len();
            app.active_tab = ActiveTab::ALL[next];
        }
        KeyCode::BackTab => {
            let idx = app.active_tab.index();
            let prev = if idx == 0 { ActiveTab::ALL.len() - 1 } else { idx - 1 };
            app.active_tab = ActiveTab::ALL[prev];
        }

        // Help
        KeyCode::Char('?') => app.show_help = !app.show_help,

        // Pause/resume
        KeyCode::Char(' ') => app.paused = !app.paused,

        // Refresh rate adjustment
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let ms = app.tick_rate.as_millis();
            if ms > 100 {
                app.tick_rate = std::time::Duration::from_millis((ms - 100) as u64);
            }
        }
        KeyCode::Char('-') => {
            let ms = app.tick_rate.as_millis();
            if ms < 5000 {
                app.tick_rate = std::time::Duration::from_millis((ms + 100) as u64);
            }
        }

        // ── Process-specific keys ────────────────────────────────────────
        // Sorting
        KeyCode::F(1) => {
            if app.sort_mode == SortMode::Cpu {
                app.sort_direction = flip(app.sort_direction);
            } else {
                app.sort_mode = SortMode::Cpu;
                app.sort_direction = SortDirection::Desc;
            }
        }
        KeyCode::F(2) => {
            if app.sort_mode == SortMode::Memory {
                app.sort_direction = flip(app.sort_direction);
            } else {
                app.sort_mode = SortMode::Memory;
                app.sort_direction = SortDirection::Desc;
            }
        }
        KeyCode::F(3) => {
            if app.sort_mode == SortMode::Pid {
                app.sort_direction = flip(app.sort_direction);
            } else {
                app.sort_mode = SortMode::Pid;
                app.sort_direction = SortDirection::Desc;
            }
        }
        KeyCode::F(4) => {
            if app.sort_mode == SortMode::Name {
                app.sort_direction = flip(app.sort_direction);
            } else {
                app.sort_mode = SortMode::Name;
                app.sort_direction = SortDirection::Asc;
            }
        }
        KeyCode::F(5) => {
            if app.sort_mode == SortMode::Virt {
                app.sort_direction = flip(app.sort_direction);
            } else {
                app.sort_mode = SortMode::Virt;
                app.sort_direction = SortDirection::Desc;
            }
        }
        KeyCode::F(6) => {
            if app.sort_mode == SortMode::DiskIo {
                 app.sort_direction = flip(app.sort_direction);
            } else {
                app.sort_mode = SortMode::DiskIo;
                app.sort_direction = SortDirection::Desc;
            }
        }

        // Legacy sort keys (like original)
        KeyCode::Char('c') if app.active_tab == ActiveTab::Processes => {
            app.sort_mode = SortMode::Cpu;
            app.sort_direction = SortDirection::Desc;
        }
        KeyCode::Char('m') if app.active_tab == ActiveTab::Processes => {
            app.sort_mode = SortMode::Memory;
            app.sort_direction = SortDirection::Desc;
        }
        KeyCode::Char('p') if app.active_tab == ActiveTab::Processes => {
            app.sort_mode = SortMode::Pid;
            app.sort_direction = SortDirection::Desc;
        }
        KeyCode::Char('n') if app.active_tab == ActiveTab::Processes => {
            app.sort_mode = SortMode::Name;
            app.sort_direction = SortDirection::Asc;
        }
        KeyCode::Char('v') if app.active_tab == ActiveTab::Processes => {
            app.sort_mode = SortMode::Virt;
            app.sort_direction = SortDirection::Desc;
        }

        // Tree view toggle
        KeyCode::Char('t') if app.active_tab == ActiveTab::Processes => {
            app.process_view = match app.process_view {
                ProcessViewMode::List => ProcessViewMode::Tree,
                ProcessViewMode::Tree => ProcessViewMode::List,
            };
        }

        // Detail panel toggle
        KeyCode::Char('d') if app.active_tab == ActiveTab::Processes => {
            app.show_detail_panel = !app.show_detail_panel;
        }

        // Kill / signal menu
        KeyCode::Char('k') if app.active_tab == ActiveTab::Processes => {
            app.signal_menu_open = true;
            app.selected_signal = 0;
        }

        // Regex filter toggle
        KeyCode::Char('r') if app.active_tab == ActiveTab::Processes => {
            app.filter_regex = !app.filter_regex;
        }

        // Filter start / clear
        KeyCode::Char('/') if app.active_tab == ActiveTab::Processes => {
            app.filter.clear();
        }

        // Navigation
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected = app.selected.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') if app.active_tab != ActiveTab::Processes || !app.signal_menu_open => {
            app.selected = app.selected.saturating_sub(1);
        }
        KeyCode::PageDown => {
            app.selected = app.selected.saturating_add(20);
        }
        KeyCode::PageUp => {
            app.selected = app.selected.saturating_sub(20);
        }
        KeyCode::Home => {
            app.selected = 0;
        }
        KeyCode::End => {
            app.selected = usize::MAX; // clamped during render
        }

        // Filter text input (on Processes tab)
        KeyCode::Char(ch) if app.active_tab == ActiveTab::Processes => {
            app.filter.push(ch);
        }
        KeyCode::Backspace => {
            app.filter.pop();
        }
        KeyCode::Esc => {
            if !app.filter.is_empty() {
                app.filter.clear();
            } else {
                app.show_help = false;
            }
        }

        _ => {}
    }
    false
}

fn handle_signal_menu_key(key: KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.signal_menu_open = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected_signal = app.selected_signal.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_signal + 1 < SIGNALS.len() {
                app.selected_signal += 1;
            }
        }
        KeyCode::Enter => {
            send_signal_to_selected(app);
            app.signal_menu_open = false;
        }
        _ => {}
    }
    false
}

fn handle_mouse(mouse: MouseEvent, app: &mut App) {
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            app.selected = app.selected.saturating_add(3);
        }
        MouseEventKind::ScrollUp => {
            app.selected = app.selected.saturating_sub(3);
        }
        MouseEventKind::Down(_) => {
            // Tab bar click detection (row 1, tabs at columns)
            if mouse.row <= 2 {
                let col = mouse.column as usize;
                // Approximate tab positions based on label widths
                let mut pos = 4; // offset for border
                for tab in ActiveTab::ALL {
                    let label_len = tab.label().len() + 5; // " [N] Label "
                    if col >= pos && col < pos + label_len {
                        app.active_tab = tab;
                        break;
                    }
                    pos += label_len + 1;
                }
            }
        }
        _ => {}
    }
}

fn flip(d: SortDirection) -> SortDirection {
    match d {
        SortDirection::Asc => SortDirection::Desc,
        SortDirection::Desc => SortDirection::Asc,
    }
}

fn send_signal_to_selected(app: &mut App) {

    let mut processes: Vec<_> = app.sys.processes().values().collect();
    crate::widgets::processes::apply_sort(&mut processes, app.sort_mode, app.sort_direction, app);

    if !app.filter.is_empty() {
        if app.filter_regex {
            if let Ok(re) = regex::Regex::new(&app.filter) {
                processes.retain(|p| {
                    re.is_match(&p.name().to_string_lossy()) ||
                    re.is_match(&p.pid().to_string())
                });
            }
        } else {
            let filter_str = app.filter.to_lowercase();
            processes.retain(|p| {
                p.name().to_string_lossy().to_lowercase().contains(&filter_str) ||
                p.pid().to_string().contains(&filter_str)
            });
        }
    }

    if processes.is_empty() || app.selected >= processes.len() {
        return;
    }

    let pid = processes[app.selected].pid();
    let sig = SIGNALS[app.selected_signal].number;

    unsafe {
        libc::kill(pid.as_u32() as i32, sig);
    }
}
