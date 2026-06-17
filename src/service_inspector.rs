use std::path::PathBuf;

/// Resolve the on-disk path of a service file from its label.
///
/// **macOS**: searches LaunchAgent/LaunchDaemon directories for `{label}.plist`.
/// **Linux**: uses `systemctl show -p FragmentPath` to get the exact unit file path.
#[cfg(target_os = "macos")]
pub fn resolve_service_file(label: &str) -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{}/Library/LaunchAgents/{}.plist", home, label),
        format!("/Library/LaunchAgents/{}.plist", label),
        format!("/Library/LaunchDaemons/{}.plist", label),
        format!("/System/Library/LaunchDaemons/{}.plist", label),
        format!("/System/Library/LaunchAgents/{}.plist", label),
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.clone());
        }
    }
    None
}

#[cfg(target_os = "linux")]
pub fn resolve_service_file(label: &str) -> Option<String> {
    let output = std::process::Command::new("systemctl")
        .args(["show", "-p", "FragmentPath", label])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output is "FragmentPath=/path/to/unit.service"
    let path = stdout.trim().strip_prefix("FragmentPath=")?;
    if path.is_empty() || !std::path::Path::new(path).exists() {
        return None;
    }
    Some(path.to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn resolve_service_file(_label: &str) -> Option<String> {
    None
}

/// Returns true if the service file is in a SIP-protected location (macOS only).
pub fn is_sip_protected(path: &str) -> bool {
    path.starts_with("/System/")
        || path.starts_with("/usr/")
        || path.starts_with("/bin/")
        || path.starts_with("/sbin/")
}

/// Read the raw contents of a service file.
pub fn read_service_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Attempt to detect a config file from service file contents.
///
/// **macOS**: parses `ProgramArguments` from the plist XML and looks for
/// arguments that look like config file paths.
/// **Linux**: parses `ExecStart=` and checks the command-line arguments.
pub fn detect_config_file(_service_file_contents: &str) -> Option<String> {
    // Common config file extensions (used on all platforms)
    let config_extensions = [
        ".conf", ".yaml", ".yml", ".json", ".toml",
        ".cfg", ".ini", ".config", ".properties", ".xml",
    ];

    #[allow(unused_mut)]
    let mut candidate_args: Vec<String> = Vec::new();

    cfg_select! {
        target_os = "macos" => {
            // Common config-specifying flags (used only in platform arms)
            let config_flags = ["--config", "--conf", "-c", "-f", "--config-file", "--settings"];

            // Parse ProgramArguments from plist XML
            let mut in_program_args = false;
            let mut in_array = false;
            let mut found_flag = false;

            for line in _service_file_contents.lines() {
                let trimmed = line.trim();
                if trimmed.contains("<key>ProgramArguments</key>") {
                    in_program_args = true;
                    continue;
                }
                if in_program_args && trimmed.contains("<array>") {
                    in_array = true;
                    continue;
                }
                if in_array && trimmed.contains("</array>") {
                    break;
                }
                if in_array {
                    // Extract string value from <string>...</string>
                    if let Some(start) = trimmed.find("<string>")
                        && let Some(end) = trimmed.find("</string>") {
                            let val = &trimmed[start + 8..end];
                            if found_flag {
                                candidate_args.push(val.to_string());
                                found_flag = false;
                            } else if config_flags.contains(&val) {
                                found_flag = true;
                            } else {
                                candidate_args.push(val.to_string());
                            }
                        }
                }
            }
        },
        target_os = "linux" => {
            let config_flags = ["--config", "--conf", "-c", "-f", "--config-file", "--settings"];

            // Parse ExecStart= from the unit file
            for line in _service_file_contents.lines() {
                let trimmed = line.trim();
                if let Some(exec_line) = trimmed.strip_prefix("ExecStart=") {
                    let mut found_flag = false;
                    for part in exec_line.split_whitespace() {
                        if found_flag {
                            candidate_args.push(part.to_string());
                            found_flag = false;
                        } else if config_flags.iter().any(|f| part == *f) {
                            found_flag = true;
                        } else if part.contains('=') {
                            // Handle --config=/path/to/file format
                            if let Some((_flag, val)) = part.split_once('=') {
                                if config_flags.iter().any(|f| part.starts_with(&format!("{}=", f))) {
                                    candidate_args.push(val.to_string());
                                }
                            }
                        } else {
                            candidate_args.push(part.to_string());
                        }
                    }
                }
            }
        },
        _ => {}
    }

    // Skip the first arg (it's the executable itself), check the rest
    // First pass: look for files with config extensions
    for arg in candidate_args.iter().skip(1) {
        let lower = arg.to_lowercase();
        if config_extensions.iter().any(|ext| lower.ends_with(ext))
            && std::path::Path::new(arg).exists() {
                return Some(arg.clone());
            }
    }

    // Second pass: look for any existing file path that isn't the executable
    for arg in candidate_args.iter().skip(1) {
        let path = PathBuf::from(arg);
        if path.is_absolute() && path.exists() && path.is_file() {
            // Skip obvious non-config files (binaries, libraries)
            let lower = arg.to_lowercase();
            if !lower.ends_with(".so")
                && !lower.ends_with(".dylib")
                && !lower.ends_with(".py")
                && !lower.ends_with(".sh")
            {
                return Some(arg.clone());
            }
        }
    }

    None
}

/// Retrieve log output for a service.
///
/// **macOS**: reads from `StandardOutPath` / `StandardErrorPath` in the plist.
/// **Linux**: runs `journalctl -u <label> --no-pager -n 100`.
#[cfg(target_os = "macos")]
pub fn get_service_logs(label: &str, service_file_contents: Option<&str>) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(contents) = service_file_contents {
        // Parse StandardOutPath and StandardErrorPath from plist
        let paths = extract_plist_paths(contents, &["StandardOutPath", "StandardErrorPath"]);

        for (key, path) in &paths {
            lines.push(format!("── {} ──", key));
            lines.push(format!("   Path: {}", path));
            lines.push(String::new());
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    // Show last 100 lines
                    let all_lines: Vec<&str> = content.lines().collect();
                    let start = all_lines.len().saturating_sub(100);
                    for line in &all_lines[start..] {
                        lines.push(format!("  {}", line));
                    }
                }
                Err(e) => {
                    lines.push(format!("  (Could not read: {})", e));
                }
            }
            lines.push(String::new());
        }

        if paths.is_empty() {
            lines.push("No StandardOutPath/StandardErrorPath found in plist.".into());
            lines.push(String::new());
        }
    }

    // Also try `log show` for the label (brief, last 50 entries)
    lines.push("── system log (last 20) ──".to_string());
    if let Ok(output) = std::process::Command::new("log")
        .args(["show", "--predicate", &format!("subsystem == '{}'", label),
               "--last", "5m", "--style", "compact"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let log_lines: Vec<&str> = stdout.lines().collect();
        let start = log_lines.len().saturating_sub(20);
        for line in &log_lines[start..] {
            lines.push(format!("  {}", line));
        }
        if log_lines.is_empty() {
            lines.push("  (no recent log entries)".into());
        }
    }

    lines
}

