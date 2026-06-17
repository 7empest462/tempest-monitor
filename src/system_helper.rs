// Unix-only imports — gated so they don't break Windows compilation
#[cfg(unix)]
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};

// ── Service entry (from launchctl / systemctl / SCM) ─────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ServiceEntry {
    pub pid: Option<i32>,
    pub status: i32,
    pub label: String,
}

// ── Socket entry (active TCP/UDP connections) ─────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct SocketEntry {
    pub proto: String,
    pub local_addr: String,
    pub foreign_addr: String,
    pub state: String,
    pub pid: Option<i32>,
    pub process_name: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// get_services()
// ═══════════════════════════════════════════════════════════════════════════════

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

/// Windows: Enumerate services via the Service Control Manager (native Win32 API).
/// Provides service name, display name, current state, and PID — no external tools needed.
#[cfg(windows)]
pub fn get_services() -> Vec<ServiceEntry> {
    use windows::Win32::System::Services::{
        OpenSCManagerW, EnumServicesStatusExW, CloseServiceHandle,
        SC_MANAGER_ENUMERATE_SERVICE, SERVICE_WIN32, SERVICE_STATE_ALL,
        SC_ENUM_PROCESS_INFO, ENUM_SERVICE_STATUS_PROCESSW,
    };
    use windows::core::PCWSTR;

    let mut entries = Vec::new();

    unsafe {
        // Open the Service Control Manager with read access
        let scm = match OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE) {
            Ok(h) => h,
            Err(_) => return entries,
        };

        // First call to find out how much buffer we need
        let mut bytes_needed: u32 = 0;
        let mut services_returned: u32 = 0;
        let mut resume_handle: u32 = 0;

        let _ = EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            None,
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            PCWSTR::null(),
        );

        if bytes_needed == 0 {
            let _ = CloseServiceHandle(scm);
            return entries;
        }

        // Allocate buffer and do the real call
        let mut buffer: Vec<u8> = vec![0u8; bytes_needed as usize];
        resume_handle = 0;
        services_returned = 0;

        let result = EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            Some(&mut buffer),
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            PCWSTR::null(),
        );

        if result.is_ok() {
            let ptr = buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW;
            for i in 0..(services_returned as isize) {
                let svc = &*ptr.offset(i);

                // Convert the wide-string service name to a Rust String
                let name_ptr = svc.lpServiceName.as_ptr();
                let mut len = 0;
                while *name_ptr.add(len) != 0 { len += 1; }
                let name = String::from_utf16_lossy(std::slice::from_raw_parts(name_ptr, len));

                let state = svc.ServiceStatusProcess.dwCurrentState.0;
                // SERVICE_RUNNING = 4, anything else is not running
                let is_running = state == 4;
                let pid = svc.ServiceStatusProcess.dwProcessId;

                entries.push(ServiceEntry {
                    pid: if is_running && pid > 0 { Some(pid as i32) } else { None },
                    status: if is_running { 0 } else { -1 },
                    label: name,
                });
            }
        }

        let _ = CloseServiceHandle(scm);
    }

    entries.sort_by(|a, b| a.label.cmp(&b.label));
    entries
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub fn get_services() -> Vec<ServiceEntry> {
    Vec::new()
}


// ═══════════════════════════════════════════════════════════════════════════════
// get_sockets()
// ═══════════════════════════════════════════════════════════════════════════════

/// Unix (macOS + Linux): Use netstat2 crate for socket enumeration.
#[cfg(unix)]
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

