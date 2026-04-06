use sysinfo::{System, ProcessRefreshKind};

#[derive(Debug, Clone)]
pub struct ProcessSummary {
    pub pid: i32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
}

/// Returns the top N processes by memory usage.
pub fn get_top_memory_processes(count: usize) -> Vec<ProcessSummary> {
    let mut sys = System::new_all();
    sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_memory().with_cpu());
    
    let mut processes: Vec<ProcessSummary> = sys.processes().values().map(|p| {
        ProcessSummary {
            pid: p.pid().as_u32() as i32,
            name: p.name().to_string_lossy().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_bytes: p.memory(),
        }
    }).collect();

    // Sort by memory descending
    processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    processes.truncate(count);
    processes
}

/// Returns the top N processes by CPU usage.
pub fn get_top_cpu_processes(count: usize) -> Vec<ProcessSummary> {
    let mut sys = System::new_all();
    sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_memory().with_cpu());
    
    let mut processes: Vec<ProcessSummary> = sys.processes().values().map(|p| {
        ProcessSummary {
            pid: p.pid().as_u32() as i32,
            name: p.name().to_string_lossy().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_bytes: p.memory(),
        }
    }).collect();

    // Sort by CPU descending
    processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
    processes.truncate(count);
    processes
}
