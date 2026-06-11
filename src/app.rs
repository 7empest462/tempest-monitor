use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind,
    RefreshKind, System,
};

// use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
// use pnet::datalink;
use ratatui_textarea::TextArea;

use crate::cli::CliArgs;
use crate::config::TempestConfig;
use crate::power_mode::CpuPowerMode;

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
    History,
}

impl ActiveTab {
    pub const ALL: [ActiveTab; 10] = [
        ActiveTab::Overview,
        ActiveTab::Cpu,
        ActiveTab::Memory,
        ActiveTab::Disks,
        ActiveTab::Network,
        ActiveTab::Processes,
        ActiveTab::Gpu,
        ActiveTab::Services,
        ActiveTab::Sockets,
        ActiveTab::History,
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
            ActiveTab::History  => 9,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ActiveTab::Overview  => "Overview",
            ActiveTab::Cpu      => "CPU",
            ActiveTab::Memory   => "RAM",
            ActiveTab::Disks    => "Disk",
            ActiveTab::Network  => "Net",
            ActiveTab::Processes => "Proc",
            ActiveTab::Gpu      => "GPU",
            ActiveTab::Services => "Svc",
            ActiveTab::Sockets  => "Socks",
            ActiveTab::History  => "History",
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

// ── Service Inspector mode ───────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ServiceInspectorMode {
    View,   // Show service file contents
    Logs,   // Show service logs
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

// Structs moved to system_helper.rs for library portability
pub use crate::system_helper::{ServiceEntry, SocketEntry};

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

#[derive(Clone, Default)]
pub struct ProcessExtraInfo {
    pub compressed_mem: u64,
    pub thread_count: i32,
    pub priority: i32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct MetricSnapshot {
    pub id: i64,
    pub timestamp: String,
    pub cpu_usage: f64,
    pub mem_used_gb: f64,
    pub gpu_usage: f64,
    pub net_rx_kbps: f64,
    pub net_tx_kbps: f64,
}

pub struct ProcessesState {
    pub sort_mode: SortMode,
    pub sort_direction: SortDirection,
    pub view_mode: ProcessViewMode,
    pub table_state: ratatui::widgets::TableState,
    pub filter_text_area: TextArea<'static>,
    pub filter_active: bool,
    pub filter_regex: bool,
    pub selected: usize,
    pub show_detail_panel: bool,
    pub signal_menu_open: bool,
    pub selected_signal: usize,
    pub focus_pid: Option<sysinfo::Pid>,
    pub focus_cpu_history: VecDeque<u64>,
    pub focus_mem_history: VecDeque<u64>,
    pub extra_cache: HashMap<sysinfo::Pid, ProcessExtraInfo>,
}

pub struct ServicesState {
    pub list: Vec<ServiceEntry>,
    pub selected: usize,
    pub action_pending: Option<String>,
    pub inspector_open: bool,
    pub inspector_scroll: u16,
    pub file_path: Option<String>,
    pub file_contents: Option<String>,
    pub config_path: Option<String>,
    pub log_lines: Vec<String>,
    pub inspector_mode: ServiceInspectorMode,
    pub is_sip_protected: bool,
}

pub struct SocketsState {
    pub list: Vec<SocketEntry>,
    pub selected: usize,
}

pub struct CpuPowerState {
    pub mode: CpuPowerMode,
    pub feedback: Option<String>,
    pub available_modes: Vec<CpuPowerMode>,
}

pub struct HistoryState {
    pub snapshots: Vec<MetricSnapshot>,
    pub selected: usize,
}

// ── Main App state ───────────────────────────────────────────────────────────

pub struct App {
    pub config: TempestConfig,
    pub config_path: Option<String>,

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
    pub cpu_power_mw: Option<f64>,
    pub pkg_power_mw: Option<f64>,
    pub ane_power_mw: Option<f64>,
    pub gpu_freq_mhz: Option<f64>,

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
    pub battery_manager: Option<starship_battery::Manager>,
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

    // Sub-states
    pub processes: ProcessesState,
    pub services: ServicesState,
    pub sockets: SocketsState,
    pub cpu_power: CpuPowerState,
    pub mem_segments: crate::system_helper::MemorySegments,
    pub history: HistoryState,

    // Editor request (set by input, consumed by main loop)
    pub editor_request: Option<String>,

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
    pub fn new_with_config(_cli: &CliArgs, config: &TempestConfig, config_path: Option<String>) -> Self {
        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything());

        let sys = System::new_with_specifics(refresh_kind);
        let num_cpus = sys.cpus().len();

        let battery_manager = starship_battery::Manager::new().ok();
        let battery_info = battery_manager.as_ref().and_then(|mgr| {
            mgr.batteries().ok().and_then(|mut iter| {
                iter.next().and_then(|b| {
                    b.ok().map(|bat| BatteryInfo {
                        percent: bat.state_of_charge().get::<starship_battery::units::ratio::percent>() as f64,
                        state: format!("{:?}", bat.state()),
                        time_remaining: bat.time_to_empty().map(|t| {
                            Duration::from_secs(t.get::<starship_battery::units::time::second>() as u64)
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
            10 => ActiveTab::History,
            _ => ActiveTab::Overview,
        };

        let mut app = App {
            config: config.clone(),
            config_path,
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

            tick_rate: Duration::from_millis(config.refresh_rate_ms),
            last_update: Instant::now() - Duration::from_secs(10), // force immediate first refresh
            last_process_refresh: Instant::now() - Duration::from_secs(10),
            last_disk_refresh: Instant::now() - Duration::from_secs(10),
            last_gpu_refresh: Instant::now() - Duration::from_secs(10),
            last_service_refresh: Instant::now() - Duration::from_secs(30),
            load_avg: (0.0, 0.0, 0.0),
            gpu_model: String::new(),
            gpu_driver: String::new(),
            gpu_vendor: String::new(),
            gpu_usage: -1.0,
            gpu_power_mw: None,
            cpu_power_mw: None,
            pkg_power_mw: None,
            ane_power_mw: None,
            gpu_freq_mhz: None,

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

            processes: ProcessesState {
                sort_mode: SortMode::Cpu,
                sort_direction: SortDirection::Desc,
                view_mode: ProcessViewMode::List,
                table_state: ratatui::widgets::TableState::default(),
                filter_text_area: TextArea::default(),
                filter_active: false,
                filter_regex: false,
                selected: 0,
                show_detail_panel: false,
                signal_menu_open: false,
                selected_signal: 0,
                focus_pid: None,
                focus_cpu_history: VecDeque::with_capacity(HISTORY_LEN),
                focus_mem_history: VecDeque::with_capacity(HISTORY_LEN),
                extra_cache: HashMap::new(),
            },

            services: ServicesState {
                list: Vec::new(),
                selected: 0,
                action_pending: None,
                inspector_open: false,
                inspector_scroll: 0,
                file_path: None,
                file_contents: None,
                config_path: None,
                log_lines: Vec::new(),
                inspector_mode: ServiceInspectorMode::View,
                is_sip_protected: false,
            },

            sockets: SocketsState {
                list: Vec::new(),
                selected: 0,
            },

            cpu_power: CpuPowerState {
                mode: crate::power_mode::detect_power_mode(),
                feedback: None,
                available_modes: crate::power_mode::available_modes(),
            },

            mem_segments: crate::system_helper::MemorySegments::default(),

            history: HistoryState {
                snapshots: Vec::new(),
                selected: 0,
            },

            editor_request: None,
        };

        // Perform initial GPU detection to populate model and initial stats
        app.refresh_gpu();
        
        app
    }

    /// Push a value into a history buffer, evicting the oldest if full.
    fn push_history(buf: &mut VecDeque<u64>, val: u64) {
        if buf.len() >= HISTORY_LEN {
            buf.pop_front();
        }
        buf.push_back(val);
    }

    /// Cycle to the next theme dynamically and save the setting to config.yaml.
    pub fn cycle_theme(&mut self) {
        let next_theme = match self.config.theme.to_lowercase().as_str() {
            "dark" => "light",
            "light" => "nord",
            "nord" => "catppuccin",
            "catppuccin" => "dracula",
            "dracula" => "gruvbox",
            "gruvbox" => "tokyo-night",
            "tokyo-night" | "tokyo_night" | "tokyonight" => "dark",
            _ => "dark",
        };
        self.config.theme = next_theme.to_string();
        crate::theme::set_theme(&self.config.theme);
        if let Err(e) = self.config.save(self.config_path.as_deref()) {
            log::warn!("Failed to save theme config: {}", e);
        }
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

            // Expensive platform-specific metadata calculations
            cfg_select! {
                target_os = "macos" => {
                    self.processes.extra_cache.clear();
                    for pid in self.sys.processes().keys() {
                        if let Some(meta) = crate::macos_helper::get_process_metadata(pid.as_u32() as i32) {
                            self.processes.extra_cache.insert(*pid, ProcessExtraInfo {
                                compressed_mem: meta.compressed,
                                thread_count: meta.thread_count,
                                priority: meta.priority,
                            });
                        }
                    }
                },
                target_os = "linux" => {
                    self.processes.extra_cache.clear();
                    for pid in self.sys.processes().keys() {
                        if let Some(meta) = crate::linux_helper::get_process_metadata(pid.as_u32() as i32) {
                            self.processes.extra_cache.insert(*pid, ProcessExtraInfo {
                                compressed_mem: 0,
                                thread_count: meta.thread_count,
                                priority: meta.priority,
                            });
                        }
                    }
                },
                _ => {}
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
        #[cfg(target_os = "macos")]
        {
            for interface in pnet::datalink::interfaces() {
                let mut info = NetworkInterfaceInfo {
                    mac: interface.mac.map(|m| m.to_string()).unwrap_or_else(|| "00:00:00:00:00:00".into()),
                    mtu: 0,
                    speed: None,
                    duplex: None,
                    driver: None,
                };

                if let Some(mac_info) = crate::macos_helper::get_macos_interface_info(&interface.name) {
                    info.mtu = mac_info.mtu;
                    info.speed = mac_info.speed;
                    info.duplex = mac_info.duplex;
                    info.driver = mac_info.driver;
                }

                self.network_info.insert(interface.name.clone(), info);
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
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

        // Memory segments
        self.mem_segments = crate::system_helper::get_memory_segments(&self.sys);

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
        let rx: u64 = self.networks.values().map(|d| d.received()).sum();
        let tx: u64 = self.networks.values().map(|d| d.transmitted()).sum();
        Self::push_history(&mut self.net_rx_history, rx);
        Self::push_history(&mut self.net_tx_history, tx);

        // Battery
        if let Some(ref mgr) = self.battery_manager
            && let Ok(mut batteries) = mgr.batteries()
            && let Some(Ok(bat)) = batteries.next()
        {
            let state = bat.state();
            let percent = bat.state_of_charge().get::<starship_battery::units::ratio::percent>() as f64;
            
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
                        t.get::<starship_battery::units::time::second>() as u64,
                    )
                }),
            });
        }

        self.last_update = now;
    }

    /// Get tick rate in human-readable format.
    pub fn tick_rate_label(&self) -> String {
        format!("{}ms", self.tick_rate.as_millis())
    }

    /// Get extra info for a process.
    pub fn get_extra_info(&self, pid: sysinfo::Pid) -> ProcessExtraInfo {
        self.processes.extra_cache.get(&pid).cloned().unwrap_or_default()
    }

    /// Get compressed memory for a process.
    pub fn get_compressed_mem(&self, pid: sysinfo::Pid) -> u64 {
        self.processes.extra_cache.get(&pid).map(|e| e.compressed_mem).unwrap_or(0)
    }

    /// Refresh GPU usage and power data.
    fn refresh_gpu(&mut self) {
        if !self.config.gpu_enabled {
            self.gpu_usage = -1.0;
            return;
        }

        cfg_select! {
            target_os = "macos" => {
                let tel = crate::macos_helper::get_macos_gpu_info(true);
                self.gpu_usage = tel.usage_pct;
                self.gpu_power_mw = tel.power_mw;
                self.cpu_power_mw = tel.cpu_power_mw;
                self.pkg_power_mw = tel.package_power_mw;
                self.ane_power_mw = tel.ane_power_mw;
                self.gpu_freq_mhz = tel.gpu_freq_mhz;
                self.gpu_model = tel.model;
                
                Self::push_history(&mut self.gpu_history, self.gpu_usage.max(0.0) as u64);
            },
            target_os = "linux" => {
                let tel = crate::linux_helper::collect_gpu_telemetry();
                self.gpu_usage = tel.usage_pct;
                self.gpu_temp = tel.temp_c;
                self.gpu_clock_mhz = tel.clock_mhz;
                self.gpu_vram_used = tel.vram_used;
                self.gpu_vram_total = tel.vram_total;
                self.gpu_model = tel.model;
                self.gpu_driver = tel.driver;
                self.nvidia_gpus = tel.nvidia_info;

                Self::push_history(&mut self.gpu_history, self.gpu_usage.max(0.0) as u64);
            },
            _ => {}
        }
    }

    /// Refresh list of services via `launchctl` (macOS) or `systemctl` (Linux).
    pub fn refresh_services(&mut self) {
        self.services.list = crate::system_helper::get_services();
        
        // Clamp selection
        if self.services.selected >= self.services.list.len() && !self.services.list.is_empty() {
            self.services.selected = self.services.list.len() - 1;
        }
    }

    /// Refresh active TCP/UDP sockets via netstat2 (native).
    pub fn refresh_sockets(&mut self) {
        self.sockets.list = crate::system_helper::get_sockets(&self.sys);

        // Clamp selection
        if self.sockets.selected >= self.sockets.list.len() && !self.sockets.list.is_empty() {
            self.sockets.selected = self.sockets.list.len() - 1;
        }
    }

    /// Update per-process focus history (called every tick if focus_pid is set).
    pub fn update_focus_history(&mut self) {
        if let Some(pid) = self.processes.focus_pid
            && let Some(proc) = self.sys.process(pid)
        {
            let cpu = proc.cpu_usage() as u64;
            let mem_total = self.get_compressed_mem(pid);
            let mem_bytes = proc.memory() + mem_total;
            let total_mem = self.sys.total_memory();
            let mem_pct = (mem_bytes * 100).checked_div(total_mem).unwrap_or(0);
            Self::push_history(&mut self.processes.focus_cpu_history, cpu);
            Self::push_history(&mut self.processes.focus_mem_history, mem_pct);
        }
    }

    /// Open the service inspector for the currently selected service.
    pub fn open_service_inspector(&mut self) {
        if let Some(svc) = self.services.list.get(self.services.selected) {
            let label = svc.label.clone();

            // Resolve the service file path
            let file_path = crate::service_inspector::resolve_service_file(&label);
            let file_contents = file_path.as_ref()
                .and_then(|p| crate::service_inspector::read_service_file(p));

            // Detect config file
            let config_path = file_contents.as_ref()
                .and_then(|c| crate::service_inspector::detect_config_file(c));

            // Check SIP protection
            let is_protected = file_path.as_ref()
                .map(|p| crate::service_inspector::is_sip_protected(p))
                .unwrap_or(false);

            self.services.file_path = file_path;
            self.services.file_contents = file_contents;
            self.services.config_path = config_path;
            self.services.is_sip_protected = is_protected;
            self.services.inspector_scroll = 0;
            self.services.inspector_mode = ServiceInspectorMode::View;
            self.services.log_lines.clear();
            self.services.inspector_open = true;
        }
    }

    /// Close the service inspector and return to the list.
    pub fn close_service_inspector(&mut self) {
        self.services.inspector_open = false;
        self.services.file_path = None;
        self.services.file_contents = None;
        self.services.config_path = None;
        self.services.log_lines.clear();
        self.services.is_sip_protected = false;
    }

    /// Load logs for the currently inspected service.
    pub fn load_service_logs(&mut self) {
        if let Some(svc) = self.services.list.get(self.services.selected) {
            let label = svc.label.clone();
            self.services.log_lines = crate::service_inspector::get_service_logs(
                &label,
                self.services.file_contents.as_deref(),
            );
        }
    }

    /// Refresh the current CPU power mode.
    pub fn refresh_cpu_power_mode(&mut self) {
        self.cpu_power.mode = crate::power_mode::detect_power_mode();
    }

    /// Set CPU power mode and store feedback.
    pub fn set_cpu_power_mode(&mut self, mode: CpuPowerMode) {
        match crate::power_mode::set_power_mode(mode) {
            Ok(msg) => {
                self.cpu_power.feedback = Some(msg);
                self.cpu_power.mode = mode;
            }
            Err(msg) => {
                self.cpu_power.feedback = Some(msg);
            }
        }
    }
}
