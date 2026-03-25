use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{ActiveTab, App, ProcessViewMode, SortDirection, SortMode, SIGNALS};

pub fn handle_key(key: KeyEvent, app: &mut App) -> bool {
    // ── Signal menu takes priority ───────────────────────────────────────
    if app.signal_menu_open {
        return handle_signal_menu_key(key, app);
    }

    // ── Filter mode takes priority ───────────────────────────────────────
    if app.filter_active {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.filter_active = false;
            }
            _ => {
                app.filter_text_area.input(key);
            }
        }
        return false;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => return false,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return false,

        // Tab switching (1–9)
        KeyCode::Char('1') => app.active_tab = ActiveTab::Overview,
        KeyCode::Char('2') => app.active_tab = ActiveTab::Cpu,
        KeyCode::Char('3') => app.active_tab = ActiveTab::Memory,
        KeyCode::Char('4') => app.active_tab = ActiveTab::Disks,
        KeyCode::Char('5') => app.active_tab = ActiveTab::Network,
        KeyCode::Char('6') => app.active_tab = ActiveTab::Processes,
        KeyCode::Char('7') => app.active_tab = ActiveTab::Gpu,
        KeyCode::Char('8') => {
            app.active_tab = ActiveTab::Services;
            app.refresh_services();
        }
        KeyCode::Char('9') => {
            app.active_tab = ActiveTab::Sockets;
            if app.sockets.is_empty() { app.refresh_sockets(); }
        }

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
            app.filter_active = true;
            // Clear on first '/' hit
            app.filter_text_area = tui_textarea::TextArea::default();
        }

        // Ctrl+U to clear filter (even if not active)
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.filter_text_area = tui_textarea::TextArea::default();
        }

        // Navigation — routes to the right selection field per active tab
        KeyCode::Down | KeyCode::Char('j') => {
            match app.active_tab {
                ActiveTab::Services => {
                    let max = app.services.len().saturating_sub(1);
                    app.service_selected = (app.service_selected + 1).min(max);
                }
                ActiveTab::Sockets => {
                    let max = app.sockets.len().saturating_sub(1);
                    app.socket_selected = (app.socket_selected + 1).min(max);
                }
                _ => { app.selected = app.selected.saturating_add(1); }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            match app.active_tab {
                ActiveTab::Services => {
                    app.service_selected = app.service_selected.saturating_sub(1);
                }
                ActiveTab::Sockets => {
                    app.socket_selected = app.socket_selected.saturating_sub(1);
                }
                _ if !app.signal_menu_open => {
                    app.selected = app.selected.saturating_sub(1);
                }
                _ => {}
            }
        }
        KeyCode::PageDown => {
            match app.active_tab {
                ActiveTab::Services => {
                    let max = app.services.len().saturating_sub(1);
                    app.service_selected = (app.service_selected + 20).min(max);
                }
                ActiveTab::Sockets => {
                    let max = app.sockets.len().saturating_sub(1);
                    app.socket_selected = (app.socket_selected + 20).min(max);
                }
                _ => { app.selected = app.selected.saturating_add(20); }
            }
        }
        KeyCode::PageUp => {
            match app.active_tab {
                ActiveTab::Services => { app.service_selected = app.service_selected.saturating_sub(20); }
                ActiveTab::Sockets  => { app.socket_selected = app.socket_selected.saturating_sub(20); }
                _ => { app.selected = app.selected.saturating_sub(20); }
            }
        }
        KeyCode::Home => {
            match app.active_tab {
                ActiveTab::Services => { app.service_selected = 0; }
                ActiveTab::Sockets  => { app.socket_selected = 0; }
                _ => { app.selected = 0; }
            }
        }
        KeyCode::End => {
            match app.active_tab {
                ActiveTab::Services => { app.service_selected = app.services.len().saturating_sub(1); }
                ActiveTab::Sockets  => { app.socket_selected = app.sockets.len().saturating_sub(1); }
                _ => { app.selected = usize::MAX; } // clamped during render
            }
        }

        // Filter text input (legacy - now handled by filter_active block)
        // KeyCode::Char(ch) if app.active_tab == ActiveTab::Processes => {
        //     app.filter.push(ch);
        // }
        // KeyCode::Backspace => {
        //     app.filter.pop();
        // }
        // Esc: exit focus or close help
        KeyCode::Esc => {
            if app.focus_pid.is_some() {
                app.focus_pid = None;
                app.focus_cpu_history.clear();
                app.focus_mem_history.clear();
            } else {
                app.show_help = false;
            }
        }

        // Enter: focus mode (Processes tab) or service action (Services tab)
        KeyCode::Enter => {
            if app.active_tab == ActiveTab::Processes && app.focus_pid.is_none() {
                // Launch focus for selected process
                let mut procs: Vec<_> = app.sys.processes().values().collect();
                crate::widgets::processes::apply_sort(&mut procs, app.sort_mode, app.sort_direction, app);
                if let Some(p) = procs.get(app.selected) {
                    app.focus_pid = Some(p.pid());
                    app.focus_cpu_history.clear();
                    app.focus_mem_history.clear();
                }
            } else if app.active_tab == ActiveTab::Services {
                crate::widgets::services::run_service_action(app, "start");
            }
        }

        // Services tab actions
        KeyCode::Char('s') if app.active_tab == ActiveTab::Services => {
            crate::widgets::services::run_service_action(app, "stop");
        }

        _ => {}
    }
    true
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
    true
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

    if processes.is_empty() || app.selected >= processes.len() {
        return;
    }

    let pid = processes[app.selected].pid();
    let sig = SIGNALS[app.selected_signal].number;

    unsafe {
        libc::kill(pid.as_u32() as i32, sig);
    }
}
