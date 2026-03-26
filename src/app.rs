use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind,
    RefreshKind, System,
};

use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
// use pnet::datalink;
use tui_textarea::TextArea;

use crate::cli::CliArgs;
use crate::config::TempestConfig;

// ── History buffer size (number of sparkline data points) ────────────────────
pub const HISTORY_LEN: usize = 120;



// ── Tabs ─────────────────────────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ActiveTab {
    Overview,
    Cpu,
    Memory,
    Disks,
    Network,
    Processes,
    Gpu,
    Services,
    Sockets,
}

impl ActiveTab {
    pub const ALL: [ActiveTab; 9] = [
        ActiveTab::Overview,
        ActiveTab::Cpu,
        ActiveTab::Memory,
        ActiveTab::Disks,
        ActiveTab::Network,
        ActiveTab::Processes,
        ActiveTab::Gpu,
        ActiveTab::Services,
        ActiveTab::Sockets,
    ];

    pub fn index(self) -> usize {
        match self {
            ActiveTab::Overview  => 0,
            ActiveTab::Cpu      => 1,
            ActiveTab::Memory   => 2,
            ActiveTab::Disks    => 3,
            ActiveTab::Network  => 4,
            ActiveTab::Processes => 5,
            ActiveTab::Gpu      => 6,
            ActiveTab::Services => 7,
            ActiveTab::Sockets  => 8,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ActiveTab::Overview  => "Overvw",
            ActiveTab::Cpu      => "CPU",
            ActiveTab::Memory   => "Mem",
            ActiveTab::Disks    => "Disk",
            ActiveTab::Network  => "Net",
            ActiveTab::Processes => "Proc",
            ActiveTab::Gpu      => "GPU",
            ActiveTab::Services => "Svc",
            ActiveTab::Sockets  => "Socks",
        }
    }
}

// ── Process sorting ──────────────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SortMode {
    Cpu,
    Memory,      // Resident (physical)
    Virt,        // Virtual (imprint)
    Pid,
    Name,
    DiskIo,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            SortMode::Cpu => "CPU",
            SortMode::Memory => "MEM",
            SortMode::Pid => "PID",
            SortMode::Name => "NAME",
            SortMode::DiskIo => "DISK",
            SortMode::Virt => "VIRT",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

// ── Process view mode ────────────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ProcessViewMode {
    List,
    Tree,
}

// ── Signals ──────────────────────────────────────────────────────────────────

#[derive(Copy, Clone)]
pub struct SignalInfo {
    pub name: &'static str,
    pub number: i32,
}

pub const SIGNALS: [SignalInfo; 7] = [
    SignalInfo { name: "SIGTERM", number: libc::SIGTERM },
    SignalInfo { name: "SIGKILL", number: libc::SIGKILL },
    SignalInfo { name: "SIGSTOP", number: libc::SIGSTOP },
    SignalInfo { name: "SIGCONT", number: libc::SIGCONT },
    SignalInfo { name: "SIGHUP",  number: libc::SIGHUP },
    SignalInfo { name: "SIGUSR1", number: libc::SIGUSR1 },
    SignalInfo { name: "SIGUSR2", number: libc::SIGUSR2 },
];

// ── Battery snapshot ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BatteryInfo {
    pub percent: f64,
    pub state: String,
    pub time_remaining: Option<Duration>,
}

// ── Service entry (from launchctl / systemctl) ───────────────────────────────

#[derive(Clone)]
pub struct ServiceEntry {
    pub pid: Option<i32>,
    pub status: i32,
    pub label: String,
}

// ── Socket entry (active TCP/UDP connections) ────────────────────────────────

#[derive(Clone)]
pub struct SocketEntry {
    pub proto: String,
    pub local_addr: String,
    pub foreign_addr: String,
    pub state: String,
    pub pid: Option<i32>,
    pub process_name: String,
}

#[derive(Clone)]
pub struct NetworkInterfaceInfo {
    pub mac: String,
    pub mtu: u32,
    pub speed: Option<u32>,   // Mbps
    pub duplex: Option<String>,
    pub driver: Option<String>,
}

#[cfg(target_os = "linux")]
pub use crate::linux_helper::NvidiaGpuInfo;

// ── Main App state ───────────────────────────────────────────────────────────

pub struct App {
    pub config: TempestConfig,

