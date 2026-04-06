use serde::{Serialize, Deserialize};
// No longer needed

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetrySnapshot {
    pub cpu_usage_avg: f64,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub gpu_usage_pct: f64,
    pub gpu_model: String,
    pub battery_pct: Option<f64>,
    pub battery_state: Option<String>,
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub power_usage_mw: Option<f64>,
}

#[cfg(target_os = "macos")]
pub fn get_macos_telemetry() -> TelemetrySnapshot {
    // Logic extracted from app.rs for macOS
    // For now, returning a subset. We will refine this.
    TelemetrySnapshot {
        cpu_usage_avg: 0.0, // sysinfo required
        ram_used_gb: 0.0,
        ram_total_gb: 0.0,
        gpu_usage_pct: 0.0,
        gpu_model: "Apple M4".into(),
        battery_pct: None,
        battery_state: None,
        cpu_temp_c: None,
        gpu_temp_c: None,
        power_usage_mw: None,
    }
}
