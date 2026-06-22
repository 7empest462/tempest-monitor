//! # Tempest Monitor Library
//!
//! This library provides the core hardware telemetry and monitoring logic used by
//! the Tempest Monitor TUI application.

#[cfg(target_os = "linux")]
pub mod linux_helper;
#[cfg(target_os = "macos")]
pub mod macos_helper;
pub mod platform;
pub mod process_helper;
pub mod system_helper;
pub mod telemetry_core;

pub mod alerts;
pub mod app;
pub mod cli;
pub mod config;
#[cfg(feature = "database")]
pub mod db;
pub mod error;
#[cfg(any(feature = "metrics", feature = "export"))]
pub mod export;
pub mod input;
pub mod power_mode;
pub mod service_inspector;
pub mod theme;
pub mod ui;
pub mod widgets;

pub use app::App;
pub use config::TempestConfig;
pub use telemetry_core::TelemetrySnapshot;

#[cfg(target_os = "linux")]
pub use linux_helper::{LinuxGpuTelemetry, collect_gpu_telemetry as collect_linux_gpu};
#[cfg(target_os = "macos")]
pub use macos_helper::{MacOSGpuTelemetry, get_macos_gpu_info as collect_macos_gpu};
