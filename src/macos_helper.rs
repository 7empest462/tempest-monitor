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
    fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buffer: *mut std::ffi::c_void, buffersize: i32) -> i32;
}

#[repr(C)]
#[derive(Default, Debug)]
#[allow(dead_code)]
pub struct proc_taskinfo {
    pub pti_virtual_size: u64,
    pub pti_resident_size: u64,
    pub pti_total_user: u64,
    pub pti_total_system: u64,
    pub pti_threads_user: u64,
    pub pti_threads_system: u64,
    pub pti_policy: i32,
    pub pti_faults: i32,
    pub pti_pageins: i32,
    pub pti_cow_faults: i32,
    pub pti_messages_sent: i32,
    pub pti_messages_received: i32,
    pub pti_syscalls_mach: i32,
    pub pti_syscalls_unix: i32,
    pub pti_csw: i32,
    pub pti_threadnum: i32,
    pub pti_numrunning: i32,
    pub pti_priority: i32,
}

const PROC_PIDTASKINFO: i32 = 4;

const TASK_VM_INFO: i32 = 22;
const TASK_VM_INFO_COUNT: u32 = (mem::size_of::<task_vm_info_compressed>() / 4) as u32;

pub struct ProcessMetadata {
    pub compressed: u64,
    pub thread_count: i32,
    pub priority: i32,
}

#[derive(Clone, Debug, Default)]
pub struct MacOSGpuTelemetry {
    pub usage_pct: f64,
    pub power_mw: Option<f64>,
    pub cpu_power_mw: Option<f64>,
    pub package_power_mw: Option<f64>,
    pub ane_power_mw: Option<f64>,
    pub gpu_freq_mhz: Option<f64>,
    pub model: String,
}

/// Retrieves resident, compressed memory info, thread count and priority for a given PID.
pub fn get_process_metadata(pid: i32) -> Option<ProcessMetadata> {
    let mut task: u32 = 0;
    let self_port = unsafe { mach_task_self() };
    
    // Memory (Compressed)
    let mut compressed: u64 = 0;
    if unsafe { task_for_pid(self_port, pid, &mut task) } == 0 {
        let mut info = task_vm_info_compressed::default();
        let mut count = TASK_VM_INFO_COUNT;
        if unsafe { task_info(task, TASK_VM_INFO, &mut info as *mut _ as *mut i32, &mut count) } == 0 {
            compressed = info.compressed;
        }
        unsafe { mach_port_deallocate(self_port, task) };
    }

    // Threads and Priority
    let mut task_info = proc_taskinfo::default();
    let res = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDTASKINFO,
            0,
            &mut task_info as *mut _ as *mut std::ffi::c_void,
            mem::size_of::<proc_taskinfo>() as i32,
        )
    };

    if res <= 0 {
        if compressed > 0 {
            return Some(ProcessMetadata {
                compressed,
                thread_count: 0,
                priority: 0,
            });
        }
        return None;
    }

    Some(ProcessMetadata {
        compressed,
        thread_count: task_info.pti_threadnum,
        priority: task_info.pti_priority,
    })
}

pub fn get_ioreg_gpu_usage() -> f64 {
    let output = std::process::Command::new("ioreg")
        .args(["-r", "-c", "AGXAccelerator"])
        .output()
        .ok();
        
    if let Some(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        // Matches: "Device Utilization %" = 67 OR "Device Utilization %"=67 OR Device Utilization %=67
        // Hardened to be case-insensitive and handle optional surrounding quotes
        let re_str = r#"(?i)"?Device Utilization %"?\s*[=:]\s*(\d+)"#;
        if let Ok(re) = regex::Regex::new(re_str) {
            if let Some(caps) = re.captures(&s) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().parse::<f64>().unwrap_or(0.0);
                }
            }
        }
    }
    0.0
}

