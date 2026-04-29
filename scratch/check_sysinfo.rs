use sysinfo::{System, ProcessExt}; // Might need trait

fn main() {
    let mut s = System::new_all();
    s.refresh_processes();
    for p in s.processes().values() {
        // println!("{}", p.tasks().len());
    }
}