    // sysinfo collectors
    pub sys: System,
    pub networks: Networks,
    pub disks: Disks,
    pub components: Components,
    pub load_avg: (f64, f64, f64),
    pub gpu_model: String,
    #[allow(dead_code)]
    pub gpu_driver: String,
    #[allow(dead_code)]
    pub gpu_vendor: String,  // "AMD", "Intel", "NVIDIA", "Apple", "Unknown"
    pub gpu_usage: f64,
    #[allow(dead_code)]
    pub gpu_power_mw: Option<f64>,   // milliwatts from powermetrics (macOS)
    #[allow(dead_code)]
    pub cpu_power_mw: Option<f64>,
    pub pkg_power_mw: Option<f64>,

    // Linux GPU stats (from sysfs / hwmon)
    #[cfg(target_os = "linux")]
    pub nvidia_gpus: Vec<NvidiaGpuInfo>,
    #[cfg(target_os = "linux")]
    pub gpu_temp: Option<u32>,        // degrees C
    #[cfg(target_os = "linux")]
    pub gpu_clock_mhz: Option<u32>,
    #[cfg(target_os = "linux")]
    pub gpu_vram_used: Option<u64>,   // bytes
    #[cfg(target_os = "linux")]
    pub gpu_vram_total: Option<u64>,  // bytes

    // Network Enrichment
    pub network_info: HashMap<String, NetworkInterfaceInfo>,

    // Battery
    pub battery_manager: Option<battery::Manager>,
    pub battery_info: Option<BatteryInfo>,

    // UI state
    pub active_tab: ActiveTab,
    pub show_help: bool,
    pub paused: bool,

    // History buffers for sparklines
    pub cpu_history: VecDeque<u64>,          // overall CPU % (0–100)
    pub per_core_history: Vec<VecDeque<u64>>, // per-core
    pub ram_history: VecDeque<u64>,           // RAM % (0–100)
    pub swap_history: VecDeque<u64>,          // SWAP % (0–100)
    pub net_rx_history: VecDeque<u64>,        // bytes/s received (total)
    pub net_tx_history: VecDeque<u64>,        // bytes/s transmitted (total)
    pub gpu_history: VecDeque<u64>,           // GPU % (0-100)

    // Processes
    pub sort_mode: SortMode,
    pub sort_direction: SortDirection,
    pub process_view: ProcessViewMode,
    pub filter_text_area: TextArea<'static>,
    pub filter_active: bool,
    pub filter_regex: bool,
    pub selected: usize,
    pub show_detail_panel: bool,

    // Signal menu
    pub signal_menu_open: bool,
    pub selected_signal: usize,

    // Services (Tab 8)
    pub services: Vec<ServiceEntry>,
    pub service_selected: usize,
    pub service_action_pending: Option<String>, // feedback message

    // Network sockets (Tab 9)
    pub sockets: Vec<SocketEntry>,
    pub socket_selected: usize,

    // Process Focus mode (Enter on a process)
    pub focus_pid: Option<sysinfo::Pid>,
    pub focus_cpu_history: VecDeque<u64>,
    pub focus_mem_history: VecDeque<u64>,

    #[cfg(target_os = "macos")]
    pub compressed_mem_cache: HashMap<sysinfo::Pid, u64>,

    // Timing
    pub tick_rate: Duration,
    pub last_update: Instant,

    // Last refresh timestamps for throttling
    pub last_process_refresh: Instant,
    pub last_disk_refresh: Instant,
    pub last_gpu_refresh: Instant,
    pub last_service_refresh: Instant,
}

impl App {
    pub fn new_with_config(_cli: &CliArgs, config: &TempestConfig) -> Self {
        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything());

        let sys = System::new_with_specifics(refresh_kind);
        let num_cpus = sys.cpus().len();

        let battery_manager = battery::Manager::new().ok();
        let battery_info = battery_manager.as_ref().and_then(|mgr| {
            mgr.batteries().ok().and_then(|mut iter| {
                iter.next().and_then(|b| {
                    b.ok().map(|bat| BatteryInfo {
                        percent: bat.state_of_charge().get::<battery::units::ratio::percent>() as f64,
                        state: format!("{:?}", bat.state()),
                        time_remaining: bat.time_to_empty().map(|t| {
                            Duration::from_secs(t.get::<battery::units::time::second>() as u64)
                        }),
                    })
                })
            })
        });