/// Retrieves Apple Silicon GPU usage and power from powermetrics or IOKit fallbacks.
/// `allow_prompt` should be true for interactive TUI use and false for background library use.
pub fn get_macos_gpu_info(allow_prompt: bool) -> MacOSGpuTelemetry {
    let mut tel = MacOSGpuTelemetry {
        model: get_soc_model(),
        ..Default::default()
    };

    // 1. Definitively get usage % from IOKit (no sudo required, very reliable on Apple Silicon)
    tel.usage_pct = get_ioreg_gpu_usage();

    let is_root = unsafe { libc::getuid() } == 0;
    
    let run_powermetrics = |use_sudo: bool, non_interactive: bool| -> Option<String> {
        let mut cmd = if use_sudo {
            let mut c = std::process::Command::new("sudo");
            if non_interactive { c.arg("-n"); }
            c.arg("powermetrics");
            c
        } else {
            std::process::Command::new("powermetrics")
        };

        cmd.args(&["-n", "1", "-i", "200", "--samplers", "gpu_power,cpu_power,thermal"]);
        
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                return Some(String::from_utf8_lossy(&output.stdout).to_string());
            }
        }
        None
    };

    let output = if is_root {
        run_powermetrics(false, false)
    } else {
        // Try non-interactive sudo first (uses cached credentials)
        run_powermetrics(true, true).or_else(|| {
            // If failed and caller allows it, try interactive sudo (may prompt for password)
            if allow_prompt {
                run_powermetrics(true, false)
            } else {
                None
            }
        })
    };

    if let Some(stdout) = output {
        for line in stdout.lines() {
            let low = line.to_lowercase();
            
            // 1. GPU Residency (Usage %)
            if low.contains("gpu") && low.contains("active residency") {
                if let Some(val) = line.split(':').nth(1) {
                    let clean_val = val.split('(').next().unwrap_or(val).trim();
                    let residency: f64 = clean_val.trim_end_matches('%').parse().unwrap_or(0.0);
                    if residency > 0.0 || tel.usage_pct == 0.0 {
                        tel.usage_pct = residency;
                    }
                }
            }

            // 2. GPU Frequency
            if low.contains("gpu hw active frequency") {
                if let Some(val) = line.split(':').last() {
                    tel.gpu_freq_mhz = val.to_lowercase().replace("mhz", "").trim().parse().ok();
                }
            }

            // 3. Power Parsing (more robust case-insensitive check)
            fn parse_mw(line: &str) -> Option<f64> {
                line.split(':').last()?.to_lowercase()
                    .replace("mw", "")
                    .trim()
                    .parse().ok()
            }

            if low.contains("gpu power") {
                tel.power_mw = parse_mw(line);
            } else if low.contains("cpu power") {
                tel.cpu_power_mw = parse_mw(line);
            } else if low.contains("ane power") {
                tel.ane_power_mw = parse_mw(line);
            } else if low.contains("package power") || low.contains("combined power") {
                tel.package_power_mw = parse_mw(line);
            }
        }
    }

    tel
}

/// Dynamic SoC detection for Apple Silicon (M1, M2, M3, M4 etc.)
pub fn get_soc_model() -> String {
    use std::ptr;
    use libc::{sysctlbyname, c_char, c_void, size_t};

    let mut size: size_t = 0;
    let name = "machdep.cpu.brand_string\0";
    unsafe {
        // Get size first
        if sysctlbyname(name.as_ptr() as *const c_char, ptr::null_mut(), &mut size, ptr::null_mut(), 0) != 0 {
            return "Apple Silicon".to_string();
        }
        let mut buf = vec![0u8; size];
        if sysctlbyname(name.as_ptr() as *const c_char, buf.as_mut_ptr() as *mut c_void, &mut size, ptr::null_mut(), 0) != 0 {
            return "Apple Silicon".to_string();
        }
        let s = String::from_utf8_lossy(&buf).trim_matches(char::from(0)).trim().to_string();
        if s.is_empty() { "Apple Silicon".to_string() } else { s }
    }
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
