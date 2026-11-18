//! Shortcut model + canonical string parsing (FR-3.2).
//!
//! Canonical form: lowercase `mod+mod+key`, mods in fixed order
//! `ctrl`, `alt`, `shift`, `cmd`; key = letter/digit, `f<n>`, arrow, or named
//! key (`enter`, `tab`, `space`, `esc`, `minus`, `equal`). Display form uses
//! macOS glyphs: `^⌥K`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
/// One keyboard shortcut: modifiers plus a key (FR-3.2).
pub struct Shortcut {
    /// Control modifier.
    pub ctrl: bool,
    /// Alt/Option modifier.
    pub alt: bool,
    /// Shift modifier.
    pub shift: bool,
    /// Command/Super modifier.
    pub cmd: bool,
    /// The non-modifier key.
    pub key: Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// A non-modifier key a shortcut can bind.
pub enum Key {
    /// Letter A
    A,
    /// Letter B
    B,
    /// Letter C
    C,
    /// Letter D
    D,
    /// Letter E
    E,
    /// Function key .
    F,
    /// Letter G
    G,
    /// Letter H
    H,
    /// Letter I
    I,
    /// Letter J
    J,
    /// Letter K
    K,
    /// Letter L
    L,
    /// Letter M
    M,
    /// Letter N
    N,
    /// Letter O
    O,
    /// Letter P
    P,
    /// Letter Q
    Q,
    /// Letter R
    R,
    /// Letter S
    S,
    /// Letter T
    T,
    /// Letter U
    U,
    /// Letter V
    V,
    /// Letter W
    W,
    /// Letter X
    X,
    /// Letter Y
    Y,
    /// Letter Z
    Z,
    /// Digit 0
    Num0,
    /// Digit 1
    Num1,
    /// Digit 2
    Num2,
    /// Digit 3
    Num3,
    /// Digit 4
    Num4,
    /// Digit 5
    Num5,
    /// Digit 6
    Num6,
    /// Digit 7
    Num7,
    /// Digit 8
    Num8,
    /// Digit 9
    Num9,
    /// Function key 1.
    F1,
    /// Function key 2.
    F2,
    /// Function key 3.
    F3,
    /// Function key 4.
    F4,
    /// Function key 5.
    F5,
    /// Function key 6.
    F6,
    /// Function key 7.
    F7,
    /// Function key 8.
    F8,
    /// Function key 9.
    F9,
    /// Function key 10.
    F10,
    /// Function key 11.
    F11,
    /// Function key 12.
    F12,
    /// Function key 13.
    F13,
    /// Function key 14.
    F14,
    /// Function key 15.
    F15,
    /// Function key 16.
    F16,
    /// Function key 17.
    F17,
    /// Function key 18.
    F18,
    /// Function key 19.
    F19,
    /// Arrow left
    Left,
    /// Arrow right
    Right,
    /// Arrow up
    Up,
    /// Arrow down
    Down,
    /// Return/Enter
    Enter,
    /// Tab
    Tab,
    /// Spacebar
    Space,
    /// Escape
    Esc,
    /// Hyphen/minus
    Minus,
    /// Equals sign
    Equal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Failed to parse a shortcut string; carries the offending input.
pub struct ShortcutParseError(pub String);

impl std::fmt::Display for ShortcutParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid shortcut {:?}: expected e.g. ctrl+alt+k, ctrl+alt+left",
            self.0
        )
    }
}

impl std::error::Error for ShortcutParseError {}

impl Key {
    /// Parse a gpui/canonical key name; case-insensitive for letters.
    pub fn parse(s: &str) -> Option<Key> {
        // Match arms name every variant; a glob keeps the list short and the
        // compiler still enforces exhaustiveness via the return type.
        #[allow(clippy::enum_glob_use)]
        use Key::*;
        Some(match s.to_ascii_lowercase().as_str() {
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
            "0" | "num0" => Num0,
            "1" | "num1" => Num1,
            "2" | "num2" => Num2,
            "3" | "num3" => Num3,
            "4" | "num4" => Num4,
            "5" | "num5" => Num5,
            "6" | "num6" => Num6,
            "7" | "num7" => Num7,
            "8" | "num8" => Num8,
            "9" | "num9" => Num9,
            "f1" => F1,
            "f2" => F2,
            "f3" => F3,
            "f4" => F4,
            "f5" => F5,
            "f6" => F6,
            "f7" => F7,
            "f8" => F8,
            "f9" => F9,
            "f10" => F10,
            "f11" => F11,
            "f12" => F12,
            "f13" => F13,
            "f14" => F14,
            "f15" => F15,
            "f16" => F16,
            "f17" => F17,
            "f18" => F18,
            "f19" => F19,
            "left" => Left,
            "right" => Right,
            "up" => Up,
            "down" => Down,
            "enter" | "return" => Enter,
            "tab" => Tab,
            "space" => Space,
            "esc" | "escape" => Esc,
            "minus" | "-" => Minus,
            "equal" | "=" => Equal,
            _ => return None,
        })
    }

