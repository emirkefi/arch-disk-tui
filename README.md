<div id="top" align="center">

  <img src="assets/demo.gif" alt="SpaceSage demo" width="94%" />

  <br/><br/>

  <h1>SpaceSage</h1>

  <p align="center">
    <strong>Find out what's eating your disk, and what's safe to delete.</strong>
  </p>

  <p align="center">
    <a href="https://github.com/emirkefi/spacesage/releases"><img src="https://img.shields.io/github/v/release/emirkefi/spacesage?style=for-the-badge&color=78e6a0" alt="Release" /></a>
    <a href="https://github.com/emirkefi/spacesage/actions"><img src="https://img.shields.io/github/actions/workflow/status/emirkefi/spacesage/ci.yml?style=for-the-badge&label=CI" alt="CI" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache--2.0-7dcfff?style=for-the-badge" alt="License" /></a>
    <a href="https://crates.io/crates/spacesage"><img src="https://img.shields.io/crates/d/spacesage?style=for-the-badge&logo=rust&color=0997e5" alt="crates.io downloads" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust_2024-e43717?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" /></a>
  </p>

  <p align="center">
    <a href="#install"><strong>Install</strong></a> &nbsp;&bull;&nbsp;
    <a href="#why-spacesage"><strong>Why SpaceSage?</strong></a> &nbsp;&bull;&nbsp;
    <a href="#features"><strong>Features</strong></a> &nbsp;&bull;&nbsp;
    <a href="#usage"><strong>Usage</strong></a> &nbsp;&bull;&nbsp;
    <a href="#roadmap"><strong>Roadmap</strong></a> &nbsp;&bull;&nbsp;
    <a href="#contributing"><strong>Contributing</strong></a>
  </p>

</div>

---

## Install

```bash
cargo install spacesage
```

Then run it on any folder:

```bash
spacesage ~
```

<details>
<summary><strong>Other ways to install (script, source, Arch Linux)</strong></summary>
<br/>

**Install script** (downloads a prebuilt binary, or builds from `master`):

```bash
curl -fsSL https://raw.githubusercontent.com/emirkefi/spacesage/master/install.sh | bash
```

**Build from source:**

```bash
git clone https://github.com/emirkefi/spacesage.git
cd spacesage
cargo build --release
./target/release/spacesage
```

**Uninstall:**

```bash
curl -fsSL https://raw.githubusercontent.com/emirkefi/spacesage/master/uninstall.sh | bash
# Or, if you installed with the Makefile: sudo make uninstall
```

</details>

---

## Why SpaceSage?

Most disk analyzers answer one question: *"what is big?"* Then you are left
alone with a 14 GB folder called `.cache` and no idea whether deleting it is
safe.

SpaceSage is built around the question you actually have: **"what can I get rid
of?"**

| A typical disk analyzer | SpaceSage |
|:---|:---|
| Shows sizes | Shows sizes, treemap and heatmap |
| You decide what a folder is | Explains known folders in plain language *(coming in v0.4)* |
| Delete and hope | Safety levels: safe, caution, keep *(coming in v0.4)* |
| Permanent delete | Trash first, with system-path protection |
| You hunt for big folders | A "Reclaimable" view lists the best wins *(coming in v0.5)* |

SpaceSage is not meant to replace tools like ncdu, dust or gdu. It is for people
who want to go from "my disk is full" to "I freed 20 GB" in a few keypresses.

---

<a id="features"></a>
## Features

**Available now**

- **Inline size bars and heatmap**: every row shows its share of the parent
  folder, colored by file category (media, archives, code, caches, documents,
  binaries), with Nerd Font or ASCII icons.
- **Fullscreen treemap page**: press `t` for a 2D proportional treemap that
  takes over the whole terminal. Press `t` or `Esc` to return.
- **Top-N largest files**: press `f` to see the biggest individual files in the
  whole scan.
- **Safe delete**: mark several items with `Space`, then send them to the
  FreeDesktop Trash (`d`) or permanently delete them (`D`). System files are
  protected.
- **Accurate accounting**: hard links are counted once, symlinks and
  pseudo-filesystems (`/proc`, `/sys`, `/dev`, `/run`) are skipped, and
  folders you can't read are marked `[needs sudo]`.
- **Partition gauge and breadcrumbs**: a live usage bar that turns yellow and
  red as the disk fills, and a clickable path bar.
- **Themes and scripting**: `arch`, `catppuccin`, `gruvbox` and `mono` themes,
  `NO_COLOR` support, a `config.toml`, and headless `--export-json` and
  `--export-csv`.

