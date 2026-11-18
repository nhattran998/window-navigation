//! Screens via NSScreen (frame, visibleFrame, scale) — SRS §4.2 ScreenProvider.

use objc2::MainThreadMarker;

use wnav_core::{Rect, Screen};

/// All connected screens in NSScreen order. Must be called on the main
/// thread (AppKit requirement).
/// All connected screens in NSScreen order (SRS 4.2 ScreenProvider).
/// Must run on the main thread.
pub fn screens() -> Vec<Screen> {
    let Some(mtm) = MainThreadMarker::new() else {
        // Not on the main thread: no safe NSScreen access.
        return Vec::new();
    };
    let all = objc2_app_kit::NSScreen::screens(mtm);
    let primary_height = all.to_vec().first().map_or(0., |p| p.frame().size.height);
    let mut out = Vec::new();
    for s in all.to_vec() {
        let f = s.frame();
        // NSScreen coordinates: origin bottom-left of the primary screen, y up.
        // Convert to top-left origin, y down (CG/AX convention).
        let frame = Rect::new(
            f.origin.x,
            primary_height - f.origin.y - f.size.height,
            f.size.width,
            f.size.height,
        );
        let vf = s.visibleFrame();
        let visible = Rect::new(
            vf.origin.x,
            primary_height - vf.origin.y - vf.size.height,
            vf.size.width,
            vf.size.height,
        );
        out.push(Screen {
            index: out.len(),
            name: s.localizedName().to_string(),
            frame,
            visible,
            scale: s.backingScaleFactor(),
        });
    }
    out
}
