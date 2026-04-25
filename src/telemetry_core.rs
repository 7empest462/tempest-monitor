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
    pub cpu_power_usage_mw: Option<f64>,
    pub ane_power_usage_mw: Option<f64>,
    pub gpu_freq_mhz: Option<f64>,
}

#[cfg(target_os = "macos")]
pub fn get_macos_telemetry() -> TelemetrySnapshot {
    let gpu = crate::macos_helper::get_macos_gpu_info(false);
    
    TelemetrySnapshot {
        cpu_usage_avg: 0.0, // sysinfo required
        ram_used_gb: 0.0,
        ram_total_gb: 0.0,
        gpu_usage_pct: gpu.usage_pct,
        gpu_model: gpu.model,
        battery_pct: None,
        battery_state: None,
        cpu_temp_c: None,
        gpu_temp_c: None,
        power_usage_mw: gpu.power_mw,
        cpu_power_usage_mw: gpu.cpu_power_mw,
        ane_power_usage_mw: gpu.ane_power_mw,
        gpu_freq_mhz: gpu.gpu_freq_mhz,
    }
}

#[cfg(target_os = "linux")]
pub fn get_linux_telemetry() -> TelemetrySnapshot {
    let gpu = crate::linux_helper::collect_gpu_telemetry();
    
    TelemetrySnapshot {
        cpu_usage_avg: 0.0, // sysinfo required
        ram_used_gb: 0.0,
        ram_total_gb: 0.0,
        gpu_usage_pct: gpu.usage_pct,
        gpu_model: gpu.model,
        battery_pct: None,
        battery_state: None,
        cpu_temp_c: None,
        gpu_temp_c: gpu.temp_c.map(|t| t as f32),
        power_usage_mw: None, // power capped on linux sysfs often
        cpu_power_usage_mw: None,
        ane_power_usage_mw: None,
        gpu_freq_mhz: None,
    }
}
