# AGENTS.md — Wnav

Agent-facing context for working in this repository. Read before changing anything.

## What this is

Wnav rebuilds the macOS menu-bar app **Grid.app** (`app.macgrid.Grid` v1.4) as an
open, cross-platform Rust app: position windows (left, right, top-left,
left-third, maximize, next/previous display…) with **global hotkeys** or a
**CLI agents can call**. Platform order: macOS (v1.0, shipping) → Windows
(v1.1) → Linux (v1.2).

Reference material: `docs/PRD.md` (scope, stories, ACs, roadmap) and
`docs/SRS.md` (architecture, FR/NFR tables, CLI contract, config schema).
The PRD's acceptance criteria IDs (AC-F1-xx) are the source of truth for
"done".

## Layout

```
crates/wnav-core     Pure engine: 17 presets, gap-free geometry vs visible
                     frames, shortcut model (canonical "ctrl+alt+k" ⇄ "⌃⌥K"),
                     TOML config. No FFI, no async — fully unit-tested.
crates/wnav-macos    macOS backend: AX move/resize (hand-rolled FFI in ax.rs,
                     stable C symbols since 10.4), CGWindowList enumeration,
                     NSScreen frames, global-hotkey engine. objc2 only.
crates/wnav          CLI binary. THE agent interface: `wnav left --pid N
                     --json`, documented exit codes (SRS §2.4).
crates/wnav-app      gpui-kit menu-bar app: native menus, keybindings,
                     Preferences window (Apple-minimal, see design notes).
vendor/              Vendored gpui stack (gpui-kit 0.10.0, gpui-pre 0.3.12),
                     path-patched via [patch.crates-io] — change behavior here
                     only with an upstream issue link in the commit message.
brand/               Identity SVGs; binaries are generated, never hand-edited.
licenses/            Third-party license texts + inventory.json.
scripts/             Automation; `justfile` is the only entry point.
.githooks/           pre-commit (fmt+clippy), pre-push (test+deny+bazel).
                     Activate: git config core.hooksPath .githooks
.codegraph/          Local code index (git-ignored). `codegraph init` to refresh.
```

## Commands

`just` lists everything. The load-bearing ones:

- `just build` / `just test` — cargo build/test
- `just lint` — clippy `-D warnings` + cargo-deny + fmt checks (strictest
  ruleset lives in `[workspace.lints]` in `Cargo.toml`; keep it strict)
- `just bazel-build` / `just bazel-test` — Bazel parity (rules_rust,
  toolchain pinned 1.100.0 in `MODULE.bazel`)
- `just bazel-repin` — regenerate `Cargo.Bazel.lock` after manifest changes
- `just pre-release X.Y.Z` — test + lint + changelog (git-cliff)
- `bash scripts/ci/ci-local.sh` — exactly what CI runs

## Non-obvious constraints (learned the hard way)

1. **macOS window visibility vs threads.** Any raw `std::thread::spawn` before
   the NSApp runloop starts breaks window visibility (bisected, 2026-11-18).
   All platform work runs on gpui executors; the hotkey pump polls
   global-hotkey's crossbeam receiver from a background task and hops events
   to the main thread through an mpsc + `cx.spawn`.
2. **AX needs the main thread.** Every `AXUIElement*` call happens on the
   process main thread. NSScreen too (`MainThreadMarker`).
3. **Coordinates are points, y-down, top-left origin** (CG/AX convention).
   NSScreen reports bottom-left y-up — `wnav-macos/src/screens.rs` converts.
   Geometry math lives ONLY in `wnav-core/src/geometry.rs`: bands anchor
   right/bottom edges to the visible frame so adjacent presets stay gap-free.
4. **gpui-pre gates inspector derives** behind
   `cfg(any(feature = "inspector", debug_assertions))`. Bazel opt builds need
   `crate.annotation(crate = "gpui-pre", crate_features = ["inspector"])` in
   `MODULE.bazel` (already there — keep it when bumping).
5. **`gpui-kit` macros expand to `::gpui::` paths.** Any crate using the
   `actions!`/`Action` macros needs `extern crate gpui_kit as gpui;`
   (kit docs, kit/src/lib.rs:94-117). `crates/wnav-app/src/actions.rs` has it.
6. **Bazel target names keep hyphens** (`@crates//:global-hotkey`), our own
   targets use underscores (`//crates/wnav-core:wnav_core`).
7. **Config round-trips through `toml_edit`** — never rewrite config.toml with
   `toml::to_string`, it destroys user comments/unknown keys (FR-5.1).
8. **Sandboxed systems:** `list-windows` must source pids from CGWindowList
   (window-server truth), not `NSWorkspace.runningApplications` (stale/short).

## CLI contract (don't break agents)

Single-line JSON to stdout, human text otherwise, exit codes: 0 ok, 2 usage,
3 no window, 4 no Accessibility, 5 unknown preset, 6 single display, 7
ambiguous `--window`, 8 refused, 10 backend. See SRS §2.4. Changing any of
these is a breaking release.

## Conventions

- Commits: conventional (`feat:`, `fix:`, …) — git-cliff builds the changelog
  from them. Author/committer must be `nhattran998 <nhattq.coding@gmail.com>`.
- Clippy is denied at CI level; run `just lint` before pushing. The ruleset
  intentionally denies `unwrap_used`, `indexing_slicing`, `float_cmp`,
  `print_stdout/stderr` exceptions listed inline with justification comments.
- UI follows Apple HIG (forked skill in `.agents/apple-design/` — SKILL.md is
  the review protocol; `crates/wnav-app/src/design.rs` holds the token
  metrics). Minimal > decorated. No alerts for recoverable state; inline
  errors only.
- Tests: pure logic in `wnav-core` (geometry/config/shortcuts) and
  `wnav-app/src/keymap.rs`. Live verification for AX behavior (move a real
  window, check frames via `wnav list-windows --json`) — never mock AX.

## Release

1. `just pre-release X.Y.Z`
2. Review CHANGELOG.md, commit
3. `git tag vX.Y.Z && git push origin main vX.Y.Z`
4. `.github/workflows/release.yml` publishes artifacts + notes from the tag
