# 7EMPEST MONITOR ⚡️ [![License: MIT + Commons Clause](https://img.shields.io/badge/License-MIT%20+%20Commons%20Clause-blue.svg)](LICENSE)

A stunning, real-time terminal system monitor (TUI) for macOS, Windows and Linux, built with Rust.

## Gallery

![Overview](README_assets/screenshots/overview.png)
![CPU](README_assets/screenshots/cpu.png)
![Memory](README_assets/screenshots/memory.png)
![Disks](README_assets/screenshots/disks.png)
![Network](README_assets/screenshots/network.png)
![Processes](README_assets/screenshots/processes.png)
![GPU](README_assets/screenshots/gpu.png)
![Services](README_assets/screenshots/services.png)
![Sockets](README_assets/screenshots/sockets.png)

## Features

- 📊 **Real-time Overview**: Instant view of your system's health with custom themed layouts.
- 🎨 **Dynamic Theme Presets**: Switch themes dynamically on-the-fly (`T`/`t` keys). Options include Dark, Light, Nord, Catppuccin, Dracula, Gruvbox, and Tokyo Night.
- 💻 **CPU Monitoring**: Per-core history, frequency, and thermal mapping (absolute range-scaled logic).
- 🧠 **Memory Tracking**: RAM and SWAP usage visualised via a detailed, customizable `htop`-style stacked bar (Active/Apps, Wired/Buffers, Cache, and Free) with a color-coded legend and compressed memory reporting on macOS.
- 📂 **Disk I/O**: Live monitoring of disk read/write activities.
- 🌐 **Network traffic**: History of received and transmitted data across all interfaces (MAC, Speed, Driver).
- 💿 **Socket Connections**: Native high-performance socket listing (Protocols, Local/Remote IPs, Connection States).
- ⚙️ **Process Management**:
  - Sort by CPU, Memory, PID, Name, Disk I/O, or Virtual Memory.
  - Interactive **Signal Menu** (SIGTERM, SIGKILL, etc.).
  - **Tree View** mode to visualize parent-child relationships.
  - Powerful **Regex Filtering** to find exactly what you need.
- 🎯 **Focus Mode**: Zero-in on a single process with dedicated high-res time-series charts.
- 🔔 **Intelligent Alerting**: Desktop notifications for customizable threshold breaches (CPU, RAM, etc.).
- 💾 **Historical Persistence**: 7-day rolling window of system metrics stored in a local SQLite database.
- 📈 **Observability**: Prometheus-compatible exporter and PNG/JSON machine-state snapshots.
- 🚀 **Full Async Engine**: Decoupled UI and data collection powered by `tokio` for perfect responsiveness.
- 🛠️ **Upgraded CLI & Configuration**: Full environment-variable mapping, strict value validations, and automatic typo suggestions powered by `clap`.
- 🧬 **Robust Test Coverage**: An automated integration test suite checking CLI, configuration saving/loading, theme styling, and alerting rules engine.

## 🔋 Battery & Hardware Monitoring

- **Battery Status**: Powered by the modern, maintained `starship-battery` crate (upgraded from the abandoned `battery` crate) to ensure robust cross-platform battery telemetry including percentage, charging state, and remaining duration.
- **Thermal Sensors**: Monitors temperatures across various system components.
- **GPU Monitoring**: Dedicated tab for GPU utilization, clock speeds, and power draw.
    - **macOS (Apple Silicon)**: High-fidelity metrics via `powermetrics` (requires sudo) with a **reliable fallback** to `ioreg` (no sudo required). Optimized for **M4** with support for **ANE Power**, **GPU Frequency (MHz)**, and **Unified Memory** utilization.
    - **Linux (AMD/Intel)**: Temperature, GPU clock, VRAM usage, and GPU busy % via `sysfs` / `hwmon`.
    - **Linux (NVIDIA)**: Professional monitoring via `NVML`.

## 🎨 Dynamic Themes

Tempest Monitor features multiple beautiful built-in color themes to match your terminal setup:
- **Dark / Catppuccin (Mocha)** (Default)
- **Light**
- **Nord**
- **Dracula**
- **Gruvbox**
- **Tokyo Night**

Cycle through themes instantly by pressing `T` on any tab (or `t` on any tab except Processes). Your selected theme is automatically saved to your configuration file (`~/.config/tempest-monitor/config.yaml`) and persisted across sessions.

## 🛡️ High-Privilege Monitoring

Some advanced features require elevated privileges to access hardware statistics:
- **GPU Utilization**: On macOS, `powermetrics` requires `sudo`.
- **System Services**: On macOS, managing `launchctl` services requires `sudo`. On Linux, `systemctl` services are listed automatically (user + system).
- **Sockets/Processes**: Full process metadata (compressed memory) requires `sudo`.

To run without typing your password every time, you can add this to your `/etc/sudoers` (using `visudo`):
```text
your_username ALL=(ALL) NOPASSWD: /path/to/tempest-monitor
```

## Usage Guide & Tabs

Tempest Monitor is designed for both speed and depth. Press `1`-`0` or use `Tab`/`Shift+Tab` to cycle.

