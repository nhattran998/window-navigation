//! wnav-core: platform-agnostic preset table, geometry math, shortcuts, config.
//!
//! Pure functions only — no FFI, no async. Backends (wnav-macos, future
//! wnav-windows/wnav-linux) adapt platform APIs to the types here.

/// Config file model: defaults + sparse overrides, TOML round-trip.
pub mod config;
/// Rect math and preset geometry (all coordinate systems documented in SRS).
pub mod geometry;
/// The 17-preset table, ids, labels, aliases (PRD S-01/S-02).
pub mod preset;
/// Shortcut model, canonical strings and glyph display (FR-3.2).
pub mod shortcut;

pub use config::{Config, ConfigError};
pub use geometry::{geometry, screen_of, LayoutError, Rect, Screen, WindowInfo};
pub use preset::{find_preset, preset, Preset, PresetId, PRESETS};
pub use shortcut::{parse_shortcut, Shortcut, ShortcutParseError};
