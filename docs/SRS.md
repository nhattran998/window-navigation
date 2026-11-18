# SRS — Grid (window-navigation)

Software requirements specification for the cross-platform rebuild of Grid.app. Companion to [PRD.md](PRD.md); IDs there (S-xx, US-xx, AC-xx) are referenced here.

## 1. System overview

Three Rust crates + platform backends, one Bazel workspace:

```
┌────────────────────────────────────────────────────────────┐
│  grid-app (gpui-kit)          wnav (clap CLI)              │
│  status-item menu, prefs UI   apply/list/presets --json    │
└──────────────┬──────────────────────────┬──────────────────┘
               │                          │
               ▼                          ▼
        ┌─────────────────────────────────────┐
        │  grid-core (platform-agnostic)      │
        │  Preset table · geometry math ·     │
        │  config (TOML) · hotkey model ·     │
        │  window/screen trait handles        │
        └──────────────┬──────────────────────┘
                       │ trait WindowController / ScreenProvider / HotkeyRegister
        ┌──────────────┼───────────────────────┐
        ▼              ▼                       ▼
  grid-macos      grid-windows (v1.1)    grid-linux (v1.2)
  AXUIElement     Win32 SetWindowPos     X11 / wlr-foreign-toplevel
  CGWindowList    RegisterHotKey         XGrabKey / compositor protocol
  CGEventTap(key) SendInput-free          (Wayland)
```

Crate names: `grid-core`, `grid-macos`, `grid-app`, `wnav` (bin), workspace root = repo root.

### 1.1 Key design decisions

| Decision | Rationale |
|----------|-----------|
| All geometry math in `grid-core`, pure functions over `{screen frame, visible frame, scale}` → frame rects | Unit-testable without a display; backends are thin FFI adapters. Coordinates in **points** (macOS) / logical pixels; backends convert to physical. |
| Async-free core; backends synchronous | AX/Win32 calls are fast (<10 ms); no runtime complexity in v1. |
| Hotkey → preset mapping resolved through the same dispatch table for menu, hotkey, CLI | One code path (S-01…S-04). |
| CLI is a separate binary, not a subcommand of the app | App is LSUIElement, always running; CLI must work standalone for agents (BG-3). |
| Config: single TOML file, watched (notify) for live reload | AC-F1-07 without restart; boring, diffable, agent-editable. |
| gpui-kit `0.10` / `gpui-pre =0.3.12` pinned; consumed via `rules_rust` `crate_universe` | Reproducible Bazel build (BR-1, BG-5). |

## 2. Functional requirements

### 2.1 Presets (FR-1, S-01/S-02)

| Name | Geometry (visible frame `V`, window `W`) | Default shortcut | Menu label |
|------|------------------------------------------|------------------|------------|
| `left` | `W = {V.x, V.y, V.w/2, V.h}` | `^⌥←` | Left |
| `right` | `W = {V.x+V.w/2, V.y, V.w/2, V.h}` | `^⌥→` | Right |
| `top` | `{V.x, V.y, V.w, V.h/2}` | `^⌥↑` | Top |
| `bottom` | `{V.x, V.y+V.h/2, V.w, V.h/2}` | `^⌥↓` | Bottom |
| `top-left` | `{V.x, V.y, V.w/2, V.h/2}` | `^⌥U` | Top Left |
| `top-right` | `{V.x+V.w/2, V.y, V.w/2, V.h/2}` | `^⌥I` | Top Right |
| `bottom-left` | `{V.x, V.y+V.h/2, V.w/2, V.h/2}` | `^⌥J` | Bottom Left |
| `bottom-right` | `{V.x+V.w/2, V.y+V.h/2, V.w/2, V.h/2}` | `^⌥K` | Bottom Right |
| `left-third` | `{V.x, V.y, V.w/3, V.h}` | `^⌥D` | First Third / Left Third |
| `left-two-thirds` | `{V.x, V.y, 2V.w/3, V.h}` | `^⌥E` | First Two Thirds |
| `center-third` | `{V.x+V.w/3, V.y, V.w/3, V.h}` | `^⌥F` | Center Third |
| `right-two-thirds` | `{V.x+V.w/3, V.y, 2V.w/3, V.h}` | `^⌥T` | Last Two Thirds |
| `right-third` | `{V.x+2V.w/3, V.y, V.w/3, V.h}` | `^⌥G` | Last Third |
| `maximize` | `W = V` | `^⌥↩` | Maximize |
| `center` | keep `W.w×W.h`, centered in `V`; clamp into `V` | `^⌥C` | Center |
| `next-display` | move `W` to screen `i+1 (mod n)`, same **relative** position | `^⌥⌘→` | Next Display |
| `previous-display` | screen `i-1 (mod n)` | `^⌥⌘←` | Previous Display |

