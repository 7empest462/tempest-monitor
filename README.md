# Tempest Monitor ⚡️

A stunning, real-time terminal system monitor (TUI) for macOS and Linux, built with Rust.

![Tempest Monitor Mockup](https://github.com/7empest462/tempest-monitor/blob/master/README_assets/tempest_monitor_mockup.png?raw=true)

## Features

- 📊 **Real-time Overview**: Instant view of your system's health.
- 💻 **CPU Monitoring**: Per-core history and global usage sparklines.
- 🧠 **Memory Tracking**: RAM and SWAP usage with detailed breakdowns. Includes **compressed memory** reporting for macOS.
- 📂 **Disk I/O**: Live monitoring of disk read/write activities.
- 🌐 **Network traffic**: History of received and transmitted data across all interfaces.
- 🔋 **Battery Status**: Percent, state, and time remaining (if applicable).
- 🛠 **Process Management**:
  - Sort by CPU, Memory, PID, Name, Disk I/O, or Virtual Memory.
  - Interactive **Signal Menu** (SIGTERM, SIGKILL, etc.).
  - **Tree View** mode to visualize parent-child relationships.
  - Powerful **Regex Filtering** to find exactly what you need.
- 🚀 **Cross-Platform**: Fully supports macOS (Apple Silicon native) and Linux (via statically linked musl binaries).

## Controls

| Key | Action |
|-----|--------|
| `1`-`6` | Switch between tabs (Overview, CPU, Mem, Disks, Net, Proc) |
| `Tab` / `Shift+Tab` | Cycle through tabs |
| `q` / `Ctrl+C` | Quit |
| `?` | Toggle help menu |
| `Space` | Pause/Resume refreshing |
| `+` / `-` | Increase/Decrease refresh rate |
| `j` / `k` (or arrows) | Navigate process list |
| `/` | Start filtering processes |
| `r` | Toggle Regex mode for filtering |
| `t` | Toggle Tree View |
| `d` | Toggle detailed process panel |
| `k` | Open Signal Menu for selected process |
| `F1`-`F6` | Quick sort options |

## Installation

### From GitHub Releases (Recommended)
Download the pre-compiled binary for your architecture from the [GitHub Actions artifacts](https://github.com/7empest462/tempest-monitor/actions) or the Releases page.

### From Source
Ensure you have [Rust](https://rustup.rs/) installed.

```bash
git clone https://github.com/7empest462/tempest-monitor.git
cd tempest-monitor
cargo build --release
./target/release/tempest-monitor
```

## Automations

This project uses **GitHub Actions** to automatically build binaries for both macOS and Linux on every push. You can find the latest builds in the "Actions" tab of this repository.

---
Built with ❤️ for 7empest.