#[cfg(target_os = "linux")]
pub fn get_service_logs(label: &str, _service_file_contents: Option<&str>) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("── journalctl -u {} ──", label));
    lines.push(String::new());

    if let Ok(output) = std::process::Command::new("journalctl")
        .args(["-u", label, "--no-pager", "-n", "100", "--output", "short-iso"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            lines.push(format!("  {}", line));
        }
        if lines.len() <= 2 {
            lines.push("  (no journal entries found)".into());
        }
    } else {
        lines.push("  (journalctl not available)".into());
    }

    lines
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn get_service_logs(_label: &str, _service_file_contents: Option<&str>) -> Vec<String> {
    vec!["Logs not available on this platform.".into()]
}

/// Helper: extract `<key>Key</key>\n<string>Value</string>` pairs from plist XML.
#[cfg(target_os = "macos")]
fn extract_plist_paths(contents: &str, keys: &[&str]) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = contents.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        for key in keys {
            let key_tag = format!("<key>{}</key>", key);
            if trimmed == key_tag {
                // Next line should be <string>path</string>
                if let Some(next_line) = lines.get(i + 1) {
                    let next_trimmed = next_line.trim();
                    if let Some(start) = next_trimmed.find("<string>")
                        && let Some(end) = next_trimmed.find("</string>") {
                            let val = &next_trimmed[start + 8..end];
                            results.push((key.to_string(), val.to_string()));
                        }
                }
            }
        }
    }

    results
}

/// Get the preferred editor command.
/// Checks $EDITOR, then $VISUAL, then falls back to "nano".
pub fn get_editor() -> String {
    std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "nano".to_string())
}
