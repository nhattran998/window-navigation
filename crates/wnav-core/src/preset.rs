use serde::{Deserialize, Serialize};

/// Identity of one of the 17 window positions (PRD S-01/S-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum PresetId {
    /// Snap to the left half of the visible frame.
    Left,
    /// Snap to the right half.
    Right,
    /// Snap to the top half.
    Top,
    /// Snap to the bottom half.
    Bottom,
    /// Snap to the top-left quarter.
    TopLeft,
    /// Snap to the top-right quarter.
    TopRight,
    /// Snap to the bottom-left quarter.
    BottomLeft,
    /// Snap to the bottom-right quarter.
    BottomRight,
    /// Snap to the left third of the visible frame.
    LeftThird,
    /// Snap to the left two thirds.
    LeftTwoThirds,
    /// Snap to the middle third.
    CenterThird,
    /// Snap to the right two thirds.
    RightTwoThirds,
    /// Snap to the right third.
    RightThird,
    /// Fill the visible frame exactly.
    Maximize,
    /// Keep the window size, center it in the visible frame.
    Center,
    /// Move the window to the next display, preserving relative position.
    NextDisplay,
    /// Move the window to the previous display.
    PreviousDisplay,
}

/// Metadata for one preset: id, menu label, default shortcut.
#[derive(Debug)]
pub struct Preset {
    /// Stable identifier used by CLI, config and hotkeys.
    pub id: PresetId,
    /// Menu label (Image 1 of the PRD).
    pub label: &'static str,
    /// Canonical default shortcut, e.g. "ctrl+alt+left" (Image 2).
    pub default_shortcut: &'static str,
    /// Preset needs >= 2 screens (next/previous display).
    pub needs_two_screens: bool,
}

/// The 17 presets in menu order (Image 1 of the PRD).
pub const PRESETS: &[Preset] = &[
    Preset {
        id: PresetId::Left,
        label: "Left",
        default_shortcut: "ctrl+alt+left",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::Right,
        label: "Right",
        default_shortcut: "ctrl+alt+right",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::Top,
        label: "Top",
        default_shortcut: "ctrl+alt+up",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::Bottom,
        label: "Bottom",
        default_shortcut: "ctrl+alt+down",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::TopLeft,
        label: "Top Left",
        default_shortcut: "ctrl+alt+u",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::TopRight,
        label: "Top Right",
        default_shortcut: "ctrl+alt+i",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::BottomLeft,
        label: "Bottom Left",
        default_shortcut: "ctrl+alt+j",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::BottomRight,
        label: "Bottom Right",
        default_shortcut: "ctrl+alt+k",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::LeftThird,
        label: "Left Third",
        default_shortcut: "ctrl+alt+d",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::LeftTwoThirds,
        label: "Left Two Thirds",
        default_shortcut: "ctrl+alt+e",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::CenterThird,
        label: "Center Third",
        default_shortcut: "ctrl+alt+f",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::RightTwoThirds,
        label: "Right Two Thirds",
        default_shortcut: "ctrl+alt+t",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::RightThird,
        label: "Right Third",
        default_shortcut: "ctrl+alt+g",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::Maximize,
        label: "Maximize",
        default_shortcut: "ctrl+alt+enter",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::Center,
        label: "Center",
        default_shortcut: "ctrl+alt+c",
        needs_two_screens: false,
    },
    Preset {
        id: PresetId::NextDisplay,
        label: "Next Display",
        default_shortcut: "ctrl+alt+cmd+right",
        needs_two_screens: true,
    },
    Preset {
        id: PresetId::PreviousDisplay,
        label: "Previous Display",
        default_shortcut: "ctrl+alt+cmd+left",
        needs_two_screens: true,
    },
];

/// Metadata for one preset.
pub fn preset(id: PresetId) -> &'static Preset {
    PRESETS
        .iter()
        .find(|p| p.id == id)
        .expect("PRESETS is exhaustive")
}

const ALIASES: &[(&str, PresetId)] = &[
    ("first-third", PresetId::LeftThird),
    ("first-two-thirds", PresetId::LeftTwoThirds),
    ("last-two-thirds", PresetId::RightTwoThirds),
    ("last-third", PresetId::RightThird),
    ("left-half", PresetId::Left),
    ("right-half", PresetId::Right),
];

/// Resolve a CLI/config name to a preset id. Canonical kebab-case ids win,
/// then aliases (PRD Image 2 labels vs Image 1 labels).
pub fn find_preset(name: &str) -> Option<PresetId> {
    let lower = name.trim().to_ascii_lowercase().replace(' ', "-");
    for p in PRESETS {
        if serde_kebab(p.id) == lower {
            return Some(p.id);
        }
    }
    ALIASES.iter().find(|(a, _)| *a == lower).map(|(_, id)| *id)
}

/// Canonical kebab-case name of a preset (serde matches this).
pub fn serde_kebab(id: PresetId) -> &'static str {
    match id {
        PresetId::Left => "left",
        PresetId::Right => "right",
        PresetId::Top => "top",
        PresetId::Bottom => "bottom",
        PresetId::TopLeft => "top-left",
        PresetId::TopRight => "top-right",
        PresetId::BottomLeft => "bottom-left",
        PresetId::BottomRight => "bottom-right",
        PresetId::LeftThird => "left-third",
        PresetId::LeftTwoThirds => "left-two-thirds",
        PresetId::CenterThird => "center-third",
        PresetId::RightTwoThirds => "right-two-thirds",
        PresetId::RightThird => "right-third",
        PresetId::Maximize => "maximize",
        PresetId::Center => "center",
        PresetId::NextDisplay => "next-display",
        PresetId::PreviousDisplay => "previous-display",
    }
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
    fn seventeen_presets() {
        assert_eq!(PRESETS.len(), 17);
    }

    #[test]
    fn canonical_names_resolve() {
        for p in PRESETS {
            assert_eq!(find_preset(serde_kebab(p.id)), Some(p.id), "{}", p.label);
        }
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(find_preset("First Third"), Some(PresetId::LeftThird));
        assert_eq!(
            find_preset("last-two-thirds"),
            Some(PresetId::RightTwoThirds)
        );
        assert_eq!(find_preset("  LEFT  "), Some(PresetId::Left));
        assert_eq!(find_preset("nope"), None);
    }

    #[test]
    fn display_rows_match_image1_order() {
        let labels: Vec<&str> = PRESETS.iter().map(|p| p.label).collect();
        assert_eq!(
            labels,
            vec![
                "Left",
                "Right",
                "Top",
                "Bottom",
                "Top Left",
                "Top Right",
                "Bottom Left",
                "Bottom Right",
                "Left Third",
                "Left Two Thirds",
                "Center Third",
                "Right Two Thirds",
                "Right Third",
                "Maximize",
                "Center",
                "Next Display",
                "Previous Display",
            ]
        );
    }

    #[test]
    fn kebab_serde_roundtrip() {
        for p in PRESETS {
            let s = serde_json::to_string(&p.id).unwrap();
            assert_eq!(s.trim_matches('"'), serde_kebab(p.id));
            let back: PresetId = serde_json::from_str(&s).unwrap();
            assert_eq!(back, p.id);
        }
    }
}
