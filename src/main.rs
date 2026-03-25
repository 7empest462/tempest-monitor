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

    // Load config (file + CLI overrides)
    let mut cfg = TempestConfig::load(cli.config.as_deref());
    cfg.apply_cli(&cli);

    log::debug!("Config: {:?}", cfg);

    // Initialize Database
    let db = db::Database::new().await?;
    let db = std::sync::Arc::new(db);

    // Startup pruning (7 days)
    let _ = db.prune_old_data(7).await;

    // 4. Initialize Metrics Export (Phase 14)
    if cfg.metrics_port > 0 {
        export::init_prometheus(cfg.metrics_port);
    }

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_with_config(&cli, &cfg);
    
    // 5. Handle One-shot Exports (Phase 15)
    if let Some(ref path) = cli.export_json {
        app.refresh();
        let data = export::export_json(&app);
        std::fs::write(path, data)?;
        println!("JSON snapshot exported to {}", path);
        return Ok(());
    }

    if let Some(ref path) = cli.export_chart {
        app.refresh();
        export::export_chart_png(&app, path)?;
        println!("Performance chart exported to {}", path);
        return Ok(());
    }

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

    Ok(())
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
