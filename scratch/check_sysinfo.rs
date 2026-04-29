use sysinfo::{System, Process};

fn main() {
    let mut s = System::new_all();
    s.refresh_processes();
    if let Some(p) = s.processes().values().next() {
        // We will try to call methods and see what compiles.
        // I'll comment out ones that fail.
        // println!("Threads: {}", p.thread_count());
        // println!("Priority: {}", p.priority());
    }
}
