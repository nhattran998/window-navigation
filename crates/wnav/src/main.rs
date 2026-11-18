//! wnav CLI (SRS §2.4). Agent-facing: no TTY required, JSON contract,
//! documented exit codes.

mod output;

use clap::{Parser, Subcommand};
use wnav_core::{find_preset, geometry, Config};

use output::{emit, JsonErr};

/// Exit codes (SRS section 2.4 / AC-F1-03).
pub const EXIT_USAGE: i32 = 2;
/// No target window matched the filters.
pub const EXIT_NO_WINDOW: i32 = 3;
/// Accessibility permission missing.
pub const EXIT_PERMISSION: i32 = 4;
/// Unknown preset name.
pub const EXIT_UNKNOWN_PRESET: i32 = 5;
/// Display-switch preset with one screen.
pub const EXIT_SINGLE_DISPLAY: i32 = 6;
/// `--window` matched multiple windows.
pub const EXIT_AMBIGUOUS: i32 = 7;
/// Window refused the move (AX error).
pub const EXIT_REFUSED: i32 = 8;
/// Backend failure (no screens, AX error).
pub const EXIT_BACKEND: i32 = 10;

#[derive(Parser)]
#[command(
    name = "wnav",
    version,
    about = "Navigate windows into positions (left, right, top-left, ...)",
    after_help = "Presets: wnav presets. Permission check: wnav permission."
)]
struct Cli {
    /// Machine-readable single-line JSON on stdout.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Apply a position preset: `wnav left`, `wnav top-right`, `wnav maximize`.
    #[command(external_subcommand)]
    PresetApply(Vec<String>),
    /// List movable windows with pid, app, title, frame, screen.
    ListWindows {
        /// Only windows on this screen index.
        #[arg(long)]
        screen: Option<usize>,
    },
    /// List screens with frame / visible frame / scale.
    ListScreens,
    /// List all preset names with default shortcuts.
    Presets,
    /// Check Accessibility permission (exit 0 granted, 4 missing).
    Permission,
    /// Print the config file path.
    ConfigPath,
}

fn main() {
    let cli = Cli::parse();
    let code = run(cli);
    std::process::exit(code);
}

fn run(cli: Cli) -> i32 {
    match cli.command {
        Cmd::Permission => {
            let trusted = wnav_macos::is_trusted();
            if cli.json {
                emit(&serde_json::json!({ "ok": trusted, "trusted": trusted }));
            } else if trusted {
                println!("accessibility: granted");
            } else {
                eprintln!("accessibility: NOT granted");
                eprintln!(
                    "fix: System Settings -> Privacy & Security -> Accessibility -> add wnav/Grid"
                );
            }
            if trusted {
                0
            } else {
                EXIT_PERMISSION
            }
        }
        Cmd::ConfigPath => {
            let p = wnav_core::config::default_config_path();
            if cli.json {
                emit(&serde_json::json!({ "path": p.to_string_lossy() }));
            } else {
                println!("{}", p.display());
            }
            0
        }
        Cmd::Presets => {
            if cli.json {
                let list: Vec<_> = wnav_core::PRESETS
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "name": wnav_core::preset::serde_kebab(p.id),
                            "label": p.label,
                            "default-shortcut": p.default_shortcut,
                            "needs-two-screens": p.needs_two_screens,
                        })
                    })
                    .collect();
                emit(&serde_json::json!({ "presets": list }));
            } else {
                for p in wnav_core::PRESETS {
                    println!(
                        "{:<20} {:<18} {}",
                        wnav_core::preset::serde_kebab(p.id),
                        p.default_shortcut,
                        p.label
                    );
                }
            }
            0
        }
        Cmd::ListScreens => {
            let screens = wnav_macos::screens();
            if cli.json {
                emit(&serde_json::json!({ "screens": screens }));
            } else {
                for s in &screens {
                    println!(
                        "{}: {} {}x{} visible {}x{} scale {}",
                        s.index,
                        s.name,
                        px(s.frame.w),
                        px(s.frame.h),
                        px(s.visible.w),
                        px(s.visible.h),
                        s.scale
                    );
                }
            }
            0
        }
        Cmd::ListWindows { screen } => {
            if !wnav_macos::is_trusted() {
                return permission_denied(cli.json);
            }
            let mut windows = wnav_macos::list_windows();
            if let Some(s) = screen {
                windows.retain(|w| w.screen == s);
            }
            if cli.json {
                emit(&serde_json::json!({ "windows": windows }));
            } else {
                for w in &windows {
                    println!(
                        "{:<8} {:<18} {:<30} ({:>5},{:>5} {}x{}) screen {}",
                        w.pid,
                        w.app,
                        truncate(&w.title, 28),
                        px(w.frame.x),
                        px(w.frame.y),
                        px(w.frame.w),
                        px(w.frame.h),
                        w.screen
                    );
                }
            }
            0
        }
        Cmd::PresetApply(args) => {
            // external_subcommand: first token = preset name, rest = trailing args
            let mut preset = None;
            let mut pid = None;
            let mut window = None;
            let mut json_flag = cli.json;
            let mut it = args.iter();
            while let Some(tok) = it.next() {
                match tok.as_str() {
                    "--json" => json_flag = true,
                    "--pid" => {
                        pid = it.next().and_then(|v| v.parse().ok());
                        if pid.is_none() {
                            return err_exit(cli.json, EXIT_USAGE, "--pid needs a number", None);
                        }
                    }
                    "--window" => window = it.next().map(String::as_str).map(str::to_string),
                    other if preset.is_none() => preset = Some(other.to_string()),
                    other => {
                        return err_exit(
                            cli.json,
                            EXIT_USAGE,
                            &format!("unexpected argument {other:?}"),
                            None,
                        )
                    }
                }
            }
            let Some(preset) = preset else {
                return err_exit(
                    cli.json,
                    EXIT_USAGE,
                    "missing preset name",
                    Some("try `wnav left`"),
                );
            };
            apply(json_flag, &preset, pid, window.as_deref())
        }
    }
}