- `1`: **Overview** - High-level dashboard of everything at once.
- `2`: **CPU** - Detailed per-core usage, frequency, and thermal mapping.
- `3`: **Memory** - Deep dive into RAM, SWAP, and macOS **Compressed Memory**.
- `4`: **Disks** - Live monitoring of all mounted volumes and I/O.
- `5`: **Network** - Per-interface traffic stats and interface info (MAC, Speed, Duplex).
- `6`: **Processes** - The interactive task manager. Hit `Enter` for Focus Mode or `k` for Signal Menu.
- `7`: **GPU** - Real-time utilization and power consumption charts.
- `8`: **Services** - Interactive system service manager: macOS `launchctl` and Linux `systemd`. Press `Enter` to open the Service Inspector to read plist/unit files, toggle live log streaming, auto-detect and edit its config file using `$EDITOR` with automatic TUI suspend/resume, or start/stop/restart.
- `9`: **Sockets** - Real-time network socket enumeration (replacing `netstat`).
- `0`: **History** - Rolling time-series charts displaying SQLite-backed long-term metric history.

## Controls

| Key | Action |
|-----|--------|
| `1`-`0` | Switch between tabs |
| `Tab` / `Shift+Tab` | Cycle through tabs |
| `Enter` | **Focus Mode** (Processes) / Start/Restart (Services) / Open Inspector (Services list) |
| `s` / `r` | Stop / Restart service (Services list / Inspector view) |
| `e` | Edit service plist/unit file (Inspector view - suspends TUI) |
| `c` | Edit auto-detected service configuration file (Inspector view - suspends TUI) |
| `l` | Toggle service log viewer pane (Inspector view) |
| `Esc` | Return to services list (Inspector view) |
| `F7` / `F8` / `F9` | Toggle CPU Performance Modes: Low Power / Normal / Performance (CPU tab only) |
| `q` / `Ctrl+C` | Quit |
| `?` | Toggle help menu |
| `Space` | Pause/Resume refreshing |
| `+` / `-` | Increase/Decrease refresh rate |
| `j` / `k` (or arrows) | Navigate lists |
| `/` | Start filtering processes |
| `r` | Toggle Regex mode for filtering |
| `t` | Toggle Tree View (Processes tab only) |
| `T` (or `t` outside Processes) | Cycle color themes (Dark, Light, Nord, Catppuccin, Dracula, Gruvbox, Tokyo Night) |
| `d` | Toggle detailed process panel |
| `k` | Open Signal Menu for selected process |
| `F1`-`F6` | Quick sort options (Processes tab only) |

## Installation

### From GitHub Releases (Recommended)
Download the pre-compiled binary for your architecture from the [GitHub Actions artifacts](https://github.com/7empest462/tempest-monitor/actions) or the Releases page.

### From Source
Ensure you have [Rust](https://rustup.rs/) installed.

#### Linux Build Dependencies

Some Linux distributions require additional development libraries before compiling:

| Distro | Install Command |
|--------|----------------|
| **Debian / Ubuntu** | `sudo apt install libfontconfig1-dev libssl-dev pkg-config` |
| **Fedora / RHEL** | `sudo dnf install fontconfig-devel openssl-devel pkg-config` |
| **Arch Linux** | `sudo pacman -S fontconfig openssl pkgconf` |
| **SteamOS (Steam Deck)** | `sudo steamos-readonly disable && sudo pacman -S fontconfig openssl pkgconf` |

```bash
git clone https://github.com/7empest462/tempest-monitor.git
cd tempest-monitor
cargo build --release
cp target/release/tempest-monitor ~/.local/bin/  # Move to PATH
```

## 📦 Using as a Library Dependency

In addition to being a TUI application, `tempest-monitor` is also a reusable library (`tempest_monitor`) designed to let developers easily collect high-fidelity, cross-platform hardware telemetry in their own Rust projects.

To use it, add this to your `Cargo.toml`:
```toml
[dependencies]
tempest-monitor = "0.4.7"
```

### Telemetry Capabilities:
* **macOS (Apple Silicon)**: Programmatically query M1/M2/M3/M4 metrics including:
  * CPU / GPU / ANE (Apple Neural Engine) power draw in milliwatts.
  * GPU Core frequency (MHz).
  * Unified Memory utilization.
  * Compressed memory statistics per process.
* **Linux**:
  * AMD/Intel GPU clock, VRAM, and temperature via `sysfs`.
  * NVIDIA GPU stats via NVML bindings.
  * CPU governor controls and frequency telemetry.
* **Cross-platform**: Sockets, disk statistics, network interface speed, and system service statuses.

### Library API Example:
```rust
use tempest_monitor::{collect_macos_gpu, TelemetrySnapshot};

fn main() {
    #[cfg(target_os = "macos")]
    {
        // Fetch macOS high-fidelity GPU & Apple Silicon power statistics
        let gpu_stats = collect_macos_gpu(true); // true = use powermetrics (requires sudo)
        if let Some(power) = gpu_stats.power_mw {
            println!("GPU Power Draw: {:.2} Watts", power / 1000.0);
        }
    }
}
```

## Automations

This project uses **GitHub Actions** to automatically build binaries for both macOS and Linux on every push.

## License

This project is licensed under the **MIT License + Commons Clause 1.0**. 
- **Free for Personal/Internal Use**: You are free to use, modify, and distribute the software for free.
- **No Commercial Resale**: You may **not** sell the software or provide it as a paid service whose value derives substantially from this software.

See the [LICENSE](LICENSE) file for details.

---
