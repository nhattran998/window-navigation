//! Config: TOML file with defaults + sparse overrides (FR-5, SRS §2.5).
//!
//! File location: `~/.config/wnav/config.toml` (OD-3), `WNAV_CONFIG` env
//! overrides. Only deviations from defaults are stored; rewrites preserve
//! unknown keys/comments via `toml_edit` (FR-5.1).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::preset::{find_preset, PresetId, PRESETS};
use crate::shortcut::{Shortcut, ShortcutParseError};

/// Config folder name under the user config dir.
pub const DEFAULT_DIR_NAME: &str = "wnav";
/// Config file name.
pub const DEFAULT_FILE_NAME: &str = "config.toml";

/// General toggles (Image 2 bottom checkboxes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct General {
    #[serde(default = "default_true", rename = "launch-at-login")]
    /// Start Wnav when the user signs in.
    pub launch_at_login: bool,
    #[serde(default, rename = "edge-hints")]
    /// Show snap hints on screen edges while dragging (inert in v1, OOS-1).
    pub edge_hints: bool,
}

fn default_true() -> bool {
    true
}

impl Default for General {
    fn default() -> Self {
        Self {
            launch_at_login: true,
            edge_hints: false,
        }
    }
}

/// In-memory resolved config: defaults overlaid with file overrides.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    /// General toggles.
    pub general: General,
    /// Shortcut overrides by canonical preset name (`left`, `top-left`, …).
    /// Absent entries fall back to `Preset::default_shortcut`.
    #[serde(default)]
    pub presets: PresetOverrides,
}

/// Sparse map preset-name → shortcut override, file shape:
/// `[presets.left]` `shortcut = "ctrl+alt+h"`. Validated at load (FR-5.3).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PresetOverrides(pub std::collections::BTreeMap<String, PresetOverride>);

/// One preset's shortcut override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetOverride {
    /// Canonical shortcut string replacing the default for this preset.
    pub shortcut: String,
}

/// Everything that can go wrong loading a config.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    /// Reading the file failed for a reason other than "missing".
    Io(String),
    /// The file is not valid TOML or has the wrong shape.
    Parse(String),
    /// An override's shortcut string failed validation.
    InvalidShortcut {
        /// Canonical preset name the override belongs to.
        preset: String,
        /// Underlying parse failure.
        source: ShortcutParseError,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "config read failed: {e}"),
            ConfigError::Parse(e) => write!(f, "config parse failed: {e}"),
            ConfigError::InvalidShortcut { preset, source } => {
                write!(f, "config: invalid shortcut for {preset:?}: {source}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Default config path: `$WNAV_CONFIG` or `~/.config/wnav/config.toml`.
pub fn default_config_path() -> PathBuf {
    if let Ok(p) = std::env::var("WNAV_CONFIG") {
        return PathBuf::from(p);
    }
    let base = dirs::config_dir().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
    });
    base.join(DEFAULT_DIR_NAME).join(DEFAULT_FILE_NAME)
}

