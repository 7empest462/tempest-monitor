//! # Tempest Monitor Library
//! 
//! This library provides the core hardware telemetry and monitoring logic used by
//! the Tempest Monitor TUI application.

#[cfg(target_os = "macos")]
pub mod macos_helper;
#[cfg(target_os = "linux")]
pub mod linux_helper;
pub mod system_helper;
pub mod telemetry_core;
pub mod process_helper;

pub mod config;
pub mod error;
pub mod app;
#[cfg(feature = "database")]
pub mod db;
pub mod alerts;
pub mod input;
pub mod ui;
pub mod theme;
#[cfg(any(feature = "metrics", feature = "export"))]
pub mod export;
pub mod cli;
pub mod widgets;

pub use app::App;
pub use config::TempestConfig;
pub use telemetry_core::TelemetrySnapshot;
