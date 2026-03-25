#![cfg(target_os = "linux")]

use procfs::process::Process;
use ethtool::{new_connection, EthtoolAttr, EthtoolLinkModeAttr, EthtoolDriverAttr};
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{TemperatureSensor, Clock};
use futures::StreamExt;

pub struct LinuxProcessInfo {
    pub fd_count: usize,
    pub thread_count: i64,
    pub cgroup: String,
}

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

pub fn get_process_extra_info(pid: i32) -> Option<LinuxProcessInfo> {
    let proc = Process::new(pid).ok()?;
    let fd_count = proc.fd_count().unwrap_or(0);
    let stat = proc.stat().ok()?;
    let cgroup = std::fs::read_to_string(format!("/proc/{}/cgroup", pid)).unwrap_or_default();

    Some(LinuxProcessInfo {
        fd_count,
        thread_count: stat.num_threads,
        cgroup,
    })
}

pub fn get_interface_extra_info(iface: &str) -> Option<LinuxInterfaceInfo> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    let iface = iface.to_string();

    rt.block_on(async move {
        let (conn, mut handle, _) = new_connection().ok()?;
        tokio::spawn(conn);

        let mut speed: Option<u32> = None;
        let mut duplex: Option<String> = None;
        let mut driver: Option<String> = None;

        // --- Link mode (speed + duplex) ---
        let mut stream = handle.link_mode().get(Some(&iface)).execute();
        while let Some(Ok(msg)) = stream.next().await {
            for attr in msg.payload.attributes {
                if let EthtoolAttr::LinkMode(lm_attrs) = attr {
                    for lm in lm_attrs {
                        match lm {
                            EthtoolLinkModeAttr::Speed(s) => speed = Some(s),
                            EthtoolLinkModeAttr::Duplex(d) => {
                                duplex = Some(match d {
                                    0 => "Half".into(),
                                    1 => "Full".into(),
                                    _ => "Unknown".into(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // --- Driver info ---
        let mut stream = handle.driver_info().get(Some(&iface)).execute();
        while let Some(Ok(msg)) = stream.next().await {
            for attr in msg.payload.attributes {
                if let EthtoolAttr::Driver(drv_attrs) = attr {
                    for drv in drv_attrs {
                        if let EthtoolDriverAttr::Driver(name) = drv {
                            driver = Some(name);
                        }
                    }
                }
            }
        }

        Some(LinuxInterfaceInfo { speed, duplex, driver })
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
