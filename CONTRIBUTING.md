# Contributing to MONORYX

MONORYX is open source under Apache-2.0 by DemonZDevelopment. Contributions are welcome.

## Development Setup

```powershell
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features
cargo test
cargo build --release
```

Use the stable Rust toolchain (1.85+). On Windows GNU builds you need MinGW-w64 on `PATH`.

## Code Style

- `cargo fmt` must pass. Do not commit unformatted code.
- `cargo clippy --all-targets --all-features` must pass without serious warnings. Do not add broad `allow` attributes to hide problems.
- Small cohesive modules, typed enums, `Result`-based error handling.
- No `unwrap()` in production paths that touch network, disk, or user input. Handle errors with context and user-facing messages.
- No blocking network or disk-heavy work on the egui thread. Use Tokio tasks plus message passing.
- No JavaScript/TypeScript/Electron/webview. Launcher is native Rust; Minecraft remains Java.

## Tests

- Add unit tests for deterministic logic: UUIDs, Maven paths, rules, argument substitution, hashes, archive safety, compatibility selection, dependency cycles, config serialization.
- Run the full suite before opening a PR: `cargo test`.
- Integration tests that hit the network must be opt-in and must not run by default.

## Commits

- Keep commits focused and descriptive, e.g. `loaders: fix Quilt profile merge`.
- Reference issues where applicable.
- Do not commit secrets, tokens, game JARs, or user data.

## Security

- Treat downloads, archives, and API responses as untrusted.
- Never use shell string concatenation for processes. Use explicit argv.
- Reject path traversal, absolute archive paths, and symlinks on extraction.
- Verify hashes before accepting files. Never execute downloaded code except the documented Java/Minecraft launch flow.
- See `SECURITY.md` for reporting vulnerabilities. Do not open public issues for suspected vulnerabilities.
