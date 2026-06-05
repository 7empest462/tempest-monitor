use clap::Parser;

/// 7EMPEST Fleet Monitor — Advanced system monitoring TUI
#[derive(Parser, Debug, Clone)]
#[command(name = "tempest-monitor", version, about, long_about = None)]
pub struct CliArgs {
    /// Initial tab to display (1-10)
    #[arg(
        short,
        long,
        default_value_t = 1,
        value_parser = clap::value_parser!(u8).range(1..=10),
        env = "TEMPEST_TAB"
    )]
    pub tab: u8,

    /// Refresh rate in milliseconds (100ms - 5000ms)
    #[arg(
        short,
        long,
        default_value_t = 1200,
        value_parser = clap::value_parser!(u64).range(100..=5000),
        env = "TEMPEST_RATE"
    )]
    pub rate: u64,

    /// Path to config file (default: ~/.config/tempest-monitor/config.yaml)
    #[arg(short, long, env = "TEMPEST_CONFIG")]
    pub config: Option<String>,

    /// Disable GPU monitoring (skip powermetrics)
    #[arg(long)]
    pub no_gpu: bool,

    /// Export a JSON snapshot and exit (use --export-json <path>)
    #[arg(long, conflicts_with = "export_chart")]
    pub export_json: Option<String>,

    /// Export a high-res PNG chart and exit
    #[arg(long, conflicts_with = "export_json")]
    pub export_chart: Option<String>,

    /// Enable Prometheus metrics endpoint on given port
    #[arg(long, env = "TEMPEST_METRICS_PORT")]
    pub metrics_port: Option<u16>,

    /// SQLite database path for metric history
    #[arg(long, env = "TEMPEST_DB")]
    pub db: Option<String>,

    /// Startup color theme
    #[arg(
        long,
        value_parser = ["dark", "light", "nord", "catppuccin", "dracula", "gruvbox", "tokyo-night"],
        env = "TEMPEST_THEME"
    )]
    pub theme: Option<String>,

    /// Disable desktop notifications
    #[arg(long)]
    pub no_alerts: bool,

    /// Increase log verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl CliArgs {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
