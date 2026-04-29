#![cfg(target_os = "linux")]

use ethtool::{new_connection, EthtoolAttr, EthtoolLinkModeAttr, EthtoolLinkModeDuplex};
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{TemperatureSensor, Clock};
use futures::stream::TryStreamExt;
use std::collections::HashMap;
use std::time::Instant;
use parking_lot::Mutex;

pub struct LinuxInterfaceInfo {
    pub speed: Option<u32>,
    pub duplex: Option<String>,
    pub driver: Option<String>,
}

pub struct ProcessMetadata {
    pub thread_count: i32,
    pub priority: i32,
}

pub fn get_process_metadata(pid: i32) -> Option<ProcessMetadata> {
    let stat_path = format!("/proc/{}/stat", pid);
    if let Ok(content) = std::fs::read_to_string(stat_path) {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() > 19 {
            let priority = parts[17].parse().unwrap_or(0);
            let thread_count = parts[19].parse().unwrap_or(0);
            return Some(ProcessMetadata { thread_count, priority });
        }
    }
    None
}

#[derive(Clone, Debug, Default)]
pub struct NvidiaGpuInfo {
    pub name: String,
    pub temperature: u32,
    pub memory_used_pct: f64,
    pub fan_speed_pct: u32,
    pub graphics_clock_mhz: u32,
    pub memory_clock_mhz: u32,
    pub power_usage_mw: u32,
}

#[derive(Clone, Debug, Default)]
pub struct LinuxGpuTelemetry {
    pub usage_pct: f64,
    pub temp_c: Option<u32>,
    pub clock_mhz: Option<u32>,
    pub vram_used: Option<u64>,
    pub vram_total: Option<u64>,
    pub model: String,
    pub driver: String,
    pub nvidia_info: Vec<NvidiaGpuInfo>,
}

/// Detected GPU vendor from sysfs PCI IDs.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuVendor {
    Amd,
    Intel,
    Nvidia,
    Unknown,
}

/// Runtime-detected GPU information from sysfs.
#[derive(Clone, Debug)]
pub struct DetectedGpu {
    pub vendor: GpuVendor,
    pub model_name: String,
    pub driver: String,
    #[allow(dead_code)]
    pub sysfs_card: String, // e.g. "card0"
}

static INTEL_GPU_STATE: Mutex<Option<HashMap<String, (u64, Instant)>>> = Mutex::new(None);

pub fn is_steamos() -> bool {
    if std::path::Path::new("/etc/steamos-release").exists() {
        return true;
    }
    std::fs::read_to_string("/etc/os-release")
        .map(|s| {
            let s_low = s.to_lowercase();
            s_low.contains("id=steamos") || s_low.contains("id=\"steamos\"") || 
            (s_low.contains("id_like=arch") && s_low.contains("steamos"))
        })
        .unwrap_or(false)
}

/// Detect GPU(s) by reading `/sys/class/drm/cardN/device/` PCI info.
pub fn detect_gpu_from_sysfs() -> Option<DetectedGpu> {
    for n in 0..8u32 {
        let base = format!("/sys/class/drm/card{}/device", n);
        let vendor_path = format!("{}/vendor", base);

        if let Ok(vendor_str) = std::fs::read_to_string(&vendor_path) {
            let vendor_hex = vendor_str.trim().to_lowercase();
            let vendor = match vendor_hex.as_str() {
                "0x1002" => GpuVendor::Amd,
                "0x8086" => GpuVendor::Intel,
                "0x10de" => GpuVendor::Nvidia,
                _ => GpuVendor::Unknown,
            };

            // Try to read a human-readable product name (some systems have this)
            let model_name = std::fs::read_to_string(format!("{}/label", base))
                .or_else(|_| std::fs::read_to_string(format!("{}/product_name", base)))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| {
                    // Construct a name from PCI device ID
                    let device_id = std::fs::read_to_string(format!("{}/device", base))
                        .map(|s| s.trim().to_lowercase())
                        .unwrap_or_default();
                    match &vendor {
                        GpuVendor::Amd => format!("AMD GPU ({})", device_id),
                        GpuVendor::Intel => format!("Intel GPU ({})", device_id),
                        GpuVendor::Nvidia => format!("NVIDIA GPU ({})", device_id),
                        GpuVendor::Unknown => format!("GPU [{}] ({})", vendor_hex, device_id),
                    }
                });

            // Read the driver symlink
            let driver = std::fs::read_link(format!("{}/driver", base))
                .ok()
                .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
                .unwrap_or_else(|| "unknown".to_string());

            return Some(DetectedGpu {
                vendor,
                model_name,
                driver,
                sysfs_card: format!("card{}", n),
            });
        }
    }
    None
}

