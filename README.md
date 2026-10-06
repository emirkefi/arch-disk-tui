<div id="top" align="center">

  <img src="assets/screenshot.png" alt="arch-disk-tui screenshot" width="94%" />

  <br/><br/>

  <a href="https://github.com/emirkefi/arch-disk-tui">
    <img src="https://readme-typing-svg.demolab.com?font=Fira+Code&weight=700&size=28&duration=2800&pause=900&color=7DCFFF&center=true&vCenter=true&width=620&lines=ARCH%C2%B7DISK%C2%B7TUI;High-Performance+Storage+Analyzer;Interactive+Terminal+Heatmap;Vim-Style+Directory+Explorer" alt="Typing SVG" />
  </a>

  <p align="center">
    <strong>A high-performance, modern terminal disk space analyzer and heatmap for Linux</strong>
  </p>

  <p align="center">
    <a href="https://github.com/emirkefi/arch-disk-tui/stargazers"><img src="https://img.shields.io/github/stars/emirkefi/arch-disk-tui?style=for-the-badge&logo=star&color=ffc66d&logoColor=white" alt="Stars" /></a>
    <a href="https://github.com/emirkefi/arch-disk-tui/releases"><img src="https://img.shields.io/github/v/release/emirkefi/arch-disk-tui?style=for-the-badge&color=78e6a0" alt="Release" /></a>
    <a href="https://github.com/emirkefi/arch-disk-tui/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-7dcfff?style=for-the-badge" alt="License" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust_2024-e43717?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" /></a>
    <a href="https://archlinux.org"><img src="https://img.shields.io/badge/Arch_Linux-1793D1?style=for-the-badge&logo=archlinux&logoColor=white" alt="Arch Linux" /></a>
  </p>

  <p align="center">
    <a href="#quick-install"><strong>Quick Install</strong></a> &nbsp;&bull;&nbsp;
    <a href="#features"><strong>Features</strong></a> &nbsp;&bull;&nbsp;
    <a href="#controls"><strong>Controls</strong></a> &nbsp;&bull;&nbsp;
    <a href="#usage"><strong>Usage</strong></a> &nbsp;&bull;&nbsp;
    <a href="#uninstall"><strong>Uninstall</strong></a>
  </p>

</div>

---

<a id="arch-disk-tui"></a>
## Overview

**`arch-disk-tui`** is a high-performance terminal disk analyzer engineered in Rust. Inspired by the speed of *WizTree* and the visual clarity of modern Unix utilities, it delivers real-time non-blocking directory scanning, live partition health telemetry, Vim-style hierarchical navigation, and proportional colored distribution heatmaps.

---

<a id="features"></a>
## Features

<table>
  <tr>
    <td width="50%">
      <h3>High Performance & Non-Blocking</h3>
      <ul>
        <li>Multi-threaded traversal powered by <code>jwalk</code></li>
        <li>Indexes <strong>500,000+ files in seconds</strong></li>
        <li>UI stays responsive and smooth while scanning in the background</li>
      </ul>
    </td>
    <td width="50%">
      <h3>Modern Terminal UI</h3>
      <ul>
        <li>Crafted with Ratatui featuring clean rounded borders</li>
        <li>Arch Ice Blue, Neon Pink, and Mint accents</li>
        <li>Proportional colored distribution heatmaps</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td width="50%">
      <h3>Interactive Exploration</h3>
      <ul>
        <li>Vim keys (<code>h</code>, <code>j</code>, <code>k</code>, <code>l</code>) or standard arrow keys</li>
        <li>Instant drill-down into subdirectories and single-key navigation back</li>
        <li>Real-time fuzzy search and filtering (<code>/</code>)</li>
      </ul>
    </td>
    <td width="50%">
      <h3>Virtual Filesystem Filtering</h3>
      <ul>
        <li>Automatically skips pseudo-filesystems (<code>/proc</code>, <code>/sys</code>, <code>/dev</code>, <code>/run</code>)</li>
        <li>Protection against infinite recursion loops</li>
        <li>Eliminates phantom multi-terabyte virtual allocation calculations</li>
      </ul>
    </td>
  </tr>