    /// Name used in the canonical string form.
    pub fn canonical(self) -> &'static str {
        #[allow(clippy::enum_glob_use)]
        use Key::*;
        match self {
            A => "a",
            B => "b",
            C => "c",
            D => "d",
            E => "e",
            F => "f",
            G => "g",
            H => "h",
            I => "i",
            J => "j",
            K => "k",
            L => "l",
            M => "m",
            N => "n",
            O => "o",
            P => "p",
            Q => "q",
            R => "r",
            S => "s",
            T => "t",
            U => "u",
            V => "v",
            W => "w",
            X => "x",
            Y => "y",
            Z => "z",
            Num0 => "0",
            Num1 => "1",
            Num2 => "2",
            Num3 => "3",
            Num4 => "4",
            Num5 => "5",
            Num6 => "6",
            Num7 => "7",
            Num8 => "8",
            Num9 => "9",
            F1 => "f1",
            F2 => "f2",
            F3 => "f3",
            F4 => "f4",
            F5 => "f5",
            F6 => "f6",
            F7 => "f7",
            F8 => "f8",
            F9 => "f9",
            F10 => "f10",
            F11 => "f11",
            F12 => "f12",
            F13 => "f13",
            F14 => "f14",
            F15 => "f15",
            F16 => "f16",
            F17 => "f17",
            F18 => "f18",
            F19 => "f19",
            Left => "left",
            Right => "right",
            Up => "up",
            Down => "down",
            Enter => "enter",
            Tab => "tab",
            Space => "space",
            Esc => "esc",
            Minus => "minus",
            Equal => "equal",
        }
    }

    /// macOS glyph for menu/Preferences display (Image 1/2 of the PRD).
    pub fn glyph(self) -> &'static str {
        #[allow(clippy::enum_glob_use)]
        use Key::*;
        match self {
            A => "A",
            B => "B",
            C => "C",
            D => "D",
            E => "E",
            F => "F",
            G => "G",
            H => "H",
            I => "I",
            J => "J",
            K => "K",
            L => "L",
            M => "M",
            N => "N",
            O => "O",
            P => "P",
            Q => "Q",
            R => "R",
            S => "S",
            T => "T",
            U => "U",
            V => "V",
            W => "W",
            X => "X",
            Y => "Y",
            Z => "Z",
            Num0 => "0",
            Num1 => "1",
            Num2 => "2",
            Num3 => "3",
            Num4 => "4",
            Num5 => "5",
            Num6 => "6",
            Num7 => "7",
            Num8 => "8",
            Num9 => "9",
            F1 => "F1",
            F2 => "F2",
            F3 => "F3",
            F4 => "F4",
            F5 => "F5",
            F6 => "F6",
            F7 => "F7",
            F8 => "F8",
            F9 => "F9",
            F10 => "F10",
            F11 => "F11",
            F12 => "F12",
            F13 => "F13",
            F14 => "F14",
            F15 => "F15",
            F16 => "F16",
            F17 => "F17",
            F18 => "F18",
            F19 => "F19",
            Left => "←",
            Right => "→",
            Up => "↑",
            Down => "↓",
            Enter => "↩",
            Tab => "⇥",
            Space => "␣",
            Esc => "⎋",
            Minus => "-",
            Equal => "=",
        }
    }
}

