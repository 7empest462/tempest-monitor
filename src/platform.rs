//! Platform abstraction layer for tempest-monitor
//! All platform-specific code (Unix vs Windows) should live here.

use sysinfo::Pid;

// ================================================================
// SHARED TYPES (must be defined before platform modules)
// ================================================================

#[derive(Copy, Clone, Debug)]
pub struct SignalInfo {
    pub name: &'static str,
    pub number: i32,
}

// ================================================================
// UNIX SECTION (macOS + Linux)
// ================================================================

#[cfg(unix)]
mod unix_impl {
    use super::SignalInfo;

    pub const SIGNALS: [SignalInfo; 7] = [
        SignalInfo { name: "SIGTERM", number: libc::SIGTERM },
        SignalInfo { name: "SIGKILL", number: libc::SIGKILL },
        SignalInfo { name: "SIGSTOP", number: libc::SIGSTOP },
        SignalInfo { name: "SIGCONT", number: libc::SIGCONT },
        SignalInfo { name: "SIGHUP",  number: libc::SIGHUP },
        SignalInfo { name: "SIGUSR1", number: libc::SIGUSR1 },
        SignalInfo { name: "SIGUSR2", number: libc::SIGUSR2 },
    ];

    pub fn is_running_as_admin() -> bool {
        unsafe { libc::getuid() == 0 }
    }

    pub fn get_current_uid() -> u32 {
        unsafe { libc::getuid() }
    }

    pub fn kill_process(pid: sysinfo::Pid, signal_index: usize) {
        let sig = SIGNALS[signal_index].number;
        unsafe {
            let _ = libc::kill(pid.as_u32() as i32, sig);
        }
    }
}

// ================================================================
// WINDOWS SECTION
// ================================================================

#[cfg(windows)]
mod windows_impl {
    use super::SignalInfo;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    // Windows doesn't have Unix signals — one meaningful action: terminate
    pub const SIGNALS: [SignalInfo; 1] = [
        SignalInfo { name: "Terminate", number: 0 },
    ];

    pub fn is_running_as_admin() -> bool {
        use windows::Win32::Security::{
            CheckTokenMembership, AllocateAndInitializeSid, FreeSid,
            SECURITY_NT_AUTHORITY,
        };
        use windows::Win32::Foundation::BOOL;
        use windows::core::PSID;

        unsafe {
            let nt_authority = windows::Win32::Security::SID_IDENTIFIER_AUTHORITY {
                Value: SECURITY_NT_AUTHORITY.Value,
            };
            let mut admin_group: PSID = PSID::default();
            let ok = AllocateAndInitializeSid(
                &nt_authority,
                2,   // two sub-authorities
                32,  // SECURITY_BUILTIN_DOMAIN_RID
                544, // DOMAIN_ALIAS_RID_ADMINS
                0, 0, 0, 0, 0, 0,
                &mut admin_group,
            );
            if ok.is_err() {
                return false;
            }

            let mut is_member = BOOL::default();
            let result = CheckTokenMembership(None, admin_group, &mut is_member);
            let _ = FreeSid(admin_group);

            result.is_ok() && is_member.as_bool()
        }
    }

    pub fn get_current_uid() -> u32 {
        // Windows has no Unix UIDs — synthesize: 0 if admin, 1000 otherwise
        if is_running_as_admin() { 0 } else { 1000 }
    }

    pub fn kill_process(pid: sysinfo::Pid, _signal_index: usize) {
        // All Windows "signals" map to TerminateProcess
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid.as_u32()) {
                let _ = TerminateProcess(handle, 1);
            }
        }
    }
}

// ================================================================
// PUBLIC API (Unified Interface)
// ================================================================

/// Returns the list of signals (or process actions) available on the current platform.
pub fn get_signals() -> &'static [SignalInfo] {
    #[cfg(unix)]
    { &unix_impl::SIGNALS }

    #[cfg(windows)]
    { &windows_impl::SIGNALS }
}

/// Terminate / send a signal to a process by PID and signal list index.
pub fn kill_process(pid: Pid, signal_index: usize) {
    #[cfg(unix)]
    { unix_impl::kill_process(pid, signal_index); }

    #[cfg(windows)]
    { windows_impl::kill_process(pid, signal_index); }
}

/// Check if the current user is running with elevated privileges (root on Unix, Administrator on Windows).
pub fn is_running_as_admin() -> bool {
    #[cfg(unix)]
    { unix_impl::is_running_as_admin() }

    #[cfg(windows)]
    { windows_impl::is_running_as_admin() }

    #[cfg(not(any(unix, windows)))]
    { false }
}

/// Get the current user ID.
/// On Unix this is the real UID. On Windows it returns 0 for admins, 1000 otherwise.
pub fn get_current_uid() -> u32 {
    #[cfg(unix)]
    { unix_impl::get_current_uid() }

    #[cfg(windows)]
    { windows_impl::get_current_uid() }

    #[cfg(not(any(unix, windows)))]
    { 1000 }
}

// Re-export shared types from system_helper for convenience
pub use crate::system_helper::{
    get_services,
    get_memory_segments,
    get_sockets,
    ServiceEntry,
    SocketEntry,
};