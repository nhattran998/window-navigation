# Security

Report vulnerabilities privately through [GitHub's Report a vulnerability](https://github.com/nhattran998/window-navigation/security/advisories/new). Don't post exploit details in a public issue.

Include the Wnav version, OS, what you found, steps to reproduce it, and the impact. A small proof of concept helps. Remove secrets and personal data from logs.

Only the latest release is supported.

## What Wnav can touch

Wnav asks for macOS **Accessibility** permission. With it, Wnav can read and move any application's windows. That is the product — window navigation — and also the blast radius of a bug:

- `wnav` CLI and `wnav-app` move windows only; they never read window content.
- Config lives at `~/.config/wnav/config.toml` and is read, never written, by the CLI.
- The app makes no network connections. `cargo-deny` runs in CI to keep it that way (sources audit in `deny.toml`).
- Global hotkeys come from the OS event tap and are delivered to our own process only.

## Supply chain

- `vendor/` carries the gpui stack; patches to it are reviewed like any code change.
- CI pins the Rust toolchain, action SHAs, and Bazel version; `Cargo.lock` and `Cargo.Bazel.lock` are committed.
