use std::mem;

#[repr(C)]
#[derive(Default, Debug)]
#[allow(dead_code)]
struct task_vm_info {
    pub virtual_size: u64,
    pub region_count: i32,
    pub page_size: i32,
    pub resident_size: u64,
    pub resident_size_peak: u64,
    pub device: u64,
    pub device_peak: u64,
    pub internal: u64,
    pub internal_peak: u64,
    pub external: u64,
    pub external_peak: u64,
    pub reusable: u64,
    pub reusable_peak: u64,
    pub purgeable_volatile_size: u64,
    pub purgeable_volatile_clean_size: u64,
    pub purgeable_volatile_compressed_size: u64,
}

#[repr(C)]
#[derive(Default, Debug)]
#[allow(dead_code)]
struct task_vm_info_compressed {
    pub base: task_vm_info,
    pub compressed: u64,
    pub compressed_peak: u64,
    pub compressed_lifetime: u64,
}

unsafe extern "C" {
    fn mach_task_self() -> u32;
    fn task_for_pid(target_tport: u32, pid: i32, tn: *mut u32) -> i32;
    fn task_info(target_task: u32, flavor: i32, task_info_out: *mut i32, task_info_outCnt: *mut u32) -> i32;
    fn mach_port_deallocate(task: u32, name: u32) -> i32;
}

const TASK_VM_INFO: i32 = 22;
const TASK_VM_INFO_COUNT: u32 = (mem::size_of::<task_vm_info_compressed>() / 4) as u32;

pub struct ProcessMemoryInfo {
    pub compressed: u64,
}

/// Retrieves resident and compressed memory info for a given PID using Mach task_info.
/// Requires elevated permissions (sudo) for processes not owned by the current user.
pub fn get_process_memory_info(pid: i32) -> Option<ProcessMemoryInfo> {
    let mut task: u32 = 0;
    let self_port = unsafe { mach_task_self() };
    
    let res = unsafe { task_for_pid(self_port, pid, &mut task) };
    if res != 0 {
        return None;
    }

    let mut info = task_vm_info_compressed::default();
    let mut count = TASK_VM_INFO_COUNT;
    let res = unsafe {
        task_info(task, TASK_VM_INFO, &mut info as *mut _ as *mut i32, &mut count)
    };

    // Clean up the port
    unsafe { mach_port_deallocate(self_port, task) };

    if res != 0 {
        return None;
    }

    Some(ProcessMemoryInfo {
        compressed: info.compressed,
    })
}
