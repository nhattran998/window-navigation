# Wnav

Navigate your windows with keyboard or CLI — the open rebuild of Grid.app in Rust, cross-platform (macOS first, then Windows, then Linux).

Position any window (left, right, top-left, left-third, maximize, next display…) with global hotkeys or a command line agents can call.

- Docs: [PRD](docs/PRD.md) · [SRS](docs/SRS.md)
- UI: [gpui-kit](https://github.com/longbridge/gpui-kit) (vendored in `vendor/`) · Build: [Bazel](https://bazel.build) or Cargo
- License: MIT — see [licenses/](licenses/) for third-party notices

## Status

macOS v1.0 in progress (M1–M4 in [PRD section 8](docs/PRD.md#8-roadmap)). Windows v1.1, Linux v1.2 planned.

## Quick start

Needs Rust 1.100+ (rustup) and [just](https://github.com/casey/just).

```sh
just build        # cargo build the workspace
just test         # unit tests + clippy -D warnings
just lint         # clippy strictest + cargo-deny + fmt check + taplo fmt --check
just bazel-build  # the same targets under Bazel
```

## CLI

```sh
wnav presets                       # all 17 presets with default shortcuts
wnav list-screens                  # displays with visible frames
wnav list-windows [--json]         # movable windows with pid/app/title/frame
wnav left --json                   # snap the frontmost window to the left half
wnav top-right --pid 4242          # target a specific app
wnav maximize --window "Safari"    # target by title substring
wnav permission                    # exit 4 when Accessibility is missing
```

First CLI/app run needs the Accessibility permission: System Settings → Privacy & Security → Accessibility → add `wnav` (or run `wnav permission` to check).

## Hotkeys

Defaults match Grid.app (`^⌥←` left, `^⌥→` right, `^⌥U` top-left, `^⌥D` left-third, `^⌥↩` maximize, `^⌥⌘→` next display…). Override in `~/.config/wnav/config.toml`:

```toml
[presets.left]
shortcut = "ctrl+alt+h"
```

## Layout

```
crates/wnav-core     presets, geometry math, shortcuts, config — pure, tested
crates/wnav-macos    AX/CG/NSScreen backend
crates/wnav          CLI binary (agent-facing)
crates/wnav-app      gpui-kit menu-bar app + Preferences window
vendor/              vendored gpui-* crates we may need to patch
brand/               icons, backgrounds, platform assets
licenses/            third-party license texts + inventory
scripts/             dev/release automation (see scripts/README.md)
docs/                PRD, SRS
```

## License

MIT
