use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

#[cfg(feature = "database")]
use tempest_monitor::db;
use tempest_monitor::{
    App,
    app::ActiveTab,
    cli::CliArgs,
    config::TempestConfig,
    ui,
    input,
    alerts,
};
#[cfg(any(feature = "metrics", feature = "export"))]
use tempest_monitor::export;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI args first (before entering raw mode, so --help works)
    let cli = CliArgs::parse_args();

    // Initialize logging based on verbosity
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(log_level),
    )
    .format_timestamp_millis()
    .init();

    log::info!("Starting Tempest Monitor v{}", env!("CARGO_PKG_VERSION"));

    // On Windows the console defaults to a legacy code page that can't render
    // Unicode block characters (▁▂▃▄▅▆▇█) — switch to UTF-8 before entering raw mode.
    #[cfg(windows)]
    unsafe {
        windows::Win32::System::Console::SetConsoleOutputCP(65001);
    }

    // Load config (file + CLI overrides)
    let mut cfg = TempestConfig::load(cli.config.as_deref());
    cfg.apply_cli(&cli);

    // Set dynamic theme
    tempest_monitor::theme::set_theme(&cfg.theme);

    log::debug!("Config: {:?}", cfg);

    // Initialize Database
    #[cfg(feature = "database")]
    let db = {
        let db = db::Database::new().await?;
        let db = std::sync::Arc::new(db);
        // Startup pruning (7 days)
        let _ = db.prune_old_data(7).await;
        db
    };

    // Initialize Metrics Export
    #[cfg(feature = "metrics")]
    if cfg.metrics_port > 0 {
        export::init_prometheus(cfg.metrics_port);
    }

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_with_config(&cli, &cfg, cli.config.clone());
    
    // Handle One-shot Exports
    if let Some(ref _path) = cli.export_json {
        app.refresh();
        #[cfg(any(feature = "metrics", feature = "export"))]
        {
            let data = export::export_json(&app);
            std::fs::write(_path, data)?;
            println!("JSON snapshot exported to {}", _path);
        }
        #[cfg(not(any(feature = "metrics", feature = "export")))]
        println!("JSON export requires 'metrics' or 'export' feature.");
        
        return Ok(());
    }

    #[cfg(feature = "export")]
    if let Some(ref path) = cli.export_chart {
        app.refresh();
        export::export_chart_png(&app, path)?;
        println!("Performance chart exported to {}", path);
        return Ok(());
    }

    let res = cfg_select! {
        feature = "database" => {
            run_app(&mut terminal, app, db).await
        },
        _ => {
            run_app_no_db(&mut terminal, app).await
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e:?}");
    }

    Ok(())
}

#[cfg(feature = "database")]
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    db: std::sync::Arc<db::Database>,
) -> io::Result<()> {
    let mut last_tick = std::time::Instant::now();
    let mut last_save = std::time::Instant::now();
    let save_interval = std::time::Duration::from_secs(60); // Save metrics every minute
    let mut alert_engine = alerts::AlertEngine::new();
    let mut last_tab = app.active_tab;

    loop {
        // Load history snapshots immediately on tab switch
        if app.active_tab == ActiveTab::History && (last_tab != ActiveTab::History || app.history.snapshots.is_empty()) {
            if let Ok(snaps) = db.get_recent_snapshots(50).await {
                app.history.snapshots = snaps;
                if app.history.selected >= app.history.snapshots.len() && !app.history.snapshots.is_empty() {
                    app.history.selected = app.history.snapshots.len() - 1;
                }
            }
            last_tab = app.active_tab;
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = app.tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)?
            && let event::Event::Key(key) = event::read()?
            && !input::handle_key(key, &mut app) {
                break;
        }

        // Handle editor request (suspend TUI, spawn editor, resume TUI)
        if let Some(path) = app.editor_request.take() {
            spawn_editor(terminal, &path)?;
        }

        if last_tick.elapsed() >= app.tick_rate {
            app.refresh();
            app.update_focus_history();
            
            // Periodic history refresh
            if app.active_tab == ActiveTab::History
                && let Ok(snaps) = db.get_recent_snapshots(50).await
            {
                app.history.snapshots = snaps;
                if app.history.selected >= app.history.snapshots.len() && !app.history.snapshots.is_empty() {
                    app.history.selected = app.history.snapshots.len() - 1;
                }
            }

            // Phase 13: Alerting
            alert_engine.check_rules(&app, &app.config.alerts);

            // Phase 14: Metrics Export
            #[cfg(feature = "metrics")]
            {
                export::update_metrics(&app);
                if !app.config.metrics_push_url.is_empty() {
                    let url = app.config.metrics_push_url.clone();
                    let data = export::export_json(&app);
                    tokio::spawn(async move {
                        let client = reqwest::Client::new();
                        let _ = client.post(url)
                            .header("Content-Type", "application/json")
                            .body(data)
                            .send()
                            .await;
                    });
                }
            }

            last_tick = std::time::Instant::now();

            // Periodic persistence
            #[cfg(feature = "database")]
            if last_save.elapsed() >= save_interval {
                let db_clone = db.clone();
                let cpu = app.cpu_history.iter().copied().last().unwrap_or(0) as f64;
                let mem = app.sys.used_memory() as f64 / 1_073_741_824.0;
                let gpu = app.gpu_usage;
                let rx = app.net_rx_history.iter().copied().last().unwrap_or(0) as f64 / 1024.0;
                let tx = app.net_tx_history.iter().copied().last().unwrap_or(0) as f64 / 1024.0;

                tokio::spawn(async move {
                    let _ = db_clone.save_snapshot(cpu, mem, gpu, rx, tx).await;
                });

                last_save = std::time::Instant::now();
            }
        }
    }
    Ok(())
}

#[cfg(not(feature = "database"))]
async fn run_app_no_db(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> io::Result<()> {
    let mut last_tick = std::time::Instant::now();
    let mut alert_engine = alerts::AlertEngine::new();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = app.tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)?
            && let event::Event::Key(key) = event::read()?
            && !input::handle_key(key, &mut app) {
                break;
        }

        // Handle editor request (suspend TUI, spawn editor, resume TUI)
        if let Some(path) = app.editor_request.take() {
            spawn_editor(terminal, &path)?;
        }

        if last_tick.elapsed() >= app.tick_rate {
            app.refresh();
            app.update_focus_history();
            
            // Phase 13: Alerting
            alert_engine.check_rules(&app, &app.config.alerts);

            last_tick = std::time::Instant::now();
        }
    }
    Ok(())
}

/// Suspend the TUI, spawn an editor for the given file, and resume the TUI.
fn spawn_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    path: &str,
) -> io::Result<()> {
    let editor = tempest_monitor::service_inspector::get_editor();

    // 1. Leave the alternate screen and disable raw mode
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // 2. Spawn the editor and wait for it to finish
    let status = std::process::Command::new(&editor)
        .arg(path)
        .status();

    if let Err(e) = &status {
        eprintln!("Failed to open editor '{}': {}", editor, e);
        eprintln!("Set $EDITOR to your preferred editor.");
        eprintln!("Press Enter to continue...");
        let _ = io::stdin().read_line(&mut String::new());
    }

    // 3. Re-enter the TUI
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    terminal.clear()?;

    Ok(())
}
