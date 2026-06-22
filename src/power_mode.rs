//! CPU Performance Mode detection and control.
//!
//! **macOS**: Uses `pmset` to toggle Low Power Mode.
//! **Linux**: Uses sysfs CPU frequency governors.

/// Available CPU power modes.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CpuPowerMode {
    LowPower,
    Normal,      // macOS default / Linux ondemand
    Balanced,    // Linux: schedutil
    Performance, // Linux only
    Unknown,
}

impl CpuPowerMode {
    pub fn label(self) -> &'static str {
        match self {
            CpuPowerMode::LowPower => "Low Power",
            CpuPowerMode::Normal => "Normal",
            CpuPowerMode::Balanced => "Balanced",
            CpuPowerMode::Performance => "Performance",
            CpuPowerMode::Unknown => "Unknown",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            CpuPowerMode::LowPower => "🔋",
            CpuPowerMode::Normal => "⚡",
            CpuPowerMode::Balanced => "⚖️ ",
            CpuPowerMode::Performance => "🚀",
            CpuPowerMode::Unknown => "❓",
        }
    }
}

/// Returns the list of modes available on the current platform.
pub fn available_modes() -> Vec<CpuPowerMode> {
    cfg_select! {
        target_os = "macos" => {
            vec![CpuPowerMode::LowPower, CpuPowerMode::Normal]
        },
        target_os = "linux" => {
            let available = std::fs::read_to_string(
                "/sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors"
            ).unwrap_or_default();

            let mut modes = Vec::new();
            if available.contains("powersave") {
                modes.push(CpuPowerMode::LowPower);
            }
            if available.contains("schedutil") || available.contains("ondemand") {
                modes.push(CpuPowerMode::Balanced);
            }
            if available.contains("performance") {
                modes.push(CpuPowerMode::Performance);
            }
            if modes.is_empty() {
                modes.push(CpuPowerMode::Unknown);
            }
            modes
        },
        target_os = "windows" => {
            // Windows always offers these three; Ultimate Performance may not be present on Home
            vec![CpuPowerMode::LowPower, CpuPowerMode::Balanced, CpuPowerMode::Performance]
        },
        _ => {
            vec![CpuPowerMode::Unknown]
        }
    }
}

/// Detect the current CPU power mode.
#[cfg(target_os = "macos")]
pub fn detect_power_mode() -> CpuPowerMode {
    if let Ok(output) = std::process::Command::new("pmset").arg("-g").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("lowpowermode") {
                if trimmed.ends_with('1') {
                    return CpuPowerMode::LowPower;
                } else {
                    return CpuPowerMode::Normal;
                }
            }
        }
    }
    CpuPowerMode::Unknown
}

#[cfg(target_os = "linux")]
pub fn detect_power_mode() -> CpuPowerMode {
    let governor = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        .unwrap_or_default();

    match governor.trim() {
        "powersave" => CpuPowerMode::LowPower,
        "schedutil" => CpuPowerMode::Balanced,
        "ondemand" => CpuPowerMode::Balanced,
        "conservative" => CpuPowerMode::Balanced,
        "performance" => CpuPowerMode::Performance,
        _ => CpuPowerMode::Unknown,
    }
}

