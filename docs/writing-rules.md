# Writing rules

A rule teaches SpaceSage to recognize a folder, explain it and say how safe it is to delete.
Rules are TOML files in [`rules/`](../rules/), grouped by topic (`system/`, `dev/`,
`gaming/`, `apps/`). One file can hold several rules.

> **Status:** the rules engine ships in v0.4. This document describes the planned format,
> and it may change while the engine is being built.

## Example

```toml
[[rule]]
id = "cargo-target"
name = "Rust build output"
match = { dir_name = "target", sibling_file = "Cargo.toml" }
safety = "safe"
explain = "Compiled artifacts. Rebuilt automatically by `cargo build`."
clean = { trash = true }
platforms = ["linux", "macos", "windows"]
```

## Fields

| Field | Required | Description |
|:---|:---:|:---|
| `id` | yes | Unique, lowercase, dashes only (`docker-images`). |
| `name` | yes | Short human-readable name. |
| `match` | yes | How to recognize the folder (see below). |
| `safety` | yes | `safe`, `caution` or `keep`. |
| `explain` | yes | What it is and what happens if it is deleted. One or two sentences. |
| `clean` | no | `{ trash = true }` or `{ command = "..." }`. Leave out for `keep`. |
| `platforms` | no | Defaults to all. Use `linux`, `macos`, `windows`. |

### Match conditions

| Condition | Meaning |
|:---|:---|
| `path` | Exact path, `~` allowed (`"~/.cache/yay"`). |
| `dir_name` | Any folder with this name. Combine with another condition to stay specific. |
| `sibling_file` | The parent folder must contain this file (`Cargo.toml`, `package.json`). |

All conditions in one `match` must be true.

## Safety levels

- **safe**: the tool rebuilds or re-downloads it on its own. Deleting costs time, not data.
- **caution**: deleting loses something, such as history, logins or saved games.
- **keep**: deleting would break the system or lose user data. Shown for explanation only.

When in doubt, choose the more cautious level.

## Checklist before opening a pull request

- [ ] The `id` is unique across all rule files.
- [ ] `match` is specific; it would not match unrelated folders.
- [ ] `explain` says what happens if the folder is deleted.
- [ ] You tested the rule on a real folder (once the engine ships, run `spacesage --check-rules`).
