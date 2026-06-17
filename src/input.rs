use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{ActiveTab, App, ProcessViewMode, ServiceInspectorMode, SortDirection, SortMode};
use crate::power_mode::CpuPowerMode;

pub fn handle_key(key: KeyEvent, app: &mut App) -> bool {
    // ── Service Inspector takes priority ─────────────────────────────────
    if app.services.inspector_open {
        return handle_inspector_key(key, app);
    }

    // ── Signal menu takes priority ───────────────────────────────────────
    if app.processes.signal_menu_open {
        return handle_signal_menu_key(key, app);
    }

    // ── Filter mode takes priority ───────────────────────────────────────
    if app.processes.filter_active {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.processes.filter_active = false;
            }
            _ => {
                app.processes.filter_text_area.input(key);
            }
        }
        return false;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => return false,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return false,

        // Tab switching (1–0)
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
            if app.sockets.list.is_empty() { app.refresh_sockets(); }
        }
        KeyCode::Char('0') => {
            app.active_tab = ActiveTab::History;
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

        // Theme cycling:
        // 'T' cycles globally. 't' cycles on all tabs except processes (where it toggles tree view)
        KeyCode::Char('T') => {
            app.cycle_theme();
        }
        KeyCode::Char('t') if app.active_tab != ActiveTab::Processes => {
            app.cycle_theme();
        }

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
            if app.processes.sort_mode == SortMode::Cpu {
                app.processes.sort_direction = flip(app.processes.sort_direction);
            } else {
                app.processes.sort_mode = SortMode::Cpu;
                app.processes.sort_direction = SortDirection::Desc;
            }
        }
        KeyCode::F(2) => {
            if app.processes.sort_mode == SortMode::Memory {
                app.processes.sort_direction = flip(app.processes.sort_direction);
            } else {
                app.processes.sort_mode = SortMode::Memory;
                app.processes.sort_direction = SortDirection::Desc;
            }
        }
        KeyCode::F(3) => {
            if app.processes.sort_mode == SortMode::Pid {
                app.processes.sort_direction = flip(app.processes.sort_direction);
            } else {
                app.processes.sort_mode = SortMode::Pid;
                app.processes.sort_direction = SortDirection::Desc;
            }
        }
        KeyCode::F(4) => {
            if app.processes.sort_mode == SortMode::Name {
                app.processes.sort_direction = flip(app.processes.sort_direction);
            } else {
                app.processes.sort_mode = SortMode::Name;
                app.processes.sort_direction = SortDirection::Asc;
            }
        }
        KeyCode::F(5) => {
            if app.processes.sort_mode == SortMode::Virt {
                app.processes.sort_direction = flip(app.processes.sort_direction);
            } else {
                app.processes.sort_mode = SortMode::Virt;
                app.processes.sort_direction = SortDirection::Desc;
            }
        }
        KeyCode::F(6) => {
            if app.processes.sort_mode == SortMode::DiskIo {
                 app.processes.sort_direction = flip(app.processes.sort_direction);
            } else {
                app.processes.sort_mode = SortMode::DiskIo;
                app.processes.sort_direction = SortDirection::Desc;
            }
        }

        // ── CPU Performance Mode keys (CPU tab) ─────────────────────────
        KeyCode::F(7) if app.active_tab == ActiveTab::Cpu => {
            let target = CpuPowerMode::LowPower;
            if app.cpu_power.available_modes.contains(&target) {
                app.set_cpu_power_mode(target);
            }
        }
        KeyCode::F(8) if app.active_tab == ActiveTab::Cpu => {
            let target = if cfg!(target_os = "linux") {
                CpuPowerMode::Balanced
            } else {
                CpuPowerMode::Normal
            };
            if app.cpu_power.available_modes.contains(&target) {
                app.set_cpu_power_mode(target);
            }
        }
        KeyCode::F(9) if app.active_tab == ActiveTab::Cpu => {
            // Linux only: Performance mode
            if app.cpu_power.available_modes.contains(&CpuPowerMode::Performance) {
                app.set_cpu_power_mode(CpuPowerMode::Performance);
            }
        }

        // Legacy sort keys (like original)
        KeyCode::Char('c') if app.active_tab == ActiveTab::Processes => {
            app.processes.sort_mode = SortMode::Cpu;
            app.processes.sort_direction = SortDirection::Desc;
        }
        KeyCode::Char('m') if app.active_tab == ActiveTab::Processes => {
            app.processes.sort_mode = SortMode::Memory;
            app.processes.sort_direction = SortDirection::Desc;
        }
        KeyCode::Char('p') if app.active_tab == ActiveTab::Processes => {
            app.processes.sort_mode = SortMode::Pid;
            app.processes.sort_direction = SortDirection::Desc;
        }
        KeyCode::Char('n') if app.active_tab == ActiveTab::Processes => {
            app.processes.sort_mode = SortMode::Name;
            app.processes.sort_direction = SortDirection::Asc;
        }
        KeyCode::Char('v') if app.active_tab == ActiveTab::Processes => {
            app.processes.sort_mode = SortMode::Virt;
            app.processes.sort_direction = SortDirection::Desc;
        }

        // Tree view toggle
        KeyCode::Char('t') if app.active_tab == ActiveTab::Processes => {
            app.processes.view_mode = match app.processes.view_mode {
                ProcessViewMode::List => ProcessViewMode::Tree,
                ProcessViewMode::Tree => ProcessViewMode::List,
            };
        }

        // Detail panel toggle
        KeyCode::Char('d') if app.active_tab == ActiveTab::Processes => {
            app.processes.show_detail_panel = !app.processes.show_detail_panel;
        }

        // Kill / signal menu
        KeyCode::Char('k') if app.active_tab == ActiveTab::Processes => {
            app.processes.signal_menu_open = true;
            app.processes.selected_signal = 0;
        }

        // Regex filter toggle
        KeyCode::Char('r') if app.active_tab == ActiveTab::Processes => {
            app.processes.filter_regex = !app.processes.filter_regex;
        }

        // Filter start / clear
        KeyCode::Char('/') if app.active_tab == ActiveTab::Processes => {
            app.processes.filter_active = true;
            // Clear on first '/' hit
            app.processes.filter_text_area = ratatui_textarea::TextArea::default();
        }

        // Ctrl+U to clear filter (even if not active)
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.processes.filter_text_area = ratatui_textarea::TextArea::default();
        }

        // Navigation — routes to the right selection field per active tab
        KeyCode::Down | KeyCode::Char('j') => {
            match app.active_tab {
                ActiveTab::Services => {
                    let max = app.services.list.len().saturating_sub(1);
                    app.services.selected = (app.services.selected + 1).min(max);
                }
                ActiveTab::Sockets => {
                    let max = app.sockets.list.len().saturating_sub(1);
                    app.sockets.selected = (app.sockets.selected + 1).min(max);
                }
                ActiveTab::History => {
                    let max = app.history.snapshots.len().saturating_sub(1);
                    app.history.selected = (app.history.selected + 1).min(max);
                }
                _ => { app.processes.selected = app.processes.selected.saturating_add(1); }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            match app.active_tab {
                ActiveTab::Services => {
                    app.services.selected = app.services.selected.saturating_sub(1);
                }
                ActiveTab::Sockets => {
                    app.sockets.selected = app.sockets.selected.saturating_sub(1);
                }
                ActiveTab::History => {
                    app.history.selected = app.history.selected.saturating_sub(1);
                }
                _ if !app.processes.signal_menu_open => {
                    app.processes.selected = app.processes.selected.saturating_sub(1);
                }
                _ => {}
            }
        }
        KeyCode::PageDown => {
            match app.active_tab {
                ActiveTab::Services => {
                    let max = app.services.list.len().saturating_sub(1);
                    app.services.selected = (app.services.selected + 20).min(max);
                }
                ActiveTab::Sockets => {
                    let max = app.sockets.list.len().saturating_sub(1);
                    app.sockets.selected = (app.sockets.selected + 20).min(max);
                }
                ActiveTab::History => {
                    let max = app.history.snapshots.len().saturating_sub(1);
                    app.history.selected = (app.history.selected + 20).min(max);
                }
                _ => { app.processes.selected = app.processes.selected.saturating_add(20); }
            }
        }
        KeyCode::PageUp => {
            match app.active_tab {
                ActiveTab::Services => { app.services.selected = app.services.selected.saturating_sub(20); }
                ActiveTab::Sockets  => { app.sockets.selected = app.sockets.selected.saturating_sub(20); }
                ActiveTab::History  => { app.history.selected = app.history.selected.saturating_sub(20); }
                _ => { app.processes.selected = app.processes.selected.saturating_sub(20); }
            }
        }
        KeyCode::Home => {
            match app.active_tab {
                ActiveTab::Services => { app.services.selected = 0; }
                ActiveTab::Sockets  => { app.sockets.selected = 0; }
                ActiveTab::History  => { app.history.selected = 0; }
                _ => { app.processes.selected = 0; }
            }
        }
        KeyCode::End => {
            match app.active_tab {
                ActiveTab::Services => { app.services.selected = app.services.list.len().saturating_sub(1); }
                ActiveTab::Sockets  => { app.sockets.selected = app.sockets.list.len().saturating_sub(1); }
                ActiveTab::History  => { app.history.selected = app.history.snapshots.len().saturating_sub(1); }
                _ => { app.processes.selected = usize::MAX; } // clamped during render
            }
        }

        // Esc: exit focus or close help
        KeyCode::Esc => {
            if app.processes.focus_pid.is_some() {
                app.processes.focus_pid = None;
                app.processes.focus_cpu_history.clear();
                app.processes.focus_mem_history.clear();
            } else {
                app.show_help = false;
            }
        }

        // Enter: focus mode (Processes tab) or service inspector (Services tab)
        KeyCode::Enter => {
            if app.active_tab == ActiveTab::Processes && app.processes.focus_pid.is_none() {
                // Launch focus for selected process
                let mut procs: Vec<_> = app.sys.processes().values().collect();
                crate::widgets::processes::apply_sort(&mut procs, app.processes.sort_mode, app.processes.sort_direction, app);
                if let Some(p) = procs.get(app.processes.selected) {
                    app.processes.focus_pid = Some(p.pid());
                    app.processes.focus_cpu_history.clear();
                    app.processes.focus_mem_history.clear();
                }
            } else if app.active_tab == ActiveTab::Services {
                // Open service inspector instead of starting
                app.open_service_inspector();
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
            app.processes.signal_menu_open = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.processes.selected_signal = app.processes.selected_signal.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.processes.selected_signal + 1 < crate::platform::get_signals().len() {
                app.processes.selected_signal += 1;
            }
        }
        KeyCode::Enter => {
            send_signal_to_selected(app);
            app.processes.signal_menu_open = false;
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
    crate::widgets::processes::apply_sort(&mut processes, app.processes.sort_mode, app.processes.sort_direction, app);

    let filter = app.processes.filter_text_area.lines()[0].to_string();
    if !filter.is_empty() {
        if app.processes.filter_regex {
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

    if processes.is_empty() || app.processes.selected >= processes.len() {
        return;
    }

    let pid = processes[app.processes.selected].pid();

    crate::platform::kill_process(pid, app.processes.selected_signal);
}

fn handle_inspector_key(key: KeyEvent, app: &mut App) -> bool {
    match key.code {
        // Close inspector
        KeyCode::Esc => {
            app.close_service_inspector();
        }

        // Edit service file
        KeyCode::Char('e') => {
            if !app.services.is_sip_protected
                && let Some(ref path) = app.services.file_path {
                    app.editor_request = Some(path.clone());
            }
        }

        // Edit config file
        KeyCode::Char('c') => {
            if let Some(ref path) = app.services.config_path {
                app.editor_request = Some(path.clone());
            }
        }

        // Toggle log view
        KeyCode::Char('l') => {
            match app.services.inspector_mode {
                ServiceInspectorMode::View => {
                    app.load_service_logs();
                    app.services.inspector_mode = ServiceInspectorMode::Logs;
                    app.services.inspector_scroll = 0;
                }
                ServiceInspectorMode::Logs => {
                    app.services.inspector_mode = ServiceInspectorMode::View;
                    app.services.inspector_scroll = 0;
                }
            }
        }

        // Service actions within the inspector
        KeyCode::Enter => {
            crate::widgets::services::run_service_action(app, "start");
        }
        KeyCode::Char('s') => {
            crate::widgets::services::run_service_action(app, "stop");
        }
        KeyCode::Char('r') => {
            crate::widgets::services::run_service_action(app, "restart");
        }

        // Scroll through file contents / logs
        KeyCode::Down | KeyCode::Char('j') => {
            app.services.inspector_scroll = app.services.inspector_scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.services.inspector_scroll = app.services.inspector_scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            app.services.inspector_scroll = app.services.inspector_scroll.saturating_add(20);
        }
        KeyCode::PageUp => {
            app.services.inspector_scroll = app.services.inspector_scroll.saturating_sub(20);
        }
        KeyCode::Home => {
            app.services.inspector_scroll = 0;
        }

        // Quit still works
        KeyCode::Char('q') => return false,
        KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => return false,

        _ => {}
    }
    true
}