impl Shortcut {
    /// Parse canonical form: `ctrl+alt+k`, `cmd+shift+f5`, `ctrl+alt+left`.
    /// Mods in any order, lowercased; unknown tokens error (FR-3.2).
    pub fn parse(s: &str) -> Result<Shortcut, ShortcutParseError> {
        let mut sc = Shortcut {
            ctrl: false,
            alt: false,
            shift: false,
            cmd: false,
            key: Key::Space, // placeholder; replaced below
        };
        let mut has_key = false;
        for tok in s.split('+').map(str::trim).filter(|t| !t.is_empty()) {
            match tok.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => sc.ctrl = true,
                "alt" | "option" => sc.alt = true,
                "shift" => sc.shift = true,
                "cmd" | "command" | "super" | "meta" => sc.cmd = true,
                k => match Key::parse(k) {
                    Some(key) if !has_key => {
                        sc.key = key;
                        has_key = true;
                    }
                    Some(_) => {
                        return Err(ShortcutParseError(s.into())); // two keys
                    }
                    None => return Err(ShortcutParseError(s.into())),
                },
            }
        }
        if !has_key {
            return Err(ShortcutParseError(s.into()));
        }
        if !sc.ctrl && !sc.alt && !sc.shift && !sc.cmd {
            // Bare keys would swallow normal typing; require >= 1 modifier.
            return Err(ShortcutParseError(s.into()));
        }
        Ok(sc)
    }

    /// Canonical string: mods in fixed order ctrl, alt, shift, cmd (FR-3.2).
    pub fn canonical(&self) -> String {
        let mut parts = Vec::with_capacity(5);
        if self.ctrl {
            parts.push("ctrl");
        }
        if self.alt {
            parts.push("alt");
        }
        if self.shift {
            parts.push("shift");
        }
        if self.cmd {
            parts.push("cmd");
        }
        parts.push(self.key.canonical());
        parts.join("+")
    }

    /// Display form with macOS glyphs: `^⌥K`, `^⌥⌘→` (menu column, Image 1).
    pub fn glyphs(&self) -> String {
        let mut out = String::new();
        if self.ctrl {
            out.push('⌃');
        }
        if self.alt {
            out.push('⌥');
        }
        if self.shift {
            out.push('⇧');
        }
        if self.cmd {
            out.push('⌘');
        }
        out.push_str(self.key.glyph());
        out
    }
}

/// Free function per SRS §4.1 API sketch.
pub fn parse_shortcut(s: &str) -> Result<Shortcut, ShortcutParseError> {
    Shortcut::parse(s)
}

#[cfg(test)]
mod tests {
    // Tests exercise panics directly; the production deny-set doesn't apply.
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::wildcard_enum_match_arm,
        clippy::many_single_char_names,
        clippy::map_unwrap_or
    )]
    use super::*;

    #[test]
    fn defaults_from_prd_parse_and_roundtrip() {
        for p in crate::PRESETS {
            let sc = Shortcut::parse(p.default_shortcut)
                .unwrap_or_else(|e| panic!("{}: {e}", p.default_shortcut));
            assert_eq!(sc.canonical(), p.default_shortcut);
        }
    }

    #[test]
    fn glyph_forms_match_menu() {
        assert_eq!(Shortcut::parse("ctrl+alt+k").unwrap().glyphs(), "⌃⌥K");
        assert_eq!(Shortcut::parse("ctrl+alt+right").unwrap().glyphs(), "⌃⌥→");
        assert_eq!(
            Shortcut::parse("ctrl+alt+cmd+right").unwrap().glyphs(),
            "⌃⌥⌘→"
        );
        assert_eq!(Shortcut::parse("ctrl+alt+enter").unwrap().glyphs(), "⌃⌥↩");
    }

    #[test]
    fn modifier_order_and_case_insensitive() {
        let a = Shortcut::parse("ALT+CTRL+K").unwrap();
        let b = Shortcut::parse("ctrl+alt+k").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.canonical(), "ctrl+alt+k");
        assert_eq!(
            Shortcut::parse("option+control+j").unwrap().canonical(),
            "ctrl+alt+j"
        );
    }

    #[test]
    fn rejects_bare_key_double_key_and_garbage() {
        assert!(Shortcut::parse("k").is_err(), "bare key must be rejected");
        assert!(
            Shortcut::parse("ctrl+a+b").is_err(),
            "two keys must be rejected"
        );
        assert!(Shortcut::parse("ctrl+").is_err());
        assert!(Shortcut::parse("ctrl+nope").is_err());
        assert!(Shortcut::parse("").is_err());
        assert!(Shortcut::parse("++").is_err());
    }
}
