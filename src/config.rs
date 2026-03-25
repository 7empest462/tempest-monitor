use serde::Deserialize;
use std::path::PathBuf;

/// Application configuration loaded from YAML file + CLI overrides.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TempestConfig {
    /// Default tab on startup (1-9)
    pub default_tab: u8,

    /// Refresh rate in milliseconds
    pub refresh_rate_ms: u64,

    /// Whether GPU monitoring is enabled
    pub gpu_enabled: bool,

    /// Color theme: "dark", "catppuccin", "light"
    pub theme: String,

    /// SQLite database path (empty = disabled)
    pub db_path: String,

    /// Data retention in days
    pub retention_days: u32,

    /// Alert rules
    pub alerts: Vec<AlertRuleConfig>,

    /// Remote metrics push URL (empty = disabled)
    pub metrics_push_url: String,

    /// Prometheus metrics port (0 = disabled)
    pub metrics_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AlertRuleConfig {
    /// Metric to monitor: "cpu", "memory", "gpu", "disk", "swap"
    pub metric: String,

    /// Threshold percentage (0-100)
    pub threshold: f64,
    pub cooldown_secs: u64,

    /// Action: "notify", "log", "webhook"
    #[serde(default = "default_action")]
    pub action: String,
}

fn default_action() -> String { "notify".into() }

impl Default for TempestConfig {
    fn default() -> Self {
        Self {
            default_tab: 1,
            refresh_rate_ms: 1200,
            gpu_enabled: true,
            theme: "dark".into(),
            db_path: String::new(),
            retention_days: 7,
            alerts: Vec::new(),
            metrics_push_url: String::new(),
            metrics_port: 0,
        }
    }
}

impl TempestConfig {
    /// Load configuration from the default path or a specified path.
    /// Falls back to defaults if no file exists.
    pub fn load(path: Option<&str>) -> Self {
        let config_path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            dirs_config_path()
        };

        if !config_path.exists() {
            log::info!("No config file at {:?}, using defaults", config_path);
            return Self::default();
        }

        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match serde_yaml::from_str(&contents) {
                Ok(cfg) => {
                    log::info!("Loaded config from {:?}", config_path);
                    cfg
                }
                Err(e) => {
                    log::warn!("Failed to parse config {:?}: {}", config_path, e);
                    Self::default()
                }
            },
            Err(e) => {
                log::warn!("Could not read config {:?}: {}", config_path, e);
                Self::default()
            }
        }
    }

    /// Apply CLI overrides on top of the loaded config.
    pub fn apply_cli(&mut self, cli: &crate::cli::CliArgs) {
        if cli.tab != 1 {
            self.default_tab = cli.tab;
        }
        if cli.rate != 1200 {
            self.refresh_rate_ms = cli.rate;
        }
        if cli.no_gpu {
            self.gpu_enabled = false;
        }
        if let Some(ref db) = cli.db {
            self.db_path = db.clone();
        }
        if let Some(port) = cli.metrics_port {
            self.metrics_port = port;
        }
    }
}

/// Default config directory: ~/.config/tempest-monitor/config.yaml
fn dirs_config_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("tempest-monitor")
            .join("config.yaml")
    } else {
        PathBuf::from("config.yaml")
    }
}
