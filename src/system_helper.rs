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