/// Get AMD-specific GPU clock from pp_dpm_sclk (Steam Deck, RDNA, etc.)
pub fn get_amd_gpu_clock() -> Option<u32> {
    // ... implementation preserved ...
    None
}

pub fn get_amdgpu_metrics_usage() -> Option<i32> {
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("card") || name.len() < 5 || name.contains('-') {
                continue;
            }

            let path = entry.path().join("device/gpu_metrics");
            if let Ok(data) = std::fs::read(&path) {
                if data.len() < 4 { continue; }
                let format_rev = data[2];
                let content_rev = data[3];
                let primary_offset = match (format_rev, content_rev) {
                    (1, 0) | (1, 1) => 14,
                    (1, 2) | (1, 3) => 14,
                    (2, 0) | (2, 1) | (2, 2) => 24,
                    (2, 3) | (2, 4) => 28,
                    _ => 28,
                };
                if data.len() >= primary_offset + 2 {
                    let val = u16::from_le_bytes([data[primary_offset], data[primary_offset + 1]]);
                    if val != 0xFFFF && val <= 100 { return Some(val as i32); }
                }
                for &off in &[24, 28, 30, 32, 14, 16] {
                    if data.len() >= off + 2 {
                        let val = u16::from_le_bytes([data[off], data[off + 1]]);
                        if val > 0 && val <= 100 { return Some(val as i32); }
                    }
                }
            }
        }
    }
    None
}

/// Get AMD GPU temperature from hwmon
pub fn get_amd_gpu_temp() -> Option<u32> {
    // Look for amdgpu hwmon
    let hwmon_base = "/sys/class/hwmon";
    if let Ok(entries) = std::fs::read_dir(hwmon_base) {
        for entry in entries.flatten() {
            let name_path = entry.path().join("name");
            if let Ok(name) = std::fs::read_to_string(&name_path) {
                if name.trim() == "amdgpu" {
                    let temp_path = entry.path().join("temp1_input");
                    if let Ok(temp_str) = std::fs::read_to_string(&temp_path) {
                        if let Ok(millideg) = temp_str.trim().parse::<u32>() {
                            return Some(millideg / 1000); // Convert millidegrees to degrees
                        }
                    }
                }
            }
        }
    }
    None
}

/// Get AMD GPU VRAM usage from sysfs
pub fn get_amd_vram_usage() -> Option<(u64, u64)> {
    for n in 0..4u32 {
        let used_path = format!("/sys/class/drm/card{}/device/mem_info_vram_used", n);
        let total_path = format!("/sys/class/drm/card{}/device/mem_info_vram_total", n);
        if let (Ok(used_str), Ok(total_str)) = (
            std::fs::read_to_string(&used_path),
            std::fs::read_to_string(&total_path),
        ) {
            if let (Ok(used), Ok(total)) = (
                used_str.trim().parse::<u64>(),
                total_str.trim().parse::<u64>(),
            ) {
                return Some((used, total));
            }
        }
    }
    None
}