Rules:
- **FR-1.1** Fractions use integer rounding that keeps edges gap-free: `w1 = round(V.w·f)`, `x2 = V.x + V.w − w_right`.
- **FR-1.2** All math against the screen's **visible frame** (excluding menu bar and dock) (S-07).
- **FR-1.3** `center` never resizes; `maximize` is `visible-frame`, not macOS native zoom (native zoom keeps aspect/uses own semantics — we set exact frame).
- **FR-1.4** `next/previous-display` disabled (greyed, CLI error 6) when only 1 screen (AC-F1-09).
- **FR-1.5** Resulting frame is clamped into the target visible frame and rounded to integral points.

### 2.2 Window targeting (FR-2)

- **FR-2.1** Default target = frontmost window of the frontmost app (AX `kAXFocusedWindowAttribute` of `kAXSystemWideElement` frontmost app).
- **FR-2.2** `--pid N` targets the app with pid N (its focused window, else first window).
- **FR-2.3** `--window "title"`: case-insensitive substring match over all windows of all apps; error 7 if ambiguous unless unique; error 3 if none.
- **FR-2.4** Minimized windows: restore, then move. Hidden apps: unhide (AX raise).
- **FR-2.5** Windows rejected: size 0, off-screen-only, or AX element without movable/resizable set (reported, exit 8, per-window in JSON mode).

### 2.3 Hotkeys (FR-3, S-03)