</table>

---

<a id="quick-install"></a>
## Quick Install

### Automated Installer

```bash
curl -fsSL https://raw.githubusercontent.com/emirkefi/arch-disk-tui/main/install.sh | bash
```

<details>
<summary><strong>Alternative Installation Methods (Cargo, Source, Make)</strong></summary>
<br/>

#### Option A: Build from Source

```bash
# 1. Clone the repository
git clone https://github.com/emirkefi/arch-disk-tui.git
cd arch-disk-tui

# 2. Build release binary
cargo build --release

# 3. Run
./target/release/arch-disk-tui
```

#### Option B: Direct Cargo Install

```bash
cargo install --path .
```

#### Option C: Standard Makefile

```bash
make
sudo make install
```

</details>

---

<a id="controls"></a>
<a id="keybindings"></a>
<a id="interactive-keybindings"></a>
## Interactive Keybindings

| Shortcut | Action | Description |
|:---:|:---|:---|
| <kbd>j</kbd> / <kbd>&darr;</kbd> | Move Selection Down | Navigate current directory items |
| <kbd>k</kbd> / <kbd>&uarr;</kbd> | Move Selection Up | Navigate current directory items |
| <kbd>l</kbd> / <kbd>Enter</kbd> | Drill Down | Enter highlighted directory |
| <kbd>h</kbd> / <kbd>Backspace</kbd> | Navigate Back | Return to parent directory |
| <kbd>s</kbd> | Cycle Sort Order | Switch between **Size (descending)**, **Name (alphabetical)**, and **Count** |
| <kbd>/</kbd> | Search Filter | Instant real-time regex/name search |
| <kbd>d</kbd> / <kbd>Del</kbd> | Safe Delete | Delete highlighted file or folder (with confirmation modal) |
| <kbd>Esc</kbd> | Dismiss | Clear search query or close dialogs |
| <kbd>r</kbd> | Refresh Drives | Poll live filesystem storage metrics |
| <kbd>?</kbd> | Cheatsheet | Open interactive modal help window |
| <kbd>q</kbd> | Safe Exit | Restore terminal state and exit |

---

<a id="safe-deletion"></a>
## Safe File & Folder Deletion

`arch-disk-tui` allows users to clean up disk space directly from the tool with multi-layered safety guards:

- **Always Confirms**: Every single deletion requires explicit confirmation via an interactive modal displaying item type, full path, file size, and subdirectory/file counts.
- **System File Protection**: Core system hierarchies (`/etc`, `/usr`, `/var`, `/boot`, `/bin`, `/lib`, `/opt`, `/root`, etc.) and system/root-owned files cannot be deleted by regular users.
- **Personal Scope**: Standard users can only delete their own personal, downloaded, or user-created files (e.g. `~/Downloads`, `~/Projects`, external drives, user files).
- **Superuser Support**: When connected with superuser privileges (e.g. `sudo arch-disk-tui`), system items can be deleted, accompanied by high-visibility caution alerts and warnings.
- **Root Shield**: The filesystem root (`/`) and active scan root cannot be deleted under any circumstances.

---

<a id="usage"></a>
## Usage Examples

```bash
# Scan current directory
arch-disk-tui

# Scan root partition
arch-disk-tui /

# Scan home folder
arch-disk-tui ~

# Check build help
arch-disk-tui --help
```

---

<a id="uninstall"></a>
## Uninstall

Cleanly remove `arch-disk-tui` with one command:

```bash
# Using the automated uninstaller
./uninstall.sh

# Or via remote curl
curl -fsSL https://raw.githubusercontent.com/emirkefi/arch-disk-tui/main/uninstall.sh | bash
```

Or via Makefile:

```bash
sudo make uninstall
```

---

<div align="center">

  <p>Maintained for Arch Linux and Rust enthusiasts by <a href="https://github.com/emirkefi"><strong>Emir</strong></a></p>

  <a href="#top">&uarr; Back to Top</a>

</div>
