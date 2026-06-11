// No longer needed
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};

// ── Service entry (from launchctl / systemctl) ───────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ServiceEntry {
    pub pid: Option<i32>,
    pub status: i32,
    pub label: String,
}

// ── Socket entry (active TCP/UDP connections) ────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct SocketEntry {
    pub proto: String,
    pub local_addr: String,
    pub foreign_addr: String,
    pub state: String,
    pub pid: Option<i32>,
    pub process_name: String,
}

#[cfg(target_os = "macos")]
pub fn get_services() -> Vec<ServiceEntry> {
    let mut entries = Vec::new();
    if let Ok(output) = std::process::Command::new("launchctl").arg("list").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        entries = stdout
            .lines()
            .skip(1)
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(3, '\t').collect();
                if parts.len() == 3 {
                    let pid = parts[0].trim().parse::<i32>().ok().filter(|&p| p > 0);
                    let status = parts[1].trim().parse::<i32>().unwrap_or(0);
                    let label = parts[2].trim().to_string();
                    if !label.is_empty() {
                        return Some(ServiceEntry { pid, status, label });
                    }
                }
                None
            })
            .collect();
        entries.sort_by(|a, b| a.label.cmp(&b.label));
    }
    entries
}

#[cfg(target_os = "linux")]
pub fn get_services() -> Vec<ServiceEntry> {
    let try_systemctl = |extra_args: &[&str]| -> Vec<ServiceEntry> {
        let mut args = vec!["list-units", "--type=service", "--all", "--no-pager", "--no-legend", "--plain"];
        args.extend_from_slice(extra_args);
        
        let output = match std::process::Command::new("systemctl")
            .args(&args)
            .output()
        {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let label = parts[0].to_string();
                    let active = parts[2];
                    let sub = parts[3];
                    let pid = if sub == "running" { Some(1) } else { None };
                    let status = if active == "active" { 0 } else { -1 };
                    Some(ServiceEntry { pid, status, label })
                } else {
                    None
                }
            })
            .collect()
    };

    let mut entries = try_systemctl(&[]);
    if entries.is_empty() {
        entries = try_systemctl(&["--user"]);
    }
    entries.sort_by(|a, b| a.label.cmp(&b.label));
    entries
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn get_services() -> Vec<ServiceEntry> {
    Vec::new()
}


pub fn get_sockets(sys: &sysinfo::System) -> Vec<SocketEntry> {
    let mut entries = Vec::new();
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    if let Ok(sockets) = get_sockets_info(af_flags, proto_flags) {
        for info in sockets {
            let (proto, local_addr, foreign_addr, state, pid) = match info.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => (
                    if tcp.local_addr.is_ipv4() { "tcp4" } else { "tcp6" }.to_string(),
                    format!("{}:{}", tcp.local_addr, tcp.local_port),
                    format!("{}:{}", tcp.remote_addr, tcp.remote_port),
                    format!("{:?}", tcp.state).to_uppercase(),
                    info.associated_pids.first().copied().map(|p| p as i32),
                ),
                ProtocolSocketInfo::Udp(udp) => (
                    if udp.local_addr.is_ipv4() { "udp4" } else { "udp6" }.to_string(),
                    format!("{}:{}", udp.local_addr, udp.local_port),
                    "*:*".to_string(),
                    "NONE".to_string(),
                    info.associated_pids.first().copied().map(|p| p as i32),
                ),
            };

            let process_name = if let Some(p) = pid {
                sys.process(sysinfo::Pid::from(p as usize))
                    .map(|pr| pr.name().to_string_lossy().to_string())
                    .unwrap_or_else(|| "-".to_string())
            } else {
                "-".to_string()
            };

            if state != "CLOSED" {
                entries.push(SocketEntry {
                    proto,
                    local_addr,
                    foreign_addr,
                    state,
                    pid,
                    process_name,
                });
            }
        }
    }
    entries
}

