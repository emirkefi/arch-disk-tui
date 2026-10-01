# arch-disk-tui

<div align="center">

  <h1>⚡ arch-disk-tui</h1>
  <p><strong>A blazingly fast, modern, and aesthetic Terminal Disk Space Analyzer & Treemap for Linux / Arch Linux</strong></p>

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
  [![Rust](https://img.shields.io/badge/Language-Rust_2024-orange.svg)](https://www.rust-lang.org/)
  [![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Arch%20Linux-1793d1.svg)](https://archlinux.org)
  [![CI](https://github.com/emirkefi/arch-disk-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/emirkefi/arch-disk-tui/actions)

</div>

---

## ✨ Features

- ⚡ **Blazingly Fast Asynchronous Scanning**: Multi-threaded, non-blocking disk traversal powered by `jwalk`. UI never freezes or stutters while indexing millions of files.
- 🎨 **Next-Level Aesthetic UI**: Styled with modern rounded borders, Arch Ice Blue theme accents, and vibrant color palettes powered by `ratatui`.
- 📊 **Partition Health & Hardware Gauges**: Real-time partition overview displaying mounted filesystems, total/used capacities, and color-coded utilization meters.
- 📁 **Interactive Hierarchical Navigation**:
  - Drill down into folders (`Enter` or `l`)
  - Jump back to parent directories (`Backspace` or `h`)
  - Intuitive Vim-style keys (`j`/`k`/`h`/`l`) and arrow keys
- 📦 **Proportional Heatmap Blocks**: Proportional block visualization highlighting the top space hogs at a glance.
- 🔍 **Live Search & Filter**: Instant file/folder fuzzy matching (`/`) to quickly spot deep bloated directories.
- 🔃 **Multi-Mode Sorting**: Sort instantly by size, file count, or name (`s`).
- 🛡️ **Virtual FS Protection**: Automatically prevents scan cycles and 128TB ghost allocations by cleanly bypassing Linux pseudo-filesystems (`/proc`, `/sys`, `/dev`, `/run`).

---

## 📸 Interface Preview

```text
╭── 󰣇 ARCH·DISK·TUI v0.1 ─╮╭──  Location: /home/user ──────────────╮╭────── 󰄬 Ready (38,421 scanned) ─╮
╰─────────────────────────╯╰────────────────────────────────────────╯╰──────────────────────────────────╯
╭─   Disks & Partitions Health ────────────────────────────────────────────────────────────────────────╮
│   /dev/nvme0n1p2 (btrfs) on /          [━━━━━━━━━━━━───]  72.4% (362.00 GB / 500.00 GB)             │
│   /dev/nvme0n1p1 (vfat ) on /boot      [━━─────────────]  14.2% (145.00 MB / 1.00 GB)               │
╰───────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭─  Explorer [32.40 GB] - Sort: [Size 󰄼] ──╮╭─ 󱁤 Target Inspection ────────────────────────────────────╮
│ ❯  .local               14.20 GB  43.8%  ││ Name:      .local (Directory)                            │
│    .cargo                8.15 GB  25.1%  ││ Full Path: /home/user/.local                             │
│    Projects              5.30 GB  16.3%  ││ Disk Size: 14.20 GB (43.8% of current scope)             │
│    .cache                3.10 GB   9.5%  ││ Contains:  412 subdirs, 12,840 files                     │
│    big_backup.tar.gz     1.10 GB   3.4%  │╰──────────────────────────────────────────────────────────╯
│    Downloads             550.0 MB  1.7%  │╭─ 󰄛 Space Allocation & Heatmap ───────────────────────────╮
│                                           ││ Top Space Consumers (Proportional Heatmap Blocks):       │
│                                           ││  1 .local               14.20 GB   43.8%                 │
│                                           ││    ██████████████████████████████                        │
│                                           ││  2 .cargo                8.15 GB   25.1%                 │
│                                           ││    █████████████████                                     │
╰───────────────────────────────────────────╯╰──────────────────────────────────────────────────────────╯
```

---

## 🚀 Quick Install & Download

### Option 1: One-Line Installer Script
Clone and install directly into your `~/.local/bin` (or `/usr/local/bin`):
```bash
curl -fsSL https://raw.githubusercontent.com/emirkefi/arch-disk-tui/main/install.sh | bash
```

### Option 2: Build From Source (Cargo)
Ensure you have the latest Rust toolchain installed:
```bash
# Clone the repository
git clone https://github.com/emirkefi/arch-disk-tui.git
cd arch-disk-tui

# Build optimized release binary
cargo build --release

# Run immediately
./target/release/arch-disk-tui
```

### Option 3: Install directly with Cargo
```bash
cargo install --path .
```

### Option 4: Makefile
```bash
make
sudo make install
```

---

## 🗑️ Uninstallation

To remove `arch-disk-tui` from your system:

### Option 1: Using the Uninstall Script
```bash
# Locally
./uninstall.sh

# Or via curl
curl -fsSL https://raw.githubusercontent.com/emirkefi/arch-disk-tui/main/uninstall.sh | bash
```

### Option 2: Using Makefile
```bash
sudo make uninstall
```

### Option 3: Using Cargo (if installed via cargo)
```bash
cargo uninstall arch-disk-tui
```

---

## 🎮 Usage & Keybindings

Run the analyzer in your current directory:
```bash
arch-disk-tui
```

Analyze a specific mount point or directory (e.g. system root):
```bash
arch-disk-tui /
# Or check your home directory
arch-disk-tui ~
```

### Keyboard Cheatsheet

| Key | Action |
|:---|:---|
| <kbd>j</kbd> / <kbd>↓</kbd> | Move selection down |
| <kbd>k</kbd> / <kbd>↑</kbd> | Move selection up |
| <kbd>Enter</kbd> / <kbd>l</kbd> | Drill down into selected folder |
| <kbd>Backspace</kbd> / <kbd>h</kbd> | Jump back up to parent directory |
| <kbd>s</kbd> | Cycle sort order (Size 󰄼 / Name 󰄾 / Total Items) |
| <kbd>/</kbd> | Live filter / search file and folder names |
| <kbd>Esc</kbd> | Clear search filter or close popups |
| <kbd>r</kbd> | Refresh disk filesystem statistics |
| <kbd>?</kbd> | Open help dialog |
| <kbd>q</kbd> | Quit application |

---

## 🛠️ Development

```bash
# Check code style & compile checks
cargo check

# Run in debug mode
cargo run -- /path/to/scan

# Run tests
cargo test
```

---

## 📄 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.