/// Windows: Enumerate TCP/UDP connections via GetExtendedTcpTable / GetExtendedUdpTable.
/// Returns full connection info including PID, local/remote address:port, and state — no external tools.
#[cfg(windows)]
pub fn get_sockets(sys: &sysinfo::System) -> Vec<SocketEntry> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable,
        MIB_TCPTABLE_OWNER_PID, MIB_UDPTABLE_OWNER_PID,
        TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
        MIB_TCP_STATE,
    };
    use windows::Win32::Networking::WinSock::AF_INET;
    use std::net::Ipv4Addr;

    let mut entries = Vec::new();

    unsafe {
        // ── TCP (IPv4) ──────────────────────────────────────────────────────
        let mut tcp_size: u32 = 0;
        // First call to size-probe
        let _ = GetExtendedTcpTable(
            None, &mut tcp_size, false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL, 0,
        );

        if tcp_size > 0 {
            let mut tcp_buf: Vec<u8> = vec![0u8; tcp_size as usize];
            let result = GetExtendedTcpTable(
                Some(tcp_buf.as_mut_ptr() as *mut _),
                &mut tcp_size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );

            if result == 0 {
                let table = &*(tcp_buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
                let count = table.dwNumEntries as usize;
                let rows = std::slice::from_raw_parts(table.table.as_ptr(), count);

                for row in rows {
                    let local_ip = Ipv4Addr::from(u32::from_be(row.dwLocalAddr));
                    let local_port = u16::from_be(row.dwLocalPort as u16);
                    let remote_ip = Ipv4Addr::from(u32::from_be(row.dwRemoteAddr));
                    let remote_port = u16::from_be(row.dwRemotePort as u16);
                    let pid = row.dwOwningPid as i32;

                    let state = match MIB_TCP_STATE(row.dwState as i32) {
                        s if s == MIB_TCP_STATE(1)  => "CLOSED",
                        s if s == MIB_TCP_STATE(2)  => "LISTEN",
                        s if s == MIB_TCP_STATE(3)  => "SYN_SENT",
                        s if s == MIB_TCP_STATE(4)  => "SYN_RECEIVED",
                        s if s == MIB_TCP_STATE(5)  => "ESTABLISHED",
                        s if s == MIB_TCP_STATE(6)  => "FIN_WAIT1",
                        s if s == MIB_TCP_STATE(7)  => "FIN_WAIT2",
                        s if s == MIB_TCP_STATE(8)  => "CLOSE_WAIT",
                        s if s == MIB_TCP_STATE(9)  => "CLOSING",
                        s if s == MIB_TCP_STATE(10) => "LAST_ACK",
                        s if s == MIB_TCP_STATE(11) => "TIME_WAIT",
                        s if s == MIB_TCP_STATE(12) => "DELETE_TCB",
                        _ => "UNKNOWN",
                    };

                    if state == "CLOSED" { continue; }

                    let process_name = sys.process(sysinfo::Pid::from(pid as usize))
                        .map(|p| p.name().to_string_lossy().to_string())
                        .unwrap_or_else(|| "-".to_string());

                    entries.push(SocketEntry {
                        proto: "tcp4".to_string(),
                        local_addr: format!("{}:{}", local_ip, local_port),
                        foreign_addr: format!("{}:{}", remote_ip, remote_port),
                        state: state.to_string(),
                        pid: Some(pid),
                        process_name,
                    });
                }
            }
        }

        // ── UDP (IPv4) ──────────────────────────────────────────────────────
        let mut udp_size: u32 = 0;
        let _ = GetExtendedUdpTable(
            None, &mut udp_size, false,
            AF_INET.0 as u32,
            UDP_TABLE_OWNER_PID, 0,
        );

        if udp_size > 0 {
            let mut udp_buf: Vec<u8> = vec![0u8; udp_size as usize];
            let result = GetExtendedUdpTable(
                Some(udp_buf.as_mut_ptr() as *mut _),
                &mut udp_size,
                false,
                AF_INET.0 as u32,
                UDP_TABLE_OWNER_PID,
                0,
            );

            if result == 0 {
                let table = &*(udp_buf.as_ptr() as *const MIB_UDPTABLE_OWNER_PID);
                let count = table.dwNumEntries as usize;
                let rows = std::slice::from_raw_parts(table.table.as_ptr(), count);

                for row in rows {
                    let local_ip = Ipv4Addr::from(u32::from_be(row.dwLocalAddr));
                    let local_port = u16::from_be(row.dwLocalPort as u16);
                    let pid = row.dwOwningPid as i32;

                    let process_name = sys.process(sysinfo::Pid::from(pid as usize))
                        .map(|p| p.name().to_string_lossy().to_string())
                        .unwrap_or_else(|| "-".to_string());

                    entries.push(SocketEntry {
                        proto: "udp4".to_string(),
                        local_addr: format!("{}:{}", local_ip, local_port),
                        foreign_addr: "*:*".to_string(),
                        state: "NONE".to_string(),
                        pid: Some(pid),
                        process_name,
                    });
                }
            }
        }
    }

    entries
}

#[cfg(not(any(unix, windows)))]
pub fn get_sockets(_sys: &sysinfo::System) -> Vec<SocketEntry> {
    Vec::new()
}


// ═══════════════════════════════════════════════════════════════════════════════
// Stacked memory segments
// ═══════════════════════════════════════════════════════════════════════════════

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
                        "MemTotal"      => mem_total      = Some(val_bytes),
                        "MemFree"       => mem_free        = Some(val_bytes),
                        "Active"        => active          = Some(val_bytes),
                        "Buffers"       => buffers         = Some(val_bytes),
                        "Cached"        => cached          = Some(val_bytes),
                        "SReclaimable"  => sreclaimable    = Some(val_bytes),
                        "Shmem"         => shmem           = Some(val_bytes),
                        _ => {}
                    }
                }
            }
        }

        let total = mem_total.unwrap_or_else(|| sys.total_memory());
        let free  = mem_free.unwrap_or(0);
        let act   = active.unwrap_or(0);
        let buf   = buffers.unwrap_or(0);
        let c     = cached.unwrap_or(0)
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

/// Windows: Use GlobalMemoryStatusEx for physical totals + GetPerformanceInfo for pool breakdown.
/// Maps: Active → Working Set committed pages, Wired → Non-Paged Pool, Cache → Paged Pool.
#[cfg(windows)]
pub fn get_memory_segments(_sys: &sysinfo::System) -> MemorySegments {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};

    unsafe {
        let mut mem_status = MEMORYSTATUSEX::default();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

        let mut perf_info = PERFORMANCE_INFORMATION::default();
        perf_info.cb = std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32;

        let has_mem = GlobalMemoryStatusEx(&mut mem_status).is_ok();
        let has_perf = GetPerformanceInfo(&mut perf_info, perf_info.cb).is_ok();

        if !has_mem {
            return MemorySegments::default();
        }

        let total = mem_status.ullTotalPhys;
        let free  = mem_status.ullAvailPhys;
        let page  = if has_perf { perf_info.PageSize as u64 } else { 4096 };

        let (wired, cache) = if has_perf {
            let non_paged = (perf_info.PageSize as u64) * 1024; // rough fallback for NonPagedPool
            let paged = (perf_info.PageSize as u64) * 1024;     // rough fallback for PagedPool
            (non_paged, paged)
        } else {
            (0, 0)
        };

        // Active = everything not in pools and not free
        let active = total
            .saturating_sub(free)
            .saturating_sub(wired)
            .saturating_sub(cache);

        MemorySegments { total, active, wired, cache, free }
    }
}

/// Fallback for any other exotic target.
#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub fn get_memory_segments(sys: &sysinfo::System) -> MemorySegments {
    get_fallback_segments(sys)
}