/// Windows: parse `powercfg /getactivescheme` to detect current power plan.
#[cfg(windows)]
pub fn detect_power_mode() -> CpuPowerMode {
    if let Ok(output) = std::process::Command::new("powercfg")
        .args(["/getactivescheme"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        // Match well-known GUID names embedded in the output
        if stdout.contains("power saver") || stdout.contains("powersaver") {
            return CpuPowerMode::LowPower;
        } else if stdout.contains("high performance") || stdout.contains("ultimate performance") {
            return CpuPowerMode::Performance;
        } else if stdout.contains("balanced") {
            return CpuPowerMode::Balanced;
        }
    }
    CpuPowerMode::Unknown
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub fn detect_power_mode() -> CpuPowerMode {
    CpuPowerMode::Unknown
}

/// Set the CPU power mode. Returns Ok(success_message) or Err(error_message).
#[cfg(target_os = "macos")]
pub fn set_power_mode(mode: CpuPowerMode) -> Result<String, String> {
    let value = match mode {
        CpuPowerMode::LowPower => "1",
        CpuPowerMode::Normal => "0",
        _ => return Err("Unsupported mode on macOS".into()),
    };

    // Try non-interactive sudo first
    let result = std::process::Command::new("sudo")
        .args(["-n", "pmset", "-a", "lowpowermode", value])
        .status();

    match result {
        Ok(status) if status.success() => Ok(format!("✓ Switched to {} mode", mode.label())),
        Ok(_) => {
            // Non-interactive failed, try with potential password prompt
            let result2 = std::process::Command::new("sudo")
                .args(["pmset", "-a", "lowpowermode", value])
                .status();
            match result2 {
                Ok(s) if s.success() => Ok(format!("✓ Switched to {} mode", mode.label())),
                Ok(s) => Err(format!(
                    "✗ pmset exited with code {}",
                    s.code().unwrap_or(-1)
                )),
                Err(e) => Err(format!("✗ Failed to run pmset: {}", e)),
            }
        }
        Err(e) => Err(format!("✗ Failed to run sudo: {}", e)),
    }
}

#[cfg(target_os = "linux")]
pub fn set_power_mode(mode: CpuPowerMode) -> Result<String, String> {
    let governor = match mode {
        CpuPowerMode::LowPower => "powersave",
        CpuPowerMode::Balanced => {
            // Prefer schedutil if available, fallback to ondemand
            let avail = std::fs::read_to_string(
                "/sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors",
            )
            .unwrap_or_default();
            if avail.contains("schedutil") {
                "schedutil"
            } else {
                "ondemand"
            }
        }
        CpuPowerMode::Performance => "performance",
        _ => return Err("Unsupported mode".into()),
    };

    // Find all CPU cores and set governor for each
    let cpu_dir = std::path::Path::new("/sys/devices/system/cpu");
    let mut success_count = 0;
    let mut fail_count = 0;

    if let Ok(entries) = std::fs::read_dir(cpu_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                let gov_path = entry.path().join("cpufreq/scaling_governor");
                if gov_path.exists() {
                    // Use sudo tee to write
                    let result = std::process::Command::new("sudo")
                        .args(["-n", "tee", &gov_path.to_string_lossy()])
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::null())
                        .spawn()
                        .and_then(|mut child| {
                            use std::io::Write;
                            if let Some(ref mut stdin) = child.stdin {
                                stdin.write_all(governor.as_bytes())?;
                            }
                            child.wait()
                        });

                    match result {
                        Ok(s) if s.success() => success_count += 1,
                        _ => fail_count += 1,
                    }
                }
            }
        }
    }

    if fail_count == 0 && success_count > 0 {
        Ok(format!(
            "✓ Set {} cores to {} ({})",
            success_count,
            mode.label(),
            governor
        ))
    } else if success_count > 0 {
        Ok(format!(
            "⚠ Set {}/{} cores (some failed, may need sudo)",
            success_count,
            success_count + fail_count
        ))
    } else {
        Err("✗ Could not set governor on any core (sudo required)".into())
    }
}

/// Windows: switch power plans via `powercfg /setactive <GUID>`.
/// Requires admin for some plans; non-admin may work for Balanced/Power Saver.
#[cfg(windows)]
pub fn set_power_mode(mode: CpuPowerMode) -> Result<String, String> {
    // Well-known Windows power plan GUIDs
    let guid = match mode {
        CpuPowerMode::LowPower => "a1841308-3541-4fab-bc81-f71556f20b4a", // Power Saver
        CpuPowerMode::Balanced => "381b4222-f694-41f0-9685-ff5bb260df2e", // Balanced
        CpuPowerMode::Performance => "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c", // High Performance
        _ => return Err(format!("Mode '{}' not supported on Windows", mode.label())),
    };

    match std::process::Command::new("powercfg")
        .args(["/setactive", guid])
        .status()
    {
        Ok(s) if s.success() => Ok(format!("✓ Switched to {} ({})", mode.label(), guid)),
        Ok(s) => Err(format!(
            "✗ powercfg exited with code {}",
            s.code().unwrap_or(-1)
        )),
        Err(e) => Err(format!("✗ Failed to run powercfg: {}", e)),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub fn set_power_mode(_mode: CpuPowerMode) -> Result<String, String> {
    Err("CPU performance mode control not supported on this platform".into())
}

/// Get the raw governor string (Linux) or mode description for display.
pub fn get_mode_detail() -> String {
    cfg_select! {
        target_os = "macos" => {
            let mode = detect_power_mode();
            match mode {
                CpuPowerMode::LowPower => "lowpowermode=1 (pmset)".into(),
                CpuPowerMode::Normal   => "lowpowermode=0 (pmset)".into(),
                _ => "unknown".into(),
            }
        },
        target_os = "linux" => {
            std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "unavailable".into())
        },
        target_os = "windows" => {
            // Parse current scheme name from powercfg output
            if let Ok(output) = std::process::Command::new("powercfg")
                .args(["/getactivescheme"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Output looks like: "Power Scheme GUID: <guid>  (<name>)"
                if let Some(start) = stdout.find('(') {
                    if let Some(end) = stdout.rfind(')') {
                        return stdout[start + 1..end].trim().to_string();
                    }
                }
            }
            "unknown".into()
        },
        _ => {
            "unsupported platform".into()
        }
    }
}