**Coming soon**: see the [roadmap](#roadmap).

---

<a id="usage"></a>
## Usage

```bash
# Analyze the current folder, or a specific path
spacesage
spacesage /var

# Stay on one filesystem (like du -x) and exclude patterns
spacesage / -x --exclude node_modules --exclude .git

# Choose a theme or icon mode
spacesage --theme catppuccin --icons nerd

# Export results for scripts
spacesage ~/Projects --export-json report.json
spacesage ~/Projects --export-csv report.csv
```

### Key bindings

| Key | Action |
|:---:|:---|
| <kbd>j</kbd> / <kbd>k</kbd> | Move down / up |
| <kbd>l</kbd> / <kbd>Enter</kbd> | Open folder |
| <kbd>h</kbd> / <kbd>Backspace</kbd> | Go back |
| <kbd>Space</kbd> | Mark item for batch actions |
| <kbd>d</kbd> / <kbd>D</kbd> | Move to Trash / delete permanently |
| <kbd>t</kbd> | Fullscreen treemap page |
| <kbd>?</kbd> / <kbd>q</kbd> | Help / quit |

<details>
<summary><strong>All key bindings</strong></summary>
<br/>

| Key / Mouse | Action |
|:---:|:---|
| <kbd>j</kbd> / <kbd>k</kbd> / <kbd>&uarr;</kbd> / <kbd>&darr;</kbd> / Scroll | Move selection up or down |
| <kbd>g</kbd> / <kbd>G</kbd> | Jump to top / bottom |
| <kbd>Ctrl+d</kbd> / <kbd>Ctrl+u</kbd> | Half-page down / up |
| <kbd>l</kbd> / <kbd>Enter</kbd> / Double-click | Enter folder (or locate file in Top-N view) |
| <kbd>h</kbd> / <kbd>Backspace</kbd> / Right-click | Go back (remembers cursor position) |
| <kbd>~</kbd> | Jump to `$HOME` |
| <kbd>Space</kbd> | Mark / unmark for batch actions |
| <kbd>d</kbd> | Move selected or marked items to the Trash |
| <kbd>D</kbd> / <kbd>Del</kbd> | Permanently delete selected or marked items |
| <kbd>s</kbd> / <kbd>S</kbd> | Cycle sort mode (`Size`, `Name`, `Count`) / reverse direction |
| <kbd>/</kbd> | Live search with match count |
| <kbd>t</kbd> | Fullscreen treemap page (<kbd>t</kbd> or <kbd>Esc</kbd> to leave) |
| <kbd>f</kbd> | Top-N largest files view |
| <kbd>y</kbd> / <kbd>o</kbd> | Copy path (`y`) / open in `$EDITOR` or `xdg-open` (`o`) |
| <kbd>T</kbd> / <kbd>r</kbd> | Cycle theme (`T`) / refresh partition metrics (`r`) |
| <kbd>?</kbd> / <kbd>q</kbd> | Help / quit |

</details>

### Configuration

Optional settings go in `~/.config/spacesage/config.toml`:

```toml
theme = "arch"          # "arch", "catppuccin", "gruvbox", "mono", "no_color"
icons = "auto"          # "auto", "nerd", "ascii", "none"
one_file_system = false
exclude = ["node_modules", ".git"]
```

---

## Safety

SpaceSage is careful by default, because deleting files is not reversible:

- `d` moves items to the FreeDesktop Trash, so you can restore them.
- Permanent delete (`D`) always asks for confirmation.
- System paths are protected from deletion for regular users.
- Nothing is ever deleted during a scan.

---

<a id="roadmap"></a>
## Roadmap

- [x] Fast multi-threaded scanner, treemap, Top-N view, Trash support
- [x] Fullscreen treemap page
- [ ] **v0.3**: modular codebase, CI on Linux, macOS and Windows
- [ ] **v0.4**: rules engine that explains known folders, with safety levels
      and the first 25 rules (pacman, yay, Cargo, npm, Docker, journal, browsers)
- [ ] **v0.5**: "Reclaimable" view with dry-run, Trash and undo
- [ ] **v0.6**: macOS and Windows support, shareable "space freed" report
- [ ] **v1.0**: 100+ rules, packages for the AUR, Homebrew and winget

Have an idea? [Open an issue](https://github.com/emirkefi/spacesage/issues).

---

<a id="contributing"></a>
## Contributing

SpaceSage gets smarter with every rule people add. Teaching it about a new tool
will take about five lines of TOML, with no Rust required. Once the rules
engine lands, you will be able to add one like this:

```toml
[[rule]]
id = "cargo-target"
name = "Rust build output"
match = { dir_name = "target", sibling_file = "Cargo.toml" }
safety = "safe"
explain = "Compiled artifacts. Rebuilt automatically by `cargo build`."
clean = { trash = true }
```

Code contributions are welcome too. Read [CONTRIBUTING.md](CONTRIBUTING.md) to
get started, and look for issues labeled
[`good first issue`](https://github.com/emirkefi/spacesage/labels/good%20first%20issue).

---

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

<p align="right"><a href="#top">Back to top</a></p>