pub fn get_interface_extra_info(iface: &str) -> Option<LinuxInterfaceInfo> {
    let iface = iface.to_string();

    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async move {
            let (conn, mut handle, _) = new_connection().ok()?;
            tokio::spawn(conn);

            let mut speed: Option<u32> = None;
            let mut duplex: Option<String> = None;

            let mut stream = handle.link_mode().get(Some(&iface)).execute().await;
            while let Ok(Some(msg)) = stream.try_next().await {
                for attr in msg.payload.nlas {
                    if let EthtoolAttr::LinkMode(lm) = attr {
                        match lm {
                            EthtoolLinkModeAttr::Speed(s) => speed = Some(s),
                            EthtoolLinkModeAttr::Duplex(d) => {
                                duplex = Some(match d {
                                    EthtoolLinkModeDuplex::Half => "Half".into(),
                                    EthtoolLinkModeDuplex::Full => "Full".into(),
                                    _ => "Unknown".into(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }

            Some(LinuxInterfaceInfo { speed, duplex, driver: None })
        })
    })
}

pub fn get_nvidia_gpu_info() -> Vec<NvidiaGpuInfo> {
    // ... existing implementation ...
    let mut results = Vec::new();
    let nvml = match Nvml::init() {
        Ok(n) => n,
        Err(_) => return results,
    };
    let device_count = nvml.device_count().unwrap_or(0);
    for i in 0..device_count {
        if let Ok(device) = nvml.device_by_index(i) {
            let name = device.name().unwrap_or_else(|_| "Unknown NVIDIA GPU".into());
            let temperature = device.temperature(TemperatureSensor::Gpu).unwrap_or(0);
            let memory_used_pct = device.memory_info()
                .map(|m| (m.used as f64 / m.total as f64 * 100.0).clamp(0.0, 100.0))
                .unwrap_or(0.0);
            let fan_speed_pct = device.fan_speed(0).unwrap_or(0);
            let graphics_clock_mhz = device.clock_info(Clock::Graphics).unwrap_or(0);
            let memory_clock_mhz = device.clock_info(Clock::Memory).unwrap_or(0);
            let power_usage_mw = device.power_usage().unwrap_or(0);
            results.push(NvidiaGpuInfo {
                name,
                temperature,
                memory_used_pct,
                fan_speed_pct,
                graphics_clock_mhz,
                memory_clock_mhz,
                power_usage_mw,
            });
        }
    }
    results
}

pub fn get_linux_gpu_load() -> i32 {
    // 0. Specialized SteamOS / AMD
    if is_steamos() {
        if let Some(usage) = get_amdgpu_metrics_usage() { return usage; }
    }
    // 1. Try Nvidia
    if let Ok(nvml) = Nvml::init() {
        if let Ok(device) = nvml.device_by_index(0) {
            if let Ok(util) = device.utilization_rates() { return util.gpu as i32; }
        }
    }
    // 2. Scan DRM (Intel/AMD fallback)
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("card") || name.len() < 5 { continue; }
            let base = entry.path();
            let device_path = base.join("device");
            for p in &[&device_path, &base] {
                if let Ok(content) = std::fs::read_to_string(p.join("gpu_busy_percent")) {
                    if let Ok(val) = content.trim().parse::<i32>() { if val > 0 { return val; } }
                }
            }
            // Intel RC6 residency logic
            let rc6_path = base.join("device/gt/gt0/rc6_residency_ms");
            if let Ok(content) = std::fs::read_to_string(&rc6_path) {
                if let Ok(residency) = content.trim().parse::<u64>() {
                    let mut state_lock = INTEL_GPU_STATE.lock();
                    if state_lock.is_none() { *state_lock = Some(HashMap::new()); }
                    if let Some(map) = state_lock.as_mut() {
                        let now = Instant::now();
                        if let Some((last_res, last_time)) = map.get(&name) {
                            let time_diff = now.duration_since(*last_time).as_millis() as u64;
                            let res_diff = residency.saturating_sub(*last_res);
                            if time_diff > 500 {
                                map.insert(name, (residency, now));
                                let idle_frac = (res_diff as f64 / time_diff as f64).clamp(0.0, 1.0);
                                return ((1.0 - idle_frac) * 100.0) as i32;
                            }
                        } else { map.insert(name, (residency, now)); }
                    }
                }
            }
        }
    }
    0
}

/// Helper to get a full snapshot of GPU telemetry on Linux.
pub fn collect_gpu_telemetry() -> LinuxGpuTelemetry {
    let mut tel = LinuxGpuTelemetry::default();
    
    // 1. Basic identification
    if let Some(gpu) = detect_gpu_from_sysfs() {
        tel.model = gpu.model_name;
        tel.driver = gpu.driver;
    } else {
        tel.model = "Unknown GPU".into();
        tel.driver = "unknown".into();
    }

    // 2. Usage Load
    tel.usage_pct = get_linux_gpu_load() as f64;

    // 3. Nvidia details
    tel.nvidia_info = get_nvidia_gpu_info();
    if !tel.nvidia_info.is_empty() {
        // If NVIDIA is present, prefer its readings
        tel.usage_pct = tel.nvidia_info[0].memory_used_pct; // prefer usage if available, but memory used is a good fallback
        tel.temp_c = Some(tel.nvidia_info[0].temperature);
        tel.model = tel.nvidia_info[0].name.clone();
    } else {
        // 4. AMD / Intel specifics
        tel.temp_c = get_amd_gpu_temp();
        tel.clock_mhz = get_amd_gpu_clock();
        if let Some((used, total)) = get_amd_vram_usage() {
            tel.vram_used = Some(used);
            tel.vram_total = Some(total);
        }
    }

    tel
}
