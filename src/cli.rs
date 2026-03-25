use clap::Parser;

/// 7EMPEST Fleet Monitor — Advanced system monitoring TUI
#[derive(Parser, Debug, Clone)]
#[command(name = "tempest-monitor", version, about, long_about = None)]
pub struct CliArgs {
    /// Initial tab to display (1-9)
    #[arg(short, long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=9))]
    pub tab: u8,

    /// Refresh rate in milliseconds
    #[arg(short, long, default_value_t = 1200)]
    pub rate: u64,

    /// Path to config file (default: ~/.config/tempest-monitor/config.yaml)
    #[arg(short, long)]
    pub config: Option<String>,

    /// Disable GPU monitoring (skip powermetrics)
    #[arg(long)]
    pub no_gpu: bool,

    /// Export a JSON snapshot and exit (use --export-json <path>)
    #[arg(long)]
    pub export_json: Option<String>,

    /// Export a high-res PNG chart and exit
    #[arg(long)]
    pub export_chart: Option<String>,

    /// Enable Prometheus metrics endpoint on given port
    #[arg(long)]
    pub metrics_port: Option<u16>,

    /// SQLite database path for metric history
    #[arg(long)]
    pub db: Option<String>,

    /// Disable desktop notifications
    #[arg(long)]
    pub no_alerts: bool,

    /// Increase log verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    
    /// Run in headless (daemon) mode without TUI
    #[arg(long)]
    pub headless: bool,

    /// Write logs to a specific file
    #[arg(long)]
    pub log_file: Option<String>,

    /// Write process ID to a specific file
    #[arg(long)]
    pub pid_file: Option<String>,
}

impl CliArgs {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
