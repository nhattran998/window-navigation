//! Keymap editing model (FR-3 + item 3: Preferences keymaps).
//!
//! The recorder captures a gpui `Keystroke` on key-down; we translate it into
//! a wnav-core `Shortcut`, validate, persist to config.toml, and re-register
//! hotkeys live (AC-F1-07).

use gpui_kit::gpui::{Keystroke, Modifiers};
use wnav_core::shortcut::{Key, Shortcut};
use wnav_core::PresetId;

/// What the recorder produced for one capture session.
#[derive(Debug, Clone)]
pub(crate) enum Capture {
    /// A full shortcut (at least one modifier + one key).
    Ready(Shortcut),
    /// Only modifiers so far — keep listening.
    Pending,
    /// Esc pressed — cancel without changing anything.
    Cancelled,
}

/// Translate a captured gpui keystroke into our model.
pub(crate) fn capture(keystroke: &Keystroke) -> Capture {
    // Esc cancels (HIG settings.md: recorder always has an obvious way out).
    if keystroke.key == "escape" && keystroke.modifiers == Modifiers::default() {
        return Capture::Cancelled;
    }

    let Some(key) = parse_key(&keystroke.key) else {
        return Capture::Pending;
    };

    let m = &keystroke.modifiers;
    let sc = Shortcut {
        ctrl: m.control,
        alt: m.alt,
        shift: m.shift,
        // On macOS `platform` is the Command key (gpui docs on Modifiers).
        cmd: m.platform,
        key,
    };
    if !sc.ctrl && !sc.alt && !sc.shift && !sc.cmd {
        // Bare key: swallow typing guards forbid it; keep listening.
        return Capture::Pending;
    }
    Capture::Ready(sc)
}

/// gpui key name → our Key enum (subset that our hotkeys support).
fn parse_key(gpui_key: &str) -> Option<Key> {
    // Exhaustiveness enforced by the return type; glob keeps arms short.
    #[allow(clippy::enum_glob_use)]
    use Key::*;
    Some(match gpui_key {
        "a" => A,
        "b" => B,
        "c" => C,
        "d" => D,
        "e" => E,
        "f" => F,
        "g" => G,
        "h" => H,
        "i" => I,
        "j" => J,
        "k" => K,
        "l" => L,
        "m" => M,
        "n" => N,
        "o" => O,
        "p" => P,
        "q" => Q,
        "r" => R,
        "s" => S,
        "t" => T,
        "u" => U,
        "v" => V,
        "w" => W,
        "x" => X,
        "y" => Y,
        "z" => Z,
        "0" => Num0,
        "1" => Num1,
        "2" => Num2,
        "3" => Num3,
        "4" => Num4,
        "5" => Num5,
        "6" => Num6,
        "7" => Num7,
        "8" => Num8,
        "9" => Num9,
        "left" => Left,
        "right" => Right,
        "up" => Up,
        "down" => Down,
        "enter" => Enter,
        "tab" => Tab,
        "space" => Space,
        "escape" => Esc,
        "-" => Minus,
        "=" => Equal,
        k if k.starts_with('f') && k.len() > 1 => {
            let n: u32 = k.strip_prefix('f')?.parse().ok()?;
            match n {
                1 => F1,
                2 => F2,
                3 => F3,
                4 => F4,
                5 => F5,
                6 => F6,
                7 => F7,
                8 => F8,
                9 => F9,
                10 => F10,
                11 => F11,
                12 => F12,
                13 => F13,
                14 => F14,
                15 => F15,
                16 => F16,
                17 => F17,
                18 => F18,
                19 => F19,
                _ => return None,
            }
        }
        _ => return None,
    })
}

/// Persist one preset's shortcut override and return the full config.
/// Round-trip safe: toml_edit keeps comments/unknown keys (FR-5.1).
pub(crate) fn save_shortcut(id: PresetId, sc: Shortcut) -> Result<wnav_core::Config, String> {
    let path = wnav_core::config::default_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = std::fs::read_to_string(&path).unwrap_or_default();

    let canonical = wnav_core::preset::serde_kebab(id);
    let mut doc = raw
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("config parse: {e}"))?;
    let presets = doc
        .entry("presets")
        .or_insert(toml_edit::table())
        .as_table_mut()
        .expect("presets is a table");
    let row = presets
        .entry(canonical)
        .or_insert(toml_edit::table())
        .as_table_mut()
        .expect("preset row is a table");
    row.insert("shortcut", toml_edit::value(sc.canonical()));
    std::fs::write(&path, doc.to_string()).map_err(|e| e.to_string())?;

    // Reload through the validating path so a bad write fails loudly here.
    wnav_core::Config::load(&path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    // Tests exercise panics directly; the production deny-set doesn't apply.
    #![allow(clippy::unwrap_used)]
    use super::*;

    // Test helper mirrors the Shortcut model; 4 modifiers are inherent.
    #[allow(clippy::type_complexity, clippy::struct_excessive_bools)]
    fn ks(key: &'static str, (ctrl, alt, shift, cmd): (bool, bool, bool, bool)) -> Keystroke {
        Keystroke {
            modifiers: Modifiers {
                control: ctrl,
                alt,
                shift,
                platform: cmd,
                function: false,
            },
            key: key.into(),
            key_char: None,
        }
    }

    #[test]
    fn capture_full_shortcut() {
        let out = capture(&ks("k", (true, true, false, false)));
        match out {
            Capture::Ready(sc) => assert_eq!(sc.canonical(), "ctrl+alt+k"),
            Capture::Pending | Capture::Cancelled => panic!("want Ready"),
        }
    }

    #[test]
    fn capture_bare_key_stays_pending() {
        assert!(matches!(
            capture(&ks("k", (false, false, false, false))),
            Capture::Pending
        ));
    }

    #[test]
    fn escape_cancels() {
        assert!(matches!(
            capture(&ks("escape", (false, false, false, false))),
            Capture::Cancelled
        ));
    }

    #[test]
    fn f_keys_and_arrows_parse() {
        let out = capture(&ks("f5", (true, false, false, false)));
        assert!(matches!(out, Capture::Ready(sc) if sc.canonical() == "ctrl+f5"));
        let out = capture(&ks("left", (true, true, false, false)));
        assert!(matches!(out, Capture::Ready(sc) if sc.canonical() == "ctrl+alt+left"));
    }

    #[test]
    fn cmd_maps_to_super() {
        let out = capture(&ks("right", (true, true, false, true)));
        assert!(matches!(out, Capture::Ready(sc) if sc.canonical() == "ctrl+alt+cmd+right"));
    }
}