// ── Stacked memory segments ──────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct MemorySegments {
    pub total: u64,
    pub active: u64,
    pub wired: u64,
    pub cache: u64,
    pub free: u64,
}

pub fn get_fallback_segments(sys: &sysinfo::System) -> MemorySegments {
    let total = sys.total_memory();
    let used = sys.used_memory();
    let avail = sys.available_memory();
    let free = total.saturating_sub(used);
    let cache = avail.saturating_sub(free);
    let active = used.saturating_sub(cache);

    MemorySegments {
        total,
        active,
        wired: 0,
        cache,
        free,
    }
}

#[cfg(target_os = "macos")]
pub fn get_memory_segments(sys: &sysinfo::System) -> MemorySegments {
    use std::collections::HashMap;

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;

    let try_parse = || -> Option<MemorySegments> {
        let output = std::process::Command::new("vm_stat").output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut stats = HashMap::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().to_string();
                let val_str = parts[1].trim().trim_end_matches('.');
                if let Ok(val) = val_str.parse::<u64>() {
                    stats.insert(key, val);
                }
            }
        }

        let active_pages = stats.get("Pages active").copied().unwrap_or(0);
        let wired_pages = stats.get("Pages wired down").copied().unwrap_or(0);
        let inactive_pages = stats.get("Pages inactive").copied().unwrap_or(0);
        let spec_pages = stats.get("Pages speculative").copied().unwrap_or(0);
        let free_pages = stats.get("Pages free").copied().unwrap_or(0);
        let comp_pages = stats.get("Pages occupied by compressor").copied().unwrap_or(0);

        let total = sys.total_memory();
        let active = active_pages.saturating_add(comp_pages).saturating_mul(page_size);
        let wired = wired_pages.saturating_mul(page_size);
        let cache = inactive_pages.saturating_add(spec_pages).saturating_mul(page_size);
        let free = free_pages.saturating_mul(page_size);

        Some(MemorySegments {
            total,
            active,
            wired,
            cache,
            free,
        })
    };

    try_parse().unwrap_or_else(|| get_fallback_segments(sys))
}

#[cfg(target_os = "linux")]
pub fn get_memory_segments(sys: &sysinfo::System) -> MemorySegments {
    let try_parse = || -> Option<MemorySegments> {
        let content = std::fs::read_to_string("/proc/meminfo").ok()?;
        let mut mem_total = None;
        let mut mem_free = None;
        let mut active = None;
        let mut buffers = None;
        let mut cached = None;
        let mut sreclaimable = None;
        let mut shmem = None;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = parts[0].trim_end_matches(':');
                if let Ok(val_kb) = parts[1].parse::<u64>() {
                    let val_bytes = val_kb * 1024;
                    match key {
                        "MemTotal" => mem_total = Some(val_bytes),
                        "MemFree" => mem_free = Some(val_bytes),
                        "Active" => active = Some(val_bytes),
                        "Buffers" => buffers = Some(val_bytes),
                        "Cached" => cached = Some(val_bytes),
                        "SReclaimable" => sreclaimable = Some(val_bytes),
                        "Shmem" => shmem = Some(val_bytes),
                        _ => {}
                    }
                }
            }
        }

        let total = mem_total.unwrap_or_else(|| sys.total_memory());
        let free = mem_free.unwrap_or(0);
        let act = active.unwrap_or(0);
        let buf = buffers.unwrap_or(0);
        
        let c = cached.unwrap_or(0)
            .saturating_add(sreclaimable.unwrap_or(0))
            .saturating_sub(shmem.unwrap_or(0));

        Some(MemorySegments {
            total,
            active: act,
            wired: buf,
            cache: c,
            free,
        })
    };

    try_parse().unwrap_or_else(|| get_fallback_segments(sys))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn get_memory_segments(sys: &sysinfo::System) -> MemorySegments {
    get_fallback_segments(sys)
}