/// Apply flow: config -> resolve target -> geometry -> AX apply -> report.
fn apply(json: bool, preset_name: &str, pid: Option<i32>, window: Option<&str>) -> i32 {
    let Some(id) = find_preset(preset_name) else {
        return err_exit(
            json,
            EXIT_UNKNOWN_PRESET,
            &format!("unknown preset {preset_name:?}"),
            Some("run `wnav presets` for the list"),
        );
    };

    if !wnav_macos::is_trusted() {
        return permission_denied(json);
    }

    // Fail fast on broken config before touching windows (FR-5.3, exit 2).
    let cfg_path = wnav_core::config::default_config_path();
    if let Err(e) = Config::load(&cfg_path) {
        return err_exit(
            json,
            EXIT_USAGE,
            &format!("config error: {e}"),
            Some(&format!("fix or remove {}", cfg_path.display())),
        );
    }

    // Resolve the target window (AppKit + AX; CLI is on the process main thread).
    let resolved: Result<FrontWindow, ApplyFailure> = (|| {
        let screens = wnav_macos::screens();
        if screens.is_empty() {
            return Err(ApplyFailure::Backend("no screens detected".into()));
        }
        let win = match wnav_macos::resolve_window(pid, window) {
            wnav_macos::TargetResolution::Found(w) => w,
            wnav_macos::TargetResolution::None => return Err(ApplyFailure::NoWindow),
            wnav_macos::TargetResolution::Ambiguous(t) => {
                return Err(ApplyFailure::Ambiguous(t));
            }
        };
        let info_snapshot = win.to_info(&screens);
        Ok(FrontWindow {
            screens,
            info_snapshot,
            win,
        })
    })();

    let ctx = match resolved {
        Ok(c) => c,
        Err(ApplyFailure::NoWindow) => {
            return err_exit(json, EXIT_NO_WINDOW, "no target window found", None)
        }
        Err(ApplyFailure::Ambiguous(t)) => {
            return err_exit(
                json,
                EXIT_AMBIGUOUS,
                &format!("--window matched {} windows", t.len()),
                Some(&format!("matches: {}", t.join(" | "))),
            );
        }
        Err(ApplyFailure::Backend(m)) => return err_exit(json, EXIT_BACKEND, &m, None),
    };

    let screen_idx = wnav_core::screen_of(&ctx.info_snapshot, &ctx.screens);
    let to = match geometry(id, &ctx.info_snapshot, screen_idx, &ctx.screens) {
        Ok(r) => r,
        Err(wnav_core::LayoutError::SingleDisplay) => {
            return err_exit(
                json,
                EXIT_SINGLE_DISPLAY,
                "only one display is connected",
                Some("next/previous-display need at least two screens"),
            );
        }
        Err(wnav_core::LayoutError::NoScreen) => {
            return err_exit(json, EXIT_BACKEND, "no screen found", None)
        }
    };

    let from = ctx.info_snapshot.frame;
    let ax_err = ctx.win.apply(to);
    if ax_err != 0 {
        return err_exit(
            json,
            EXIT_BACKEND,
            &format!("AX error {ax_err}"),
            Some("window may have refused the move; try again or target another window"),
        );
    }

    let preset_canonical = wnav_core::preset::serde_kebab(id).to_string();
    if json {
        emit(&serde_json::json!({
            "ok": true,
            "preset": preset_canonical,
            "window": {
                "pid": ctx.info_snapshot.pid,
                "title": ctx.info_snapshot.title,
                "app": ctx.info_snapshot.app,
            },
            "from": from,
            "to": to,
            "screen": screen_idx,
        }));
    } else {
        println!(
            "{}: {} — moved ({},{}) {}x{}",
            preset_canonical,
            ctx.info_snapshot.app,
            px(to.x),
            px(to.y),
            px(to.w),
            px(to.h)
        );
    }
    0
}

struct FrontWindow {
    screens: Vec<wnav_core::Screen>,
    info_snapshot: wnav_core::WindowInfo,
    win: wnav_macos::AxWindow,
}

enum ApplyFailure {
    NoWindow,
    Ambiguous(Vec<String>),
    Backend(String),
}

fn permission_denied(json: bool) -> i32 {
    err_exit(
        json,
        EXIT_PERMISSION,
        "Accessibility permission missing",
        Some("System Settings -> Privacy & Security -> Accessibility -> add wnav/Grid"),
    )
}

fn err_exit(json: bool, code: i32, message: &str, hint: Option<&str>) -> i32 {
    if json {
        emit(&JsonErr {
            ok: false,
            error: JsonErr::body(code, message, hint),
        });
    } else {
        eprintln!("error: {message}");
        if let Some(h) = hint {
            eprintln!("hint: {h}");
        }
    }
    code
}

/// Frame coordinate for display: f64 points are well inside i32 for any
/// real display; saturate rather than wrap.
fn px(v: f64) -> i32 {
    if v >= f64::from(i32::MAX) {
        i32::MAX
    } else if v <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        let r = v.round();
        // Bounds-checked above, so this cannot truncate or wrap.
        #[allow(clippy::cast_possible_truncation)]
        let out = r as i32;
        out
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n).collect::<String>() + "…"
    }
}
