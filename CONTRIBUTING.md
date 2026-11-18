# Contributing to SpaceSage

Thanks for helping! Bug reports, new cleanup rules, docs fixes and code are all welcome.

## The easiest way to contribute: add a rule

SpaceSage explains folders it recognizes (build caches, package caches, logs, and so on).
Each one is a small TOML file in [`rules/`](rules/), and no Rust is required.
See [docs/writing-rules.md](docs/writing-rules.md) for the format, then open a pull request.

Good rules are:
- **Specific**: they match one tool's folder, not "anything called `cache`".
- **Honest about safety**: if you're unsure, use `caution`, never `safe`.
- **Explained**: say what the folder is and what happens if it's deleted.

## Reporting bugs

Search the [issue tracker](https://github.com/emirkefi/spacesage/issues) first, then open a
bug report. Include your OS, terminal, SpaceSage version (`spacesage --version`), the steps
to reproduce, and what you expected to happen.

## Suggesting features

Open a feature request and describe the problem you're trying to solve, not just the
solution. Screenshots or mockups of TUI ideas help a lot.

## Pull requests

1. Fork the repo and create a branch: `git checkout -b feature/short-name` or `fix/short-name`.
2. Make your change. Keep it focused: one topic per pull request.
3. Run the checks locally:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   ```
4. Add or update tests when you change behavior. UI changes can be tested by rendering to
   ratatui's `TestBackend`; see the existing tests in `src/main.rs` for examples.
5. Open the pull request and fill in the template.

## Safety rules for code that deletes things

SpaceSage touches people's files, so changes in this area get extra review:
- Deleting must go to the Trash by default.
- Never delete during a scan.
- Anything under system paths must stay protected (`src/safety.rs`).

## Code of conduct

Everyone taking part must follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions are licensed under the
[Apache License 2.0](LICENSE).
