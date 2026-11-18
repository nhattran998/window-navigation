//! Design tokens for Wnav's Apple-minimal UI (HIG: `designing-for-macos.md`,
//! `settings.md`, `typography.md`). Semantic roles only — never hard-coded
//! system colors; gpui-kit's theme supplies the platform palette and we layer
//! layout constants on top.
#![cfg(target_os = "macos")]

/// Settings window metrics (HIG settings.md: fixed toolbar window, generous
/// whitespace, two-level grouping).
pub(crate) mod metrics {
    /// Window width — settings pages read best narrow and tall.
    pub(crate) const WINDOW_WIDTH: f32 = 560.0;
    pub(crate) const WINDOW_HEIGHT: f32 = 860.0;
    /// Outer padding; row inset inside a group.
    pub(crate) const PAD_OUTER: f32 = 20.0;
    pub(crate) const PAD_ROW: f32 = 12.0;
    /// Vertical rhythm: rows breathe, groups separate clearly.
    pub(crate) const ROW_GAP: f32 = 4.0;
    pub(crate) const GROUP_GAP: f32 = 24.0;
    /// Shortcut capture field height.
    pub(crate) const CAPTURE_HEIGHT: f32 = 28.0;
}

/// Type scale (HIG typography.md: desktop body 13 pt, secondary 11 pt).
pub(crate) mod typography {
    /// Group headers: small caps feel via secondary color + size.
    pub(crate) const GROUP_HEADER: f32 = 11.0;
    /// Row labels and body copy.
    pub(crate) const BODY: f32 = 13.0;
    /// Shortcut glyphs and hints.
    pub(crate) const CAPTION: f32 = 11.0;
}
