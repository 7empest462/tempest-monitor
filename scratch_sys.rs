use sysinfo::{System, Process};
fn check_process_methods(p: &Process) {
    let threads = p.thread_count();
    let cpu_time = p.run_time();
    let name = p.name();
}
fn main() {}
