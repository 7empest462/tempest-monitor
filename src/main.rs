mod alerts;
mod cli;
mod config;
mod db;
mod error;
mod export;
mod input;
#[cfg(target_os = "linux")]
mod linux_helper;
#[cfg(target_os = "macos")]
mod macos_helper;
mod theme;
mod ui;
mod widgets;
mod app;

use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;
use crate::cli::CliArgs;
use crate::config::TempestConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Manual early-parse for --headless and --pid-file to ensure silence ASAP
    let args: Vec<String> = std::env::args().collect();
    let is_headless = args.iter().any(|a| a == "--headless");
    
    // Write PID file as early as possible (before any TTY-sensitive crates init)
    if let Some(pos) = args.iter().position(|a| a == "--pid-file") {
        if let Some(pid_path) = args.get(pos + 1) {
            let _ = std::fs::write(pid_path, std::process::id().to_string());
        }
    }

    // 2. If headless, redirect stdout/stderr to /dev/null to prevent SIGTTOU on macOS
    if is_headless {
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::io::AsRawFd;
            let dev_null = std::fs::File::open("/dev/null")?;
            let null_fd = dev_null.as_raw_fd();
            unsafe {
                libc::dup2(null_fd, libc::STDOUT_FILENO);
                libc::dup2(null_fd, libc::STDERR_FILENO);
            }
        }
    }

    // 3. Initialize the real async runtime manually
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = CliArgs::parse_args();

    // Initialize logging (v0.3.3 log-file support)
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(log_level),
    );
    builder.format_timestamp_millis();

    if let Some(ref log_path) = cli.log_file {
        // Support ~ expansion for log file (Phase 16)
        let expanded_path = if log_path.starts_with("~/") {
            dirs::home_dir()
                .map(|p| p.join(&log_path[2..]))
                .unwrap_or_else(|| std::path::PathBuf::from(log_path))
        } else {
            std::path::PathBuf::from(log_path)
        };
        let file = std::fs::File::create(expanded_path)?;
        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }
    builder.init();

    log::info!("Starting Tempest Monitor v{}", env!("CARGO_PKG_VERSION"));
    
    // Load config
    let mut cfg = TempestConfig::load(cli.config.as_deref());
    cfg.apply_cli(&cli);

    // Initialize Database
    let db = db::Database::new().await?;
    let db = std::sync::Arc::new(db);
    let _ = db.prune_old_data(7).await;

    // Initialize Metrics Export
    if cfg.metrics_port > 0 {
        export::init_prometheus(cfg.metrics_port);
    }

    let mut app = App::new_with_config(&cli, &cfg);

    // Handle One-shot Exports
    if let Some(ref path) = cli.export_json {
        app.refresh();
        let data = export::export_json(&app);
        std::fs::write(path, data)?;
        return Ok(());
    }

    if let Some(ref path) = cli.export_chart {
        app.refresh();
        export::export_chart_png(&app, path)?;
        return Ok(());
    }

    // Branch: Headless Mode vs TUI Mode
    if cli.headless {
        log::info!("Running in HEADLESS mode (Daemon). Alerts and metrics export are active.");
        run_headless(app, db).await?;
    } else {
        // Terminal setup
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = run_app(&mut terminal, app, db).await;

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
    }

    Ok(())
}

async fn run_headless(
    mut app: App,
    db: std::sync::Arc<db::Database>,
) -> io::Result<()> {
    let mut last_tick = std::time::Instant::now();
    let mut last_save = std::time::Instant::now();
    let save_interval = std::time::Duration::from_secs(60);
    let mut alert_engine = alerts::AlertEngine::new();

    loop {
        // In headless mode, we just sleep until the next tick
        tokio::time::sleep(app.tick_rate).await;

        app.refresh();
        
        // Alerting
        alert_engine.check_rules(&app, &app.config.alerts);

        // Metrics Export
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

        // Periodic persistence
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

        last_tick = std::time::Instant::now();
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    db: std::sync::Arc<db::Database>,
) -> io::Result<()> {
    let mut last_tick = std::time::Instant::now();
    let mut last_save = std::time::Instant::now();
    let save_interval = std::time::Duration::from_secs(60); // Save metrics every minute
    let mut alert_engine = alerts::AlertEngine::new();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = app.tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            if let event::Event::Key(key) = event::read()? {
                if !input::handle_key(key, &mut app) {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= app.tick_rate {
            app.refresh();
            app.update_focus_history();
            
            // Phase 13: Alerting
            alert_engine.check_rules(&app, &app.config.alerts);

            // Phase 14: Metrics Export
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

            last_tick = std::time::Instant::now();

            // Periodic persistence
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