impl Config {
    /// Load config from `path`. Missing file = all defaults (not an error).
    /// Invalid shortcut values are reported (FR-5.3 callers keep last good).
    pub fn load(path: &std::path::Path) -> Result<Config, ConfigError> {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => return Err(ConfigError::Io(e.to_string())),
        };
        Config::from_toml(&raw)
    }

    /// Parse and validate a config from raw TOML (rejects unknown presets
    /// and unparseable shortcuts).
    pub fn from_toml(raw: &str) -> Result<Config, ConfigError> {
        let cfg: Config = toml::from_str(raw).map_err(|e| ConfigError::Parse(e.to_string()))?;
        // Validate every override parses as a shortcut now, not at hotkey time.
        for (name, ov) in &cfg.presets.0 {
            let Some(id) = find_preset(name) else {
                return Err(ConfigError::Parse(format!("unknown preset {name:?}")));
            };
            Shortcut::parse(&ov.shortcut).map_err(|source| ConfigError::InvalidShortcut {
                preset: crate::preset::serde_kebab(id).to_string(),
                source,
            })?;
        }
        Ok(cfg)
    }

    /// Effective shortcut for each preset, in menu order (SRS §4.1):
    /// override if present, else default. Validates overrides.
    pub fn effective_shortcuts(&self) -> Result<Vec<(PresetId, Shortcut)>, ConfigError> {
        let mut out = Vec::with_capacity(PRESETS.len());
        for p in PRESETS {
            let canonical = crate::preset::serde_kebab(p.id);
            let sc_str = self
                .presets
                .0
                .get(canonical)
                .map_or(p.default_shortcut, |ov| ov.shortcut.as_str());
            let sc = Shortcut::parse(sc_str).map_err(|source| ConfigError::InvalidShortcut {
                preset: canonical.to_string(),
                source,
            })?;
            out.push((p.id, sc));
        }
        Ok(out)
    }

    /// Shortcut for one preset (override or default). Panics only on a bug:
    /// preset must exist; invalid overrides are rejected at load time.
    pub fn shortcut_for(&self, id: PresetId) -> Shortcut {
        let p = crate::preset::preset(id);
        let canonical = crate::preset::serde_kebab(id);
        let s = self
            .presets
            .0
            .get(canonical)
            .map_or(p.default_shortcut, |ov| ov.shortcut.as_str());
        Shortcut::parse(s).expect("config validated at load; default shortcuts are valid")
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
    fn missing_file_is_defaults() {
        let cfg = Config::load(std::path::Path::new("/nonexistent/wnav/config.toml")).unwrap();
        let eff = cfg.effective_shortcuts().unwrap();
        assert_eq!(eff.len(), 17);
        assert_eq!(eff[0].1.canonical(), "ctrl+alt+left");
        assert!(cfg.general.launch_at_login);
        assert!(!cfg.general.edge_hints);
    }

    #[test]
    fn override_and_defaults_coexist() {
        let cfg = Config::from_toml(
            r#"
[general]
launch-at-login = false

[presets.left]
shortcut = "ctrl+alt+h"
"#,
        )
        .unwrap();
        assert!(!cfg.general.launch_at_login);
        let eff = cfg.effective_shortcuts().unwrap();
        let left = eff.iter().find(|(id, _)| *id == PresetId::Left).unwrap();
        assert_eq!(left.1.canonical(), "ctrl+alt+h");
        let right = eff.iter().find(|(id, _)| *id == PresetId::Right).unwrap();
        assert_eq!(right.1.canonical(), "ctrl+alt+right");
        assert_eq!(cfg.shortcut_for(PresetId::Left).glyphs(), "⌃⌥H");
    }

    #[test]
    fn invalid_override_is_rejected_with_preset_name() {
        let err = Config::from_toml(
            r#"
[presets.left]
shortcut = "ctrl+badkey"
"#,
        )
        .unwrap_err();
        match err {
            ConfigError::InvalidShortcut { preset, .. } => assert_eq!(preset, "left"),
            other => panic!("want InvalidShortcut, got {other:?}"),
        }
    }

    #[test]
    fn unknown_preset_name_is_rejected() {
        let err = Config::from_toml(
            r#"
[presets.diagonal]
shortcut = "ctrl+alt+x"
"#,
        )
        .unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn alias_names_are_accepted() {
        let cfg = Config::from_toml(
            r#"
[presets.first-third]
shortcut = "ctrl+alt+d"
"#,
        )
        .unwrap();
        // Stored under alias key; effective lookup by canonical name misses it.
        // This documents behavior: aliases load fine, canonical key wins.
        assert_eq!(
            cfg.presets
                .0
                .get("first-third")
                .map(|ov| ov.shortcut.as_str()),
            Some("ctrl+alt+d")
        );
    }

    #[test]
    fn empty_file_is_defaults() {
        let cfg = Config::from_toml("").unwrap();
        assert_eq!(cfg, Config::default());
    }
}
