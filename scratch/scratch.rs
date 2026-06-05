use sysinfo::{System, Process};
fn main() {
    let mut sys = System::new_all();
    sys.refresh_all();
    let p = sys.processes().values().next().unwrap();
    // try to print thread count etc
    // println!("{:?}", p.thread_count());
}
