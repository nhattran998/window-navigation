# PRD — Grid (window-navigation)

Rebuild of the macOS menu-bar app **Grid.app** (`app.macgrid.Grid` v1.4) as a cross-platform Rust app, macOS first, then Windows, then Linux.

- Repo: https://github.com/nhattran998/window-navigation
- UI: [gpui-kit](https://github.com/longbridge/gpui-kit) (GPUI)
- Build: [Bazel](https://bazel.build/) (`rules_rust` + `crate_universe`)
- Reference: screenshots of Grid menu + Grid Preferences (2026-11-18), and `/Applications/Grid.app`

## 1. Problem

**PROB-1** — Positioning windows (left half, top-left quarter, center third, …) on macOS today requires Grid.app, which is closed-source, macOS-only, not scriptable, and cannot be driven by AI agents. Manual mouse-drag positioning is slow and imprecise; multi-window workflows (terminal + browser + editor) need repeatable layouts.

**PROB-2** — Agents (Claude/CLI automation) have no reliable programmatic way to place a window. AppleScript/UI-scripting is fragile and permission-hostile. A first-class CLI with machine-readable output removes that.

## 2. Personas

| ID | Persona | Moment |
|----|---------|--------|
| PS-1 | **Power user (Harry)** | Multi-screen macOS dev workstation; wants Grid.app parity with keyboard shortcuts; prefers OSS he can extend. |
| PS-2 | **Agent / automation** | LLM agent or shell script that must place a window deterministically: `wnav left --pid 4242 --json`. |
| PS-3 | **Cross-platform user (later)** | Windows/Linux (Omarchy/Ubuntu) user wanting the same muscle memory and config file. |

## 3. Goals & KPIs

| ID | Goal | KPI | Target | Window |
|----|------|-----|--------|--------|
| BG-1 | Feature parity with Grid.app core | Presets supported | 17/17 (menu of Image 1) | v1.0 |
| BG-2 | Keyboard-first | Hotkey → window moved | < 150 ms perceived | v1.0 |
| BG-3 | Agent-controllable | CLI round-trip (`wnav <preset> --json`) | < 300 ms, exit code 0 | v1.0 |
| BG-4 | Cross-platform core | Platforms shipping backend | macOS v1.0; Windows v1.1; Linux v1.2 | roadmap |
| BG-5 | Reproducible build | `bazel build //:grid_app` clean on macOS | 1 command, no cargo needed | v1.0 |

## 4. Scope

### In scope (v1.0, macOS)

- **S-01** All 15 window presets: left, right, top, bottom, top-left, top-right, bottom-left, bottom-right, left-third, left-two-thirds (first-two-thirds), center-third, right-two-thirds (last-two-thirds), right-third (last-third), maximize, center.
- **S-02** Display switching presets: next-display, previous-display (moves window to adjacent screen, preserving relative geometry).
- **S-03** Global hotkeys for every preset, defaults identical to Grid Preferences (Image 2), user-remappable.
- **S-04** CLI `wnav`: apply preset, list windows, list screens, list presets; `--json`, `--pid`, `--window`, `--screen` targeting.
- **S-05** Menu-bar (status item) app with menu identical to Image 1 (icons, shortcuts column, Preferences/About/Quit), built with gpui-kit; `LSUIElement` (no Dock icon).
- **S-06** Preferences window (Image 2): per-preset shortcut recorder, Launch at login, edge-hints toggle; persisted to `~/.config/grid/config.toml`.
- **S-07** Geometry engine uses the **visible frame** (menu bar/dock excluded) and account per-screen scale factor; center keeps window size.
- **S-08** Bazel build producing a runnable macOS `.app` bundle + bare `wnav` binary.

### Out of scope (this release)

| ID | Item | Reason |
|----|------|--------|
| OOS-1 | Edge hints overlay while dragging (Prefs checkbox exists in v1 but is inert) | Requires global mouse tracking (CGEventTap); high complexity, deferred to M4+. |
| OOS-2 | Title-bar drag gestures (⇧← → etc.) | Mouse-gesture engine, macOS-only API surface; defer. |
| OOS-3 | Sparkle auto-update / Check for Updates | Needs signing infra; manual install acceptable for OSS v1. |
| OOS-4 | Windows & Linux backends | Roadmap v1.1/v1.2; core + CLI ship platform-agnostic from day one. |
| OOS-5 | Layouts spanning multiple windows (fancy zones) | Grid.app parity is single-window presets; multi-window layout is a different product. |
| OOS-6 | Toggling behavior on repeated hotkey (undo/restore) | Open decision OD-2; v1 applies the deterministic frame every time. |

## 5. User journeys

**UJ-1 (hotkey)** — User focuses Safari, presses `⌃⌥←` → Safari snaps to left half of the visible frame. ~100 ms.

**UJ-2 (agent)** — Agent runs `wnav list-windows --json`, picks the window titled "Neovim", runs `wnav center-third --pid 8123 --json`, parses `{"ok":true,...}` exit 0.

**UJ-3 (menu)** — User clicks the Grid status item, selects "Bottom Right ⌃⌥K" → frontmost app's window snaps.

**UJ-4 (remap)** — User opens Preferences, clicks the Left shortcut field, types `⌃⌥H`, field shows `⌃⌥H`, file now records `left.shortcut = "ctrl+alt+h"`, hotkey re-registered live.

## 6. Stories & acceptance criteria

Platform capability **X1** (geometry engine, CLI, config, hotkey engine); user-facing feature **F1** (position presets).

| ID | Story | MoSCoW | Acceptance |
|----|-------|--------|------------|
| US-X1-01 | As a user, I press a global hotkey and the focused window snaps to the preset. | M | AC-F1-01 |
| US-X1-02 | As a user, all 17 presets from the Grid menu exist with Grid-default shortcuts. | M | AC-F1-02 |
| US-X1-03 | As an agent, I call `wnav <preset>` and get a deterministic exit code + JSON. | M | AC-F1-03 |
| US-X1-04 | As a user, I can retarget a non-focused window via `--pid` or `--window`. | S | AC-F1-04 |
| US-X1-05 | As a user, on a second display the window moves to the next/previous screen keeping its relative position. | M | AC-F1-05 |
| US-X1-06 | As a user, geometry respects the visible frame (no window under menu bar/dock) and per-screen scale. | M | AC-F1-06 |
| US-X1-07 | As a user, I remap any shortcut in Preferences and it takes effect immediately. | S | AC-F1-07 |
| US-X1-08 | As a user, I toggle Launch at login. | S | AC-F1-08 |
| US-X1-09 | As a user, I get a menu-bar menu identical to Grid's, incl. Next/Previous Display (disabled with <2 screens). | S | AC-F1-09 |
| US-X1-10 | As a developer, `bazel build //...` builds CLI + app from a clean clone. | M | AC-F1-10 |
| US-X1-11 | As a user, first launch detects missing Accessibility permission and shows fix instructions. | M | AC-F1-11 |

### Acceptance criteria

- **AC-F1-01** With Terminal focused, `⌃⌥←` moves its window so that its frame equals the left half of the screen's visible frame (±2 px). Observable via `wnav list-windows --json`.
- **AC-F1-02** `wnav presets --json` lists 17 presets; each fires the matching geometry on a test window.
- **AC-F1-03** `wnav left --json` exits 0 and prints `{"ok":true,...,"to":{...}}`; unknown preset exits 5; no window exits 3; missing Accessibility permission exits 4 with hint on stderr.
- **AC-F1-04** `wnav maximize --window "Safari"` moves the Safari window even while Terminal stays focused; title match is case-insensitive substring.
- **AC-F1-05** With 2 displays, `wnav next-display` moves the window to the other display at the same relative anchor point (≥ same relative x/y within 5%).
- **AC-F1-06** On a 14" MBP (menu bar 24 pt + dock), `maximize` puts the window inside the visible frame; scaled (2x) and unscaled screens both correct.
- **AC-F1-07** Remapping `left` to `⌃⌥H` in Preferences: old combo stops working, new works, `config.toml` updated without restart.
- **AC-F1-08** Toggling Launch at login adds/removes the app in System Settings → Login Items.
- **AC-F1-09** Menu shows all entries of Image 1 with shortcut glyphs; Next/Previous Display greyed out with exactly 1 screen.
- **AC-F1-10** `bazel build //...` succeeds on a clean clone on macOS with only Bazel (bazelisk) + Xcode CLT installed.
- **AC-F1-11** On first `wnav`/app run without Accessibility permission: macOS dialog prompts, CLI exits 4 with the System Settings path printed.

## 7. Risks

| ID | Risk | Level | Mitigation | Owner |
|----|------|-------|------------|-------|
| BR-1 | GPUI under Bazel (`crate_universe` + build scripts + Metal) may fight the toolchain | High | Spike M1 before feature work; fallback documented in SRS section 10 (cargo wrapper rule, still one `bazel build` entry) | eng |
| BR-2 | Accessibility (AX) quirks: Electron/browser windows expose odd AX frames | Medium | Clamp/normalize frames; fallback to CGWindow bounds for position math | eng |
| BR-3 | Global hotkey conflicts (⌃⌥ combos taken by IMEs/other apps) | Medium | Remappable shortcuts; registration failure surfaced in Preferences (red badge) | eng |
| BR-4 | `gpui-pre` snapshot API drift vs docs | Low | Pin `=0.3.12` via gpui-kit 0.10; `Cargo.lock`/crate_universe lockfile committed | eng |
| BR-5 | Notarization/gatekeeper friction for distributed app | Low | v1 is self-built (OOS-3); ad-hoc signing for local run | eng |

## 8. Roadmap

| Milestone | Content | Exit |
|-----------|---------|------|
| M0 | PRD + SRS (this) | Approved |
| M1 | Bazel scaffold + gpui-kit window spike | `bazel run //app` shows a window |
| M2 | `grid-core` + `grid-macos` + `wnav` CLI | AC-F1-02…06, 11 pass |
| M3 | gpui-kit menu-bar app + Preferences + hotkeys | AC-F1-01, 07–09 pass |
| M4 | Polish: bundle target, README, login item, edge-hints groundwork | AC-F1-10 pass; v1.0 tag |
| v1.1 | Windows backend (Win32 `SetWindowPos`, `RegisterHotKey`) | same ACs on Windows |
| v1.2 | Linux backend (X11 first; Wayland via wlr foreign-toplevel) | same ACs on Ubuntu/Omarchy |

## 9. Open decisions

- **OD-1** App/product name: keep "Grid" (working title `grid`) vs rename. Default: **grid**.
- **OD-2** Repeat-hotkey behavior: deterministic re-apply (default v1) vs toggle/restore original frame.
- **OD-3** Config location: `~/.config/grid/config.toml` (default) vs macOS `~/Library/Application Support/grid/`.
