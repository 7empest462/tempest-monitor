#![cfg(target_os = "linux")]

use ethtool::{new_connection, EthtoolAttr, EthtoolLinkModeAttr, EthtoolLinkModeDuplex};
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{TemperatureSensor, Clock};
use futures::stream::TryStreamExt;

pub struct LinuxInterfaceInfo {
    pub speed: Option<u32>,
    pub duplex: Option<String>,
    pub driver: Option<String>,
}

pub struct NvidiaGpuInfo {
    pub name: String,
    pub temperature: u32,
    pub memory_used_pct: f64,
    pub fan_speed_pct: u32,
    pub graphics_clock_mhz: u32,
    pub memory_clock_mhz: u32,
    pub power_usage_mw: u32,
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
#[allow(dead_code)]
pub fn get_amd_gpu_clock() -> Option<u32> {
    for n in 0..4u32 {
        let path = format!("/sys/class/drm/card{}/device/pp_dpm_sclk", n);
        if let Ok(content) = std::fs::read_to_string(&path) {
            // Find the active clock line (marked with *)
            for line in content.lines() {
                if line.contains('*') {
                    // Format: "N: NNNMhz *"
                    if let Some(mhz_str) = line.split("Mhz").next() {
                        let num_str = mhz_str.split_whitespace().last().unwrap_or("0");
                        if let Ok(mhz) = num_str.parse::<u32>() {
                            return Some(mhz);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Get AMD GPU temperature from hwmon
#[allow(dead_code)]
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
#[allow(dead_code)]
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