        let active_tab = match config.default_tab {
            1 => ActiveTab::Overview,
            2 => ActiveTab::Cpu,
            3 => ActiveTab::Memory,
            4 => ActiveTab::Disks,
            5 => ActiveTab::Network,
            6 => ActiveTab::Processes,
            7 => ActiveTab::Gpu,
            8 => ActiveTab::Services,
            9 => ActiveTab::Sockets,
            _ => ActiveTab::Overview,
        };

        App {
            config: config.clone(),
            sys,
            networks: Networks::new(),
            disks: Disks::new(),
            components: Components::new(),

            battery_manager,
            battery_info,

            active_tab,
            show_help: false,
            paused: false,

            cpu_history: VecDeque::with_capacity(HISTORY_LEN),
            per_core_history: (0..num_cpus)
                .map(|_| VecDeque::with_capacity(HISTORY_LEN))
                .collect(),
            ram_history: VecDeque::with_capacity(HISTORY_LEN),
            swap_history: VecDeque::with_capacity(HISTORY_LEN),
            net_rx_history: VecDeque::with_capacity(HISTORY_LEN),
            net_tx_history: VecDeque::with_capacity(HISTORY_LEN),
            gpu_history: VecDeque::with_capacity(HISTORY_LEN),

            sort_mode: SortMode::Cpu,
            sort_direction: SortDirection::Desc,
            process_view: ProcessViewMode::List,
            filter_text_area: TextArea::default(),
            filter_active: false,
            filter_regex: false,
            selected: 0,
            show_detail_panel: false,

            signal_menu_open: false,
            selected_signal: 0,

            tick_rate: Duration::from_millis(config.refresh_rate_ms),
            last_update: Instant::now() - Duration::from_secs(10), // force immediate first refresh
            last_process_refresh: Instant::now() - Duration::from_secs(10),
            last_disk_refresh: Instant::now() - Duration::from_secs(10),
            last_gpu_refresh: Instant::now() - Duration::from_secs(10),
            last_service_refresh: Instant::now() - Duration::from_secs(30),
            load_avg: (0.0, 0.0, 0.0),
            gpu_model: {
                #[cfg(target_os = "macos")]
                { "Apple M4".to_string() }
                #[cfg(target_os = "linux")]
                {
                    crate::linux_helper::detect_gpu_from_sysfs()
                        .map(|g| g.model_name)
                        .unwrap_or_else(|| "Unknown GPU".to_string())
                }
                #[cfg(not(any(target_os = "macos", target_os = "linux")))]
                { "Unknown GPU".to_string() }
            },
            gpu_driver: {
                #[cfg(target_os = "macos")]
                { "Apple Metal".to_string() }
                #[cfg(target_os = "linux")]
                {
                    crate::linux_helper::detect_gpu_from_sysfs()
                        .map(|g| g.driver)
                        .unwrap_or_else(|| "unknown".to_string())
                }
                #[cfg(not(any(target_os = "macos", target_os = "linux")))]
                { "unknown".to_string() }
            },
            gpu_vendor: {
                #[cfg(target_os = "macos")]
                { "Apple".to_string() }
                #[cfg(target_os = "linux")]
                {
                    crate::linux_helper::detect_gpu_from_sysfs()
                        .map(|g| match g.vendor {
                            crate::linux_helper::GpuVendor::Amd => "AMD".to_string(),
                            crate::linux_helper::GpuVendor::Intel => "Intel".to_string(),
                            crate::linux_helper::GpuVendor::Nvidia => "NVIDIA".to_string(),
                            crate::linux_helper::GpuVendor::Unknown => "Unknown".to_string(),
                        })
                        .unwrap_or_else(|| "Unknown".to_string())
                }
                #[cfg(not(any(target_os = "macos", target_os = "linux")))]
                { "Unknown".to_string() }
            },
            gpu_usage: -1.0,
            gpu_power_mw: None,
            cpu_power_mw: None,
            pkg_power_mw: None,

            #[cfg(target_os = "linux")]
            nvidia_gpus: Vec::new(),
            #[cfg(target_os = "linux")]
            gpu_temp: None,
            #[cfg(target_os = "linux")]
            gpu_clock_mhz: None,
            #[cfg(target_os = "linux")]
            gpu_vram_used: None,
            #[cfg(target_os = "linux")]
            gpu_vram_total: None,

            network_info: HashMap::new(),

            services: Vec::new(),
            service_selected: 0,
            service_action_pending: None,

            sockets: Vec::new(),
            socket_selected: 0,

            focus_pid: None,
            focus_cpu_history: VecDeque::with_capacity(HISTORY_LEN),
            focus_mem_history: VecDeque::with_capacity(HISTORY_LEN),

            #[cfg(target_os = "macos")]
            compressed_mem_cache: HashMap::new(),
        }
    }

    /// Push a value into a history buffer, evicting the oldest if full.
    fn push_history(buf: &mut VecDeque<u64>, val: u64) {
        if buf.len() >= HISTORY_LEN {
            buf.pop_front();
        }
        buf.push_back(val);
    }

    /// Refresh all system data and update history buffers.
    pub fn refresh(&mut self) {
        if self.paused {
            return;
        }

        let now = Instant::now();

        // 1. FAST REFRESH: CPU and Memory (needed for sparklines and gauges)
        let rk_fast = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything());
        self.sys.refresh_specifics(rk_fast);
        
        // Refresh load average
        let load = System::load_average();
        self.load_avg = (load.one, load.five, load.fifteen);

        // 2. THROTTLED REFRESH: Processes (every 3 seconds, or 1 second if on Processes tab)
        let proc_timeout = if self.active_tab == ActiveTab::Processes {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        if now.duration_since(self.last_process_refresh) >= proc_timeout {
            self.sys.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::everything(),
            );
            self.last_process_refresh = now;

            // Expensive macOS memory calculations are bundled with process refresh
            #[cfg(target_os = "macos")]
            {
                self.compressed_mem_cache.clear();
                for pid in self.sys.processes().keys() {
                    if let Some(info) = crate::macos_helper::get_process_memory_info(pid.as_u32() as i32) {
                        self.compressed_mem_cache.insert(*pid, info.compressed);
                    }
                }
            }
        }

        // 3. THROTTLED REFRESH: Disks (every 10 seconds)
        if now.duration_since(self.last_disk_refresh) >= Duration::from_secs(10) {
            self.disks.refresh(true);
            self.last_disk_refresh = now;
        }

        // 4. THROTTLED REFRESH: GPU (every 2 seconds)
        if now.duration_since(self.last_gpu_refresh) >= Duration::from_secs(2) {
            self.refresh_gpu();
            self.last_gpu_refresh = now;
        }

        // 5. THROTTLED REFRESH: Services (every 10 seconds, only when on Services tab)
        if self.active_tab == ActiveTab::Services
            && now.duration_since(self.last_service_refresh) >= Duration::from_secs(10)
        {
            self.refresh_services();
            self.last_service_refresh = now;
        }

        // 5. ALWAYS REFRESH: Networks and Components (sensors)
        self.networks.refresh(true);
        self.components.refresh(true);

        // Network enrichment (pnet)
        #[cfg(target_os = "linux")]
        {
            for interface in pnet::datalink::interfaces() {
                let linux_info = crate::linux_helper::get_interface_extra_info(&interface.name);
                let (speed, duplex, driver) = (
                    linux_info.as_ref().and_then(|e| e.speed),
                    linux_info.as_ref().and_then(|e| e.duplex.clone()),
                    linux_info.as_ref().and_then(|e| e.driver.clone())
                );

                self.network_info.insert(interface.name.clone(), NetworkInterfaceInfo {
                    mac: interface.mac.map(|m| m.to_string()).unwrap_or_else(|| "00:00:00:00:00:00".into()),
                    mtu: 0,
                    speed,
                    duplex,
                    driver,
                });
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            for interface in pnet::datalink::interfaces() {
                self.network_info.insert(interface.name.clone(), NetworkInterfaceInfo {
                    mac: interface.mac.map(|m| m.to_string()).unwrap_or_else(|| "00:00:00:00:00:00".into()),
                    mtu: 0,
                    speed: None,
                    duplex: None,
                    driver: None,
                });
            }
        }

        // CPU history
        let global_cpu: f64 = if !self.sys.cpus().is_empty() {
            self.sys.cpus().iter().map(|c| c.cpu_usage() as f64).sum::<f64>()
                / self.sys.cpus().len() as f64
        } else {
            0.0
        };
        Self::push_history(&mut self.cpu_history, global_cpu as u64);

        // Per-core history
        for (i, cpu) in self.sys.cpus().iter().enumerate() {
            if i < self.per_core_history.len() {
                Self::push_history(&mut self.per_core_history[i], cpu.cpu_usage() as u64);
            }
        }

        // Memory history
        let total_mem = self.sys.total_memory();
        let used_mem = self.sys.used_memory();
        let mem_pct = if total_mem > 0 {
            (used_mem as f64 / total_mem as f64 * 100.0) as u64
        } else {
            0
        };
        Self::push_history(&mut self.ram_history, mem_pct);

        let total_swap = self.sys.total_swap();
        let used_swap = self.sys.used_swap();
        let swap_pct = if total_swap > 0 {
            (used_swap as f64 / total_swap as f64 * 100.0) as u64
        } else {
            0
        };
        Self::push_history(&mut self.swap_history, swap_pct);

        // Network history (total across all interfaces)
        let rx: u64 = self.networks.iter().map(|(_, d): (&String, &sysinfo::NetworkData)| d.received()).sum();
        let tx: u64 = self.networks.iter().map(|(_, d): (&String, &sysinfo::NetworkData)| d.transmitted()).sum();
        Self::push_history(&mut self.net_rx_history, rx);
        Self::push_history(&mut self.net_tx_history, tx);

        // Battery
        if let Some(ref mgr) = self.battery_manager {
            if let Ok(mut batteries) = mgr.batteries() {
                if let Some(Ok(bat)) = batteries.next() {
                    let state = bat.state();
                    let percent = bat.state_of_charge().get::<battery::units::ratio::percent>() as f64;
                    
                    // Sanity check for macOS "Unknown" state when plugged in
                    let state_str = if format!("{:?}", state) == "Unknown" {
                        if percent > 95.0 {
                            "Full / Plugged In".to_string()
                        } else {
                            "Plugged In / Not Charging".to_string()
                        }
                    } else {
                        format!("{:?}", state)
                    };

                    self.battery_info = Some(BatteryInfo {
                        percent,
                        state: state_str,
                        time_remaining: bat.time_to_empty().map(|t| {
                            Duration::from_secs(
                                t.get::<battery::units::time::second>() as u64,
                            )
                        }),
                    });
                }
            }
        }

        self.last_update = now;
    }

    /// Get tick rate in human-readable format.
    pub fn tick_rate_label(&self) -> String {
        format!("{}ms", self.tick_rate.as_millis())
    }

    /// Get compressed memory for a process (macOS only, returns 0 elsewhere).
    pub fn get_compressed_mem(&self, pid: sysinfo::Pid) -> u64 {
        #[cfg(target_os = "macos")]
        {
            self.compressed_mem_cache.get(&pid).copied().unwrap_or(0)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = pid;
            0
        }
    }

    /// Refresh GPU usage and power data.
    fn refresh_gpu(&mut self) {
        if !self.config.gpu_enabled {
            self.gpu_usage = -1.0;
            return;
        }

        #[cfg(target_os = "macos")]
        {
            let is_root = unsafe { libc::getuid() } == 0;
            let mut cmd = if is_root {
                std::process::Command::new("powermetrics")
            } else {
                let mut c = std::process::Command::new("sudo");
                c.arg("powermetrics");
                c
            };

            if let Ok(output) = cmd
                .args(["--samplers", "gpu_power,cpu_power", "-n", "1", "-i", "100"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut found_usage = -1.0;
                for line in stdout.lines() {
                    let low = line.to_lowercase();
                    // GPU usage %
                    // Targeted search for "active residency" or "gpu use" while ignoring "idle"
                    if low.contains('%') && (low.contains("active") || low.contains("use")) && low.contains("gpu") {
                        if !low.contains("idle") || low.find("active").unwrap_or(usize::MAX) < low.find("idle").unwrap_or(usize::MAX) {
                            if let Some(pct_idx) = line.find('%') {
                                let start = line[..pct_idx].rfind(|c: char| c == ' ' || c == ':').map(|i| i + 1).unwrap_or(0);
                                if let Ok(pct) = line[start..pct_idx].trim().parse::<f64>() {
                                    found_usage = pct;
                                }
                            }
                        }
                    }
                    // Power readings (mW)
                    let parse_mw = |line: &str| -> Option<f64> {
                        // e.g. "GPU Power: 312 mW"
                        if let Some(mw_idx) = line.to_lowercase().find("mw") {
                            let part = line[..mw_idx].trim();
                            let start = part.rfind(|c: char| c == ' ' || c == ':').map(|i| i + 1).unwrap_or(0);
                            part[start..].trim().parse::<f64>().ok()
                        } else {
                            None
                        }
                    };
                    if low.contains("gpu power") {
                        self.gpu_power_mw = parse_mw(line);
                    } else if low.contains("cpu power") && !low.contains("e-cluster") && !low.contains("p-cluster") {
                        self.cpu_power_mw = parse_mw(line);
                    } else if low.contains("package power") || low.contains("combined power") {
                        self.pkg_power_mw = parse_mw(line);
                    }
                }
                self.gpu_usage = found_usage;
            }
            Self::push_history(&mut self.gpu_history, self.gpu_usage.max(0.0) as u64);
        }

        #[cfg(target_os = "linux")]
        {
            // 1. Check NVIDIA via NVML
            self.nvidia_gpus = crate::linux_helper::get_nvidia_gpu_info();
            if !self.nvidia_gpus.is_empty() {
                // If multiple, show the first one in the sparkline for now
                self.gpu_usage = self.nvidia_gpus[0].memory_used_pct; // or usagepct if available
                self.gpu_model = self.nvidia_gpus[0].name.clone();
            } else {
                // 2. Fallback to /sys (AMD/Intel integrated)
                let paths = [
                    "/sys/class/drm/card0/device/gpu_busy_percent",
                    "/sys/class/drm/card1/device/gpu_busy_percent",
                ];
                for path in paths {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if let Ok(pct) = content.trim().parse::<f64>() {
                            self.gpu_usage = pct;
                            break;
                        }
                    }
                }

                // Collect AMD-specific stats from sysfs/hwmon
                self.gpu_temp = crate::linux_helper::get_amd_gpu_temp();
                self.gpu_clock_mhz = crate::linux_helper::get_amd_gpu_clock();
                if let Some((used, total)) = crate::linux_helper::get_amd_vram_usage() {
                    self.gpu_vram_used = Some(used);
                    self.gpu_vram_total = Some(total);
                }
            }
            Self::push_history(&mut self.gpu_history, self.gpu_usage.max(0.0) as u64);
        }
    }

    /// Refresh list of services via `launchctl` (macOS) or `systemctl` (Linux).
    pub fn refresh_services(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("launchctl").arg("list").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut entries: Vec<ServiceEntry> = stdout
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
                self.services = entries;
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try system-level first, then user-level as fallback (Steam Deck)
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
                            // Mark running services with a synthetic PID indicator
                            let pid = if sub == "running" { Some(1) } else { None };
                            let status = if active == "active" { 0 } else { -1 };
                            Some(ServiceEntry { pid, status, label })
                        } else {
                            None
                        }
                    })
                    .collect()
            };

            // Try system services first  
            let mut entries = try_systemctl(&[]);
            
            // If empty, try user services (Steam Deck often runs as user 'deck')
            if entries.is_empty() {
                entries = try_systemctl(&["--user"]);
            }

            entries.sort_by(|a, b| a.label.cmp(&b.label));
            self.services = entries;
        }

        // Clamp selection
        if self.service_selected >= self.services.len() && !self.services.is_empty() {
            self.service_selected = self.services.len() - 1;
        }
    }

    /// Refresh active TCP/UDP sockets via netstat2 (native).
    pub fn refresh_sockets(&mut self) {
        let mut entries: Vec<SocketEntry> = Vec::new();

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
                    self.sys.process(sysinfo::Pid::from(p as usize))
                        .map(|pr| pr.name().to_string_lossy().to_string())
                        .unwrap_or_else(|| "-".to_string())
                } else {
                    "-".to_string()
                };

                // Only show established or listening or meaningful states
                if state != "Closed" {
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

        entries.sort_by(|a, b| a.local_addr.cmp(&b.local_addr));
        entries.truncate(1000);
        self.sockets = entries;
        if self.socket_selected >= self.sockets.len() && !self.sockets.is_empty() {
            self.socket_selected = self.sockets.len() - 1;
        }
    }

    /// Update per-process focus history (called every tick if focus_pid is set).
    pub fn update_focus_history(&mut self) {
        if let Some(pid) = self.focus_pid {
            if let Some(proc) = self.sys.process(pid) {
                let cpu = proc.cpu_usage() as u64;
                let mem_total = self.get_compressed_mem(pid);
                let mem_bytes = proc.memory() + mem_total;
                let total_mem = self.sys.total_memory();
                let mem_pct = if total_mem > 0 { mem_bytes * 100 / total_mem } else { 0 };
                Self::push_history(&mut self.focus_cpu_history, cpu);
                Self::push_history(&mut self.focus_mem_history, mem_pct);
            }
        }
    }
}
