use std::mem;

#[repr(C)]
#[derive(Default, Debug)]
#[allow(dead_code)]
struct task_vm_info {
    pub virtual_size: u64,
    pub region_count: i32,
    pub page_size: i32,
    pub resident_size: u64,
    pub resident_size_peak: u64,
    pub device: u64,
    pub device_peak: u64,
    pub internal: u64,
    pub internal_peak: u64,
    pub external: u64,
    pub external_peak: u64,
    pub reusable: u64,
    pub reusable_peak: u64,
    pub purgeable_volatile_size: u64,
    pub purgeable_volatile_clean_size: u64,
    pub purgeable_volatile_compressed_size: u64,
}

#[repr(C)]
#[derive(Default, Debug)]
#[allow(dead_code)]
struct task_vm_info_compressed {
    pub base: task_vm_info,
    pub compressed: u64,
    pub compressed_peak: u64,
    pub compressed_lifetime: u64,
}

unsafe extern "C" {
    fn mach_task_self() -> u32;
    fn task_for_pid(target_tport: u32, pid: i32, tn: *mut u32) -> i32;
    fn task_info(target_task: u32, flavor: i32, task_info_out: *mut i32, task_info_outCnt: *mut u32) -> i32;
    fn mach_port_deallocate(task: u32, name: u32) -> i32;
}

const TASK_VM_INFO: i32 = 22;
const TASK_VM_INFO_COUNT: u32 = (mem::size_of::<task_vm_info_compressed>() / 4) as u32;

pub struct ProcessMemoryInfo {
    pub compressed: u64,
}

#[derive(Clone, Debug, Default)]
pub struct MacOSGpuTelemetry {
    pub usage_pct: f64,
    pub power_mw: Option<f64>,
    pub package_power_mw: Option<f64>,
    pub model: String,
}

/// Retrieves resident and compressed memory info for a given PID using Mach task_info.
/// Requires elevated permissions (sudo) for processes not owned by the current user.
pub fn get_process_memory_info(pid: i32) -> Option<ProcessMemoryInfo> {
    let mut task: u32 = 0;
    let self_port = unsafe { mach_task_self() };
    
    let res = unsafe { task_for_pid(self_port, pid, &mut task) };
    if res != 0 {
        return None;
    }

    let mut info = task_vm_info_compressed::default();
    let mut count = TASK_VM_INFO_COUNT;
    let res = unsafe {
        task_info(task, TASK_VM_INFO, &mut info as *mut _ as *mut i32, &mut count)
    };

    // Clean up the port
    unsafe { mach_port_deallocate(self_port, task) };

    if res != 0 {
        return None;
    }

    Some(ProcessMemoryInfo {
        compressed: info.compressed,
    })
}

/// Retrieves Apple Silicon GPU usage and power from powermetrics or IOKit fallbacks.
pub fn get_macos_gpu_info() -> MacOSGpuTelemetry {
    let mut tel = MacOSGpuTelemetry {
        model: "Apple Silicon".to_string(), // Default for modern Macs
        ..Default::default()
    };

    let is_root = unsafe { libc::getuid() } == 0;
    
    // Attempt to run powermetrics. If not root, use 'sudo -n' to try cached credentials
    // without ever prompting the user for a password (safe for library use).
    let mut cmd = if is_root {
        let mut c = std::process::Command::new("powermetrics");
        c.args(&["-n", "1", "-i", "200", "--samplers", "gpu_power,cpu_power"]);
        c
    } else {
        let mut c = std::process::Command::new("sudo");
        c.args(&["-n", "powermetrics", "-n", "1", "-i", "200", "--samplers", "gpu_power,cpu_power"]);
        c
    };

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let low = line.to_lowercase();
                if low.contains("gpu active residency") {
                    if let Some(val) = line.split(':').last() {
                        tel.usage_pct = val.trim().trim_end_matches('%').parse().unwrap_or(0.0);
                    }
                }
                if low.contains("gpu power") {
                    if let Some(val) = line.split(':').last() {
                        tel.power_mw = val.trim().trim_end_matches("mw").trim().parse().ok();
                    }
                }
                if low.contains("package power") || low.contains("combined power") {
                    if let Some(val) = line.split(':').last() {
                        tel.package_power_mw = val.trim().trim_end_matches("mw").trim().parse().ok();
                    }
                }
            }
        } else {
            // sudo -n failed (no cached credentials), return -1.0 to indicate N/A
            tel.usage_pct = -1.0;
        }
    } else {
        tel.usage_pct = -1.0;
    }

    tel
}

/// Decodes cryptic macOS SMC sensor keys into human-readable labels.
/// Common keys for Apple Silicon (M1/M2/M3) and Intel Macs included.
pub fn decode_smc_label(label: &str) -> String {
    match label {
        "TG0D" | "TG1D" => "GPU Die".to_string(),
        "TG0P" | "TG1P" => "GPU Proximity".to_string(),
        "Tp0P" | "Tp0c" => "SOC Die".to_string(),
        "TA0P" => "Airflow Proximity".to_string(),
        "TB0T" => "Battery".to_string(),
        "Ts0P" | "Ts0S" => "Palm Rest / Case".to_string(),
        "TN0P" | "TN0D" => "NAND (Storage)".to_string(),
        "Tm0P" | "Tm0D" => "Mainboard".to_string(),
        "TC0D" | "TC0c" | "TC0P" => "CPU Die/Proximity".to_string(),
        _ => label.to_string(),
    }
}
