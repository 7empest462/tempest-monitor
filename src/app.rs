#[cfg(target_os = "macos")]
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind,
    RefreshKind, System,
};

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
}

impl ActiveTab {
    pub const ALL: [ActiveTab; 7] = [
        ActiveTab::Overview,
        ActiveTab::Cpu,
        ActiveTab::Memory,
        ActiveTab::Disks,
        ActiveTab::Network,
        ActiveTab::Processes,
        ActiveTab::Gpu,
    ];

    pub fn index(self) -> usize {
        match self {
            ActiveTab::Overview => 0,
            ActiveTab::Cpu => 1,
            ActiveTab::Memory => 2,
            ActiveTab::Disks => 3,
            ActiveTab::Network => 4,
            ActiveTab::Processes => 5,
            ActiveTab::Gpu => 6,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ActiveTab::Overview => "Overvw",
            ActiveTab::Cpu => "CPU",
            ActiveTab::Memory => "Mem",
            ActiveTab::Disks => "Disk",
            ActiveTab::Network => "Net",
            ActiveTab::Processes => "Proc",
            ActiveTab::Gpu => "GPU",
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

// ── Main App state ───────────────────────────────────────────────────────────

pub struct App {
    // sysinfo collectors
    pub sys: System,
    pub networks: Networks,
    pub disks: Disks,
    pub components: Components,
    pub load_avg: (f64, f64, f64),
    pub gpu_model: String,
    pub gpu_usage: f64,

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
    pub filter: String,
    pub filter_regex: bool,
    pub selected: usize,
    pub show_detail_panel: bool,

    // Signal menu
    pub signal_menu_open: bool,
    pub selected_signal: usize,

    #[cfg(target_os = "macos")]
    pub compressed_mem_cache: HashMap<sysinfo::Pid, u64>,

    // Timing
    pub tick_rate: Duration,
    pub last_update: Instant,

    // Last refresh timestamps for throttling
    pub last_process_refresh: Instant,
    pub last_disk_refresh: Instant,
    pub last_gpu_refresh: Instant,
}

impl App {
    pub fn new() -> Self {
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

        App {
            sys,
            networks: Networks::new(),
            disks: Disks::new(),
            components: Components::new(),

            battery_manager,
            battery_info,

            active_tab: ActiveTab::Overview,
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
            filter: String::new(),
            filter_regex: false,
            selected: 0,
            show_detail_panel: false,

            signal_menu_open: false,
            selected_signal: 0,

            tick_rate: Duration::from_millis(1200),
            last_update: Instant::now() - Duration::from_secs(10), // force immediate first refresh
            last_process_refresh: Instant::now() - Duration::from_secs(10),
            last_disk_refresh: Instant::now() - Duration::from_secs(10),
            last_gpu_refresh: Instant::now() - Duration::from_secs(10),
            load_avg: (0.0, 0.0, 0.0),
            gpu_model: {
                #[cfg(target_os = "macos")]
                { "Apple M4".to_string() }
                #[cfg(not(target_os = "macos"))]
                { "Generic / Integrated GPU".to_string() }
            },
            gpu_usage: -1.0,

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

        // 5. ALWAYS REFRESH: Networks and Components (sensors)
        self.networks.refresh(true);
        self.components.refresh(true);

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

    /// Refresh GPU usage data.
    fn refresh_gpu(&mut self) {
        #[cfg(target_os = "macos")]
        {
            // Detection: powermetrics requires root. If we aren't root, we must use sudo.
            let is_root = unsafe { libc::getuid() } == 0;
            let mut cmd = if is_root {
                std::process::Command::new("powermetrics")
            } else {
                let mut c = std::process::Command::new("sudo");
                c.arg("powermetrics");
                c
            };

            if let Ok(output) = cmd
                .args(["--samplers", "gpu_power", "-n", "1", "-i", "1"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    // M4/M1/M2/M3 use different labels: "GPU use", "GPU HW active residency", etc.
                    if line.contains("GPU ") && line.contains('%') {
                        if let Some(pct_idx) = line.find('%') {
                            // Look backwards for the start of the number (space or colon)
                            let start = line[..pct_idx].rfind(|c: char| c == ' ' || c == ':').map(|i| i + 1).unwrap_or(0);
                            if let Ok(pct) = line[start..pct_idx].trim().parse::<f64>() {
                                self.gpu_usage = pct;
                                break;
                            }
                        }
                    }
                }
            }
            Self::push_history(&mut self.gpu_history, self.gpu_usage.max(0.0) as u64);
        }

        #[cfg(target_os = "linux")]
        {
            // Try common sysfs paths for GPU usage
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
        }
    }
}
