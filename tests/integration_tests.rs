use clap::Parser;
use ratatui::style::Color;
use tempest_monitor::alerts::AlertEngine;
use tempest_monitor::app::App;
use tempest_monitor::cli::CliArgs;
use tempest_monitor::config::{AlertRuleConfig, TempestConfig};
use tempest_monitor::theme;

#[test]
fn test_config_defaults_and_serialization() {
    let config = TempestConfig::default();
    assert_eq!(config.default_tab, 1);
    assert_eq!(config.refresh_rate_ms, 1200);
    assert_eq!(config.theme, "dark");
    assert!(config.gpu_enabled);

    // Save to temp file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("tempest_test_config.yaml");
    let temp_path_str = temp_path.to_str().unwrap();

    let save_result = config.save(Some(temp_path_str));
    assert!(save_result.is_ok());

    let loaded = TempestConfig::load(Some(temp_path_str));
    assert_eq!(loaded.default_tab, 1);
    assert_eq!(loaded.refresh_rate_ms, 1200);
    assert_eq!(loaded.theme, "dark");

    // Clean up
    let _ = std::fs::remove_file(temp_path);
}

#[test]
fn test_cli_parsing() {
    // Valid parses
    let args = CliArgs::try_parse_from(["tempest-monitor", "--tab", "5", "--rate", "2000"]);
    assert!(args.is_ok());
    let parsed = args.unwrap();
    assert_eq!(parsed.tab, 5);
    assert_eq!(parsed.rate, 2000);

    // Invalid tab (out of range)
    let args = CliArgs::try_parse_from(["tempest-monitor", "--tab", "11"]);
    assert!(args.is_err());

    // Invalid rate (below 100)
    let args = CliArgs::try_parse_from(["tempest-monitor", "--rate", "50"]);
    assert!(args.is_err());

    // Invalid rate (above 5000)
    let args = CliArgs::try_parse_from(["tempest-monitor", "--rate", "6000"]);
    assert!(args.is_err());

    // Invalid theme
    let args = CliArgs::try_parse_from(["tempest-monitor", "--theme", "invalid_theme"]);
    assert!(args.is_err());

    // Conflicts: export-json and export-chart
    let args = CliArgs::try_parse_from([
        "tempest-monitor",
        "--export-json",
        "out.json",
        "--export-chart",
        "out.png",
    ]);
    assert!(args.is_err());
}

#[test]
fn test_cli_overrides() {
    let mut config = TempestConfig::default();
    let cli = CliArgs::try_parse_from([
        "tempest-monitor",
        "--tab",
        "8",
        "--rate",
        "3000",
        "--theme",
        "nord",
    ])
    .unwrap();

    config.apply_cli(&cli);
    assert_eq!(config.default_tab, 8);
    assert_eq!(config.refresh_rate_ms, 3000);
    assert_eq!(config.theme, "nord");
}

#[test]
fn test_theme_and_lerp() {
    theme::set_theme("dark");
    // At 0.0%, usage_color should return exactly the first element of gradient, which is Green: Rgb(166, 227, 161)
    let color_0 = theme::usage_color(0.0);
    assert_eq!(color_0, Color::Rgb(166, 227, 161));

    // At 100.0%, usage_color should return exactly the last element of gradient, which is Red: Rgb(243, 139, 168)
    let color_100 = theme::usage_color(100.0);
    assert_eq!(color_100, Color::Rgb(243, 139, 168));

    // Test other theme set
    theme::set_theme("nord");
    let color_0_nord = theme::usage_color(0.0);
    assert_eq!(color_0_nord, Color::Rgb(163, 190, 140)); // Nord green
}

#[test]
fn test_alert_engine() {
    let cli = CliArgs::try_parse_from(["tempest-monitor"]).unwrap();
    let config = TempestConfig::default();
    let mut app = App::new_with_config(&cli, &config, None);

    // Mock CPU history (threshold is checked against last element)
    app.cpu_history.push_back(95);

    let mut engine = AlertEngine::new();
    let rules = vec![AlertRuleConfig {
        metric: "cpu".into(),
        threshold: 90.0,
        cooldown_secs: 60,
        action: "log".into(),
    }];

    // Trigger alert without issues
    engine.check_rules(&app, &rules);
}