- **FR-3.1** Global hotkeys via CGEventTap (session event tap, `listenOnly` not sufficient — we need to *consume*; use `defaultSessionEventTap` with `kCGEventTapOptionDefault`, mask `keyDown`+`flagsChanged`).
- **FR-3.2** Modifier default: `Control+Option` (Grid's `^⌥`). Parse/print canonical form `ctrl+alt+<key>`; display glyphs `^⌥X`.
- **FR-3.3** Registration conflict → Preferences shows red badge on that row; other hotkeys unaffected; CLI unaffected.
- **FR-3.4** Hotkeys only active while the app runs; tap disabled/re-enabled with config reload (live remap, AC-F1-07).

### 2.4 CLI contract `wnav` (FR-4, S-04)

```
wnav <PRESET> [--pid N | --window NAME] [--screen NAME|INDEX] [--json] [--screen-name]
wnav list-windows [--json] [--screen NAME|INDEX]
wnav list-screens [--json]
wnav presets [--json]
wnav permission   # exit 0 if accessibility granted else 4, --json for detail
wnav config path | wnav config edit
wnav --version, wnav --help
```

- Exit codes: `0` ok · `2` bad usage · `3` no/ambiguous target window (FR-2.3/2.4) · `4` missing Accessibility permission · `5` unknown preset · `6` single-display for next/previous · `7` ambiguous `--window` match · `8` window refused (FR-2.5) · `10` backend error.
- `--json` on success prints single-line JSON: `{"ok":true,"preset":"left","window":{"pid":4242,"title":"…","app":"Safari"},"from":{x,y,w,h},"to":{x,y,w,h},"screen":1}`; on error `{"ok":false,"error":{"code":5,"message":"…","hint":"…"}}` and nothing else on stdout.
- Human mode: one line `left: Safari — moved (0,25 760×900)`.
- **FR-4.1** No TTY required; safe for scripts/agents; stdout is the only machine channel (BG-3, AC-F1-03).

### 2.5 Config (FR-5, S-06)

`~/.config/grid/config.toml` (OD-3 default; `GRID_CONFIG` env overrides):

```toml
[general]
launch-at-login = true
edge-hints = false            # inert in v1 (OOS-1)

# only overrides from defaults are present in the file
[presets.left]
shortcut = "ctrl+alt+h"
```

- **FR-5.1** Unknown keys/presets are preserved on rewrite (round-trip safe via `toml_edit`).
- **FR-5.2** File watch (notify, 300 ms debounce) → reload → re-register changed hotkeys.
- **FR-5.3** Invalid config: keep last good, log + Preferences banner; CLI surfaces parse error with exit 2.
- **FR-5.4** Login item: v1 uses `SMAppService` (macOS 13+) with LSUIElement-safe registration.

### 2.6 UI (FR-6, S-05/S-06)

- **FR-6.1** Status item menu = Image 1 structure: 5 groups (halves, corners, thirds, maximize/center, displays), icons per row, shortcut column right-aligned, disabled display rows at 1 screen, then Preferences…/About/Quit.
- **FR-6.2** Preferences window (Image 2): rows preset→icon, shortcut recorder (click, capture combo, Esc cancels, X clears), gesture column placeholder (OOS-2); checkboxes Launch at login + edge hints; dark theme default, follows system.
- **FR-6.3** Implementation: gpui-kit components (`Button`, `Checkbox`, `Label`, `TextInput`, …) inside `gpui` windows; menu uses gpui `set_menus`-style native menus on macOS (verify in spike M1; fallback: native `NSMenu` via `objc2`, still built by Bazel).
- **FR-6.4** Both processes (app + CLI) coexist; app already-running is detected by the Preferences-launched second instance (single-instance handoff via unix socket).

## 3. Non-functional requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1 | Hotkey press → AX move committed | ≤ 150 ms p95 (AC via `wnav` timing < 300 ms) |
| NFR-2 | App idle CPU | ≤ 1 % |
| NFR-3 | App memory | ≤ 120 MB |
| NFR-4 | `wnav` cold start | ≤ 300 ms to side-effect |
| NFR-5 | Clean-clone build | `bazel build //...` with Bazel + Xcode CLT only (no cargo, no rustup) |
| NFR-6 | Determinism | Locked crate set (`Cargo.lock` ↔ `crate_universe` lockfile committed) |
| NFR-7 | Accessibility | Menu + prefs reachable via keyboard; AccessKit labels via gpui-kit |
| NFR-8 | Privacy | No network calls in v1; no telemetry |
| NFR-9 | Core logic test coverage | Geometry + config + dispatch covered by `bazel test //...` |

## 4. Architecture

### 4.1 `grid-core` (pure)

```rust
pub struct Screen { pub index: usize, pub name: String,
                    pub frame: Rect, pub visible: Rect, pub scale: f64 }
pub struct WindowInfo { pub pid: i32, pub title: String, pub app: String,
                        pub frame: Rect, pub minimized: bool, pub screen: usize }
pub struct Rect { pub x: f64, pub y: f64, pub w: f64, pub h: f64 } // points

pub fn presets() -> &'static [Preset];              // 17, ordered as menu
pub fn geometry(p: PresetId, w: &WindowInfo, s: &Screen,
                screens: &[Screen]) -> Result<Rect, ApplyError>;
pub fn canonical_shortcut(s: &str) -> Result<Shortcut, ParseErr>; // "ctrl+alt+h" ⇄ glyphs
pub fn load_config(path) -> Result<Config, ConfigError>;            // defaults + overrides
pub fn effective_shortcuts(cfg) -> Vec<(PresetId, Shortcut)>;       // 17 rows for UI/registration
```

Tests: fraction rounding gap-free, center clamps, display-relative move, shortcut round-trip, config round-trip via `toml_edit`.

### 4.2 `grid-macos` (FFI adapter)

- **ScreenProvider**: `CGGetActiveDisplayList` + `CGDisplayBounds` + `NSScreen` visibleFrame equivalent via `CGDisplayGetBounds` minus menu bar (`MenuBar/NSStatusBar` not needed — use `kCGDisplayBounds` + `CGDisplayPixelsWide` ratio) — concretely: use `objc2` + `NSScreen::screens` for `frame`/`visibleFrame`/`scale` (simplest correct source).
- **WindowController**: `AXUIElementCreateApplication(pid)`, `kAXWindowsAttribute`, set `kAXPositionAttribute`/`kAXSizeAttribute` (`AXUIElementSetAttributeValue` with `AXValue` structs). Permission probe: `AXIsProcessTrustedWithOptions(kAXTrustedCheckOptionPrompt)`.
- **HotkeyRegister**: CGEventTap as FR-3.1; callback → channel → dispatch loop in app; CLI path skips tap entirely.
- Window enumeration for CLI: `CGWindowListCopyWindowInfo` + AX merge for titles (CGWindow names unreliable for Electron).

Crate choice: `objc2` + `objc2-app-kit`/`objc2-application-services`/`objc2-core-graphics` (no ObjC source files → pure Rust → crate_universe-friendly).

### 4.3 `wnav` CLI

clap; thin: parse → resolve target via backend → `grid_core::geometry` → apply → print. `--json` via `serde_json`. Same binary drives the app's IPC (`wnav` invoked by menu actions is NOT needed — app calls core directly).

### 4.4 `grid-app` (gpui-kit)

- `main`: `Application::new().run(...)`; create status item + menu (FR-6.1), open Preferences window on demand; run hotkey dispatch loop; watch config.
- Single-instance: lock file + unix socket `~/Library/Application Support/grid/agent.sock`; second launch forwards `--open-prefs` and exits.

## 5. Data & state

| State | Where |
|-------|-------|
| Config (shortcuts, toggles) | `~/.config/grid/config.toml` (FR-5) |
| Runtime: registered hotkeys, screens cache (refresh on `NSApplicationDidChangeScreenParametersNotification`) | in-memory only |
| No DB, no caches on disk | — |

## 6. Error handling

| Case | Behavior |
|------|----------|
| No accessibility permission | CLI exit 4 + hint; app shows first-run dialog (AC-F1-11) |
| AX set fails (timeout 2 s) | retry once; exit 10 with AX error code in JSON |
| Config invalid | keep last good (FR-5.3) |
| Screen unplug mid-apply | clamp into nearest remaining screen |
| Window gone between list & apply | exit 3 |

## 7. Verification plan

| Layer | How |
|-------|-----|
| Geometry/config (core) | `bazel test //crates/grid-core:test` (pure, exhaustive fractions, clamps, multi-screen) |
| CLI contract | integration test spawns `wnav` against a fixture window (TestApp via `osascript`-created window or `wnav` self-window); asserts exit codes + JSON schema |
| Manual UI | Preferences render + recorder, menu rows/disabled states — screenshots in PR |
| End-to-end | `wnav left --json` on a real Terminal window, frame verified via `wnav list-windows --json` (AC-F1-01) |

## 8. Build & layout

```
MODULE.bazel              # rules_rust, crate_universe (lockfile), macos pkg rules
.bazelrc                  # macOS arm64 defaults, release opt
Cargo.toml                # workspace (cargo users get parity)
crates/grid-core/…        # lib + tests
crates/grid-macos/…       # lib (cfg(target_os = "macos"))
crates/wnav/…             # bin
crates/grid-app/…         # bin (gpui-kit)
tools/bazel/…             # app bundle rule (Info.plist, LSUIElement, icon)
docs/PRD.md docs/SRS.md
```

- `bazel build //:grid_app_bundle` → `Grid.app` (LSUIElement, AC-F1-10); `bazel run //crates/wnav` for CLI dev.
- Info.plist: `CFBundleIdentifier app.grid.Grid`, `LSUIElement=true`, minimal (no Sparkle, OOS-3).
- Icons in v1: reuse the existing Grid menu icon shapes drawn as gpui SVGs; app icon optional (defer).

## 9. Platform backends (roadmap)

| Concern | macOS v1.0 | Windows v1.1 | Linux v1.2 (X11 / Wayland) |
|---------|-----------|--------------|-----------|
| Enumerate windows | CGWindowList+AX | EnumWindows | XQueryTree+NET_CLIENT_LIST / wlr-foreign-toplevel |
| Move/resize | AX set position/size | SetWindowPos | XMoveResizeWindow / toplevel protocol |
| Visible frame | NSScreen visibleFrame | SPI_GETWORKAREA | _NET_WORKAREA / compositor margins |
| Global hotkey | CGEventTap | RegisterHotKey | XGrabKey / global-shortcuts protocol |
| Login item | SMAppService | Registry Run key | XDG autostart |
| Menu bar icon | NSStatusItem (via gpui) | tray (winrt) | StatusNotifierItem |

Core stays backend-agnostic; the three traits (`ScreenProvider`, `WindowController`, `HotkeyRegister`) are the only porting surface (BG-4).

## 10. Risks & fallbacks (from PRD section 7, technical detail)

- **BR-1** gpui under Bazel: spike M1 validates `crate_universe` + `gpui-pre` build scripts on macOS arm64. Fallbacks in order: (a) `rules_rust` with cargo vendor dir; (b) genrule that shells to `cargo build --release` for the app target only, core+CLI stay pure Bazel. Either keeps BG-5's "one command" property.
- **BR-2** AX frame anomalies: normalize frames (min size 1×1, clamp to screen); if AX reports garbage, fall back to CGWindow bounds for `from`-reporting but never for apply target.
- **BR-4** `gpui-pre` drift: exact pin `=0.3.12` both in Cargo workspace and crate_universe annotation.
