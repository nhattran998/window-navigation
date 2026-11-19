use serde::{Deserialize, Serialize};

use crate::preset::PresetId;

/// Axis-aligned rectangle in points (logical coordinates, y down, top-left origin
/// as macOS CGWindow/AX reports). Points, not pixels: scale handled by backends.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Left edge, global points.
    pub x: f64,
    /// Top edge, global points.
    pub y: f64,
    /// Width in points.
    pub w: f64,
    /// Height in points.
    pub h: f64,
}

impl Rect {
    /// Build a rect from its edges.
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// Right edge (`x + w`).
    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    /// Bottom edge (`y + h`).
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    /// Center point.
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }

    /// Whether the point lies inside (right/bottom-exclusive).
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Overlap area; zero-size when disjoint.
    #[must_use]
    pub fn intersection(&self, other: &Rect) -> Rect {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());
        Rect::new(x1, y1, (x2 - x1).max(0.), (y2 - y1).max(0.))
    }
}

/// One display as reported by the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screen {
    /// 0-based index, stable within one call.
    /// 0-based position in the backend's screen list.
    pub index: usize,
    /// Human-readable display name.
    pub name: String,
    /// Full frame in points (global coordinates).
    pub frame: Rect,
    /// Visible frame (menu bar/dock excluded) (FR-1.2).
    pub visible: Rect,
    /// Backing scale factor (2.0 on Retina).
    pub scale: f64,
}

/// One movable window as reported by the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// Owning process id.
    pub pid: i32,
    /// Window title as reported by the backend.
    pub title: String,
    /// Owning application display name.
    pub app: String,
    /// Current frame in points.
    pub frame: Rect,
    /// Whether the window is minimized to the dock/taskbar.
    pub minimized: bool,
    /// Index into the screens list of the screen with the largest overlap.
    pub screen: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
/// Layout computation failures surfaced to CLI/app.
pub enum LayoutError {
    /// Preset requires >= 2 screens (FR-1.4, CLI exit 6).
    SingleDisplay,
    /// No screen could be resolved.
    NoScreen,
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::SingleDisplay => write!(f, "only one display is connected"),
            LayoutError::NoScreen => write!(f, "no screen found"),
        }
    }
}

/// One band along an axis. The end edge at `frac_end >= 1.0` anchors to the
/// absolute frame edge so adjacent bands stay gap-free (FR-1.1): interior
/// edges round from the left/top, the final band absorbs rounding residue.
fn band(from: f64, len: f64, frac_start: f64, frac_end: f64, abs_end: f64) -> (f64, f64) {
    let start = from + (len * frac_start).round();
    let end = if frac_end >= 1.0 {
        abs_end
    } else {
        from + (len * frac_end).round()
    };
    (start, (end - start).max(1.))
}

fn col(v: &Rect, frac_start: f64, frac_end: f64) -> (f64, f64) {
    band(v.x, v.w, frac_start, frac_end, v.right())
}

fn row(v: &Rect, frac_start: f64, frac_end: f64) -> (f64, f64) {
    band(v.y, v.h, frac_start, frac_end, v.bottom())
}

fn split(v: &Rect, col_start: f64, col_end: f64, row_start: f64, row_end: f64) -> Rect {
    let (col_x, col_w) = col(v, col_start, col_end);
    let (row_y, row_h) = row(v, row_start, row_end);
    Rect::new(col_x, row_y, col_w, row_h)
}

/// Screen the window currently lives on (largest visible-frame overlap wins,
/// fallback: screen containing the window center, fallback: screen 0).
pub fn screen_of(w: &WindowInfo, screens: &[Screen]) -> usize {
    if screens.is_empty() {
        return 0;
    }
    let mut best = 0usize;
    let mut best_area = -1f64;
    for (i, s) in screens.iter().enumerate() {
        let area = w.frame.intersection(&s.frame).w * w.frame.intersection(&s.frame).h;
        if area > best_area {
            best_area = area;
            best = i;
        }
    }
    if best_area <= 0. {
        let (cx, cy) = w.frame.center();
        for (i, s) in screens.iter().enumerate() {
            if s.frame.contains_point(cx, cy) {
                return i;
            }
        }
        return 0;
    }
    best
}

/// Target screen index for next/previous display (mod n, FR-1.4).
fn adjacent(current: usize, screens: usize, next: bool) -> usize {
    if screens <= 1 {
        return current;
    }
    if next {
        (current + 1) % screens
    } else {
        (current + screens - 1) % screens
    }
}

/// Compute the target frame for `id` on the window's (or explicit) screen.
/// Pure: backends apply the returned rect via AX/Win32/X11 (SRS §4.2).
pub fn geometry(
    id: PresetId,
    w: &WindowInfo,
    screen_of_window: usize,
    screens: &[Screen],
) -> Result<Rect, LayoutError> {
    let screen_idx = match id {
        PresetId::NextDisplay | PresetId::PreviousDisplay => {
            if screens.len() < 2 {
                return Err(LayoutError::SingleDisplay);
            }
            let next = id == PresetId::NextDisplay;
            adjacent(screen_of_window, screens.len(), next)
        }
        PresetId::Left
        | PresetId::Right
        | PresetId::Top
        | PresetId::Bottom
        | PresetId::TopLeft
        | PresetId::TopRight
        | PresetId::BottomLeft
        | PresetId::BottomRight
        | PresetId::LeftThird
        | PresetId::LeftTwoThirds
        | PresetId::CenterThird
        | PresetId::RightTwoThirds
        | PresetId::RightThird
        | PresetId::Maximize
        | PresetId::Center => screen_of_window,
    };
    let Some(screen) = screens.get(screen_idx) else {
        return Err(LayoutError::NoScreen);
    };
    let v = &screen.visible;

    let rect = match id {
        PresetId::Left => split(v, 0., 1. / 2., 0., 1.),
        PresetId::Right => split(v, 1. / 2., 1., 0., 1.),
        PresetId::Top => split(v, 0., 1., 0., 1. / 2.),
        PresetId::Bottom => split(v, 0., 1., 1. / 2., 1.),
        PresetId::TopLeft => split(v, 0., 1. / 2., 0., 1. / 2.),
        PresetId::TopRight => split(v, 1. / 2., 1., 0., 1. / 2.),
        PresetId::BottomLeft => split(v, 0., 1. / 2., 1. / 2., 1.),
        PresetId::BottomRight => split(v, 1. / 2., 1., 1. / 2., 1.),
        PresetId::LeftThird => split(v, 0., 1. / 3., 0., 1.),
        PresetId::LeftTwoThirds => split(v, 0., 2. / 3., 0., 1.),
        PresetId::CenterThird => split(v, 1. / 3., 2. / 3., 0., 1.),
        PresetId::RightTwoThirds => split(v, 1. / 3., 1., 0., 1.),
        PresetId::RightThird => split(v, 2. / 3., 1., 0., 1.),
        PresetId::Maximize => *v,
        PresetId::Center => {
            // Keep size, center within visible frame; clamp_into finalizes (FR-1.5).
            let cw = w.frame.w.min(v.w);
            let ch = w.frame.h.min(v.h);
            let x = v.x + ((v.w - cw) / 2.).round();
            let y = v.y + ((v.h - ch) / 2.).round();
            Rect::new(x, y, cw, ch)
        }
        PresetId::NextDisplay | PresetId::PreviousDisplay => {
            // Preserve relative position/size within the old visible frame.
            let Some(cur_screen) = screens.get(screen_of_window) else {
                return Err(LayoutError::NoScreen);
            };
            let cur = &cur_screen.visible;
            let rel_x = if cur.w > 0. {
                (w.frame.x - cur.x) / cur.w
            } else {
                0.
            };
            let rel_y = if cur.h > 0. {
                (w.frame.y - cur.y) / cur.h
            } else {
                0.
            };
            let rw = if cur.w > 0. { w.frame.w / cur.w } else { 1. };
            let rh = if cur.h > 0. { w.frame.h / cur.h } else { 1. };
            Rect::new(
                v.x + rel_x * v.w,
                v.y + rel_y * v.h,
                (rw * v.w).round().max(1.),
                (rh * v.h).round().max(1.),
            )
        }
    };

    Ok(clamp_into(rect, v))
}

/// Clamp `r` fully inside visible frame `v` (FR-1.5). Sizes snap to integral
/// points; position snaps integral *relative to the visible-frame origin*
/// (screens may report fractional origins) and clamps into the frame.
fn clamp_into(r: Rect, v: &Rect) -> Rect {
    let w = r.w.min(v.w).round().max(1.);
    let h = r.h.min(v.h).round().max(1.);
    let rx = (r.x - v.x).round().clamp(0., v.w - w);
    let ry = (r.y - v.y).round().clamp(0., v.h - h);
    Rect::new(v.x + rx, v.y + ry, w, h)
}

#[cfg(test)]
mod tests {
    // Tests exercise panics directly; the production deny-set doesn't apply.
    // float_cmp: gap-free tests assert exact shared edges — the whole point
    // is that band math yields bit-identical coordinates, not approximates.
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::wildcard_enum_match_arm,
        clippy::many_single_char_names,
        clippy::map_unwrap_or,
        clippy::float_cmp
    )]
    use super::*;

    fn screen(index: usize, frame: Rect, visible: Rect, scale: f64) -> Screen {
        Screen {
            index,
            name: format!("Screen {index}"),
            frame,
            visible,
            scale,
        }
    }

    fn win(frame: Rect, screen: usize) -> WindowInfo {
        WindowInfo {
            pid: 42,
            title: "test".into(),
            app: "Test".into(),
            frame,
            minimized: false,
            screen,
        }
    }

    /// Built-in 14" MBP: main screen 1512x982 at (0,0), menu bar 25 pt top,
    /// no dock overlap in visible for simplicity.
    fn laptop() -> Vec<Screen> {
        vec![screen(
            0,
            Rect::new(0., 0., 1512., 982.),
            Rect::new(0., 25., 1512., 957.),
            2.,
        )]
    }

    fn laptop_plus_external() -> Vec<Screen> {
        vec![
            screen(
                0,
                Rect::new(0., 0., 1512., 982.),
                Rect::new(0., 25., 1512., 957.),
                2.,
            ),
            screen(
                1,
                Rect::new(1512., 0., 1920., 1080.),
                Rect::new(1512., 0., 1920., 1045.),
                1.,
            ),
        ]
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.51
    }

    fn assert_rect(r: Rect, x: f64, y: f64, w: f64, h: f64) {
        assert!(
            approx(r.x, x) && approx(r.y, y) && approx(r.w, w) && approx(r.h, h),
            "got {r:?}, want ({x},{y} {w}x{h})"
        );
    }

    #[test]
    fn left_right_halves_are_gap_free() {
        let s = laptop();
        let v = s[0].visible;
        let l = geometry(PresetId::Left, &win(v, 0), 0, &s).unwrap();
        let r = geometry(PresetId::Right, &win(v, 0), 0, &s).unwrap();
        assert_rect(l, 0., 25., 756., 957.);
        assert_rect(r, 756., 25., 756., 957.);
        assert_eq!(l.right(), r.x, "halves must share an edge exactly");
    }

    #[test]
    fn thirds_are_gap_free_with_odd_width() {
        // 1512/3 = 504 exactly; try an odd width too via 1513-like geometry.
        let mut s = laptop();
        s[0].visible.w = 1513.;
        let v = s[0].visible;
        let a = geometry(PresetId::LeftThird, &win(v, 0), 0, &s).unwrap();
        let b = geometry(PresetId::CenterThird, &win(v, 0), 0, &s).unwrap();
        let c = geometry(PresetId::RightThird, &win(v, 0), 0, &s).unwrap();
        assert_eq!(a.right(), b.x);
        assert_eq!(b.right(), c.x);
        assert_eq!(c.right(), v.right());
        assert_rect(a, 0., 25., 504., 957.);
    }

    #[test]
    fn maximize_uses_visible_frame_not_menu_bar() {
        let s = laptop();
        let r = geometry(PresetId::Maximize, &win(s[0].visible, 0), 0, &s).unwrap();
        assert_rect(r, 0., 25., 1512., 957.);
        assert!(r.y >= 25., "window must not sit under the menu bar");
    }

    #[test]
    fn center_keeps_size_and_clamps() {
        let s = laptop();
        // Window bigger than half the screen, off-center: keep size, center.
        let w = win(Rect::new(-500., -500., 1200., 800.), 0);
        let r = geometry(PresetId::Center, &w, 0, &s).unwrap();
        assert_rect(r, 156., 104., 1200., 800.);
        // Tiny window clamps into visible frame.
        let tiny = win(Rect::new(-50., -50., 20., 20.), 0);
        let r2 = geometry(PresetId::Center, &tiny, 0, &s).unwrap();
        assert_rect(r2, 746., 494., 20., 20.);
    }

    #[test]
    fn corners_quarter_the_visible_frame() {
        let s = laptop();
        let v = s[0].visible;
        let tl = geometry(PresetId::TopLeft, &win(v, 0), 0, &s).unwrap();
        let br = geometry(PresetId::BottomRight, &win(v, 0), 0, &s).unwrap();
        assert_rect(tl, 0., 25., 756., 479.);
        assert_rect(br, 756., 504., 756., 478.);
        assert_eq!(br.right(), v.right());
        assert_eq!(br.bottom(), v.bottom());
    }

    #[test]
    fn next_display_preserves_relative_position() {
        let s = laptop_plus_external();
        // Window at relative (0.5, 0.5) of main visible, size half visible.
        let v0 = s[0].visible;
        let w = win(
            Rect::new(v0.x + v0.w * 0.5, v0.y + v0.h * 0.5, v0.w * 0.5, v0.h * 0.5),
            0,
        );
        let r = geometry(PresetId::NextDisplay, &w, 0, &s).unwrap();
        let v1 = s[1].visible;
        assert!(
            r.x >= v1.x && r.y >= v1.y && r.right() <= v1.right() && r.bottom() <= v1.bottom(),
            "must land inside external visible frame, got {r:?}"
        );
        let rel_x = (r.x - v1.x) / v1.w;
        assert!(
            (rel_x - 0.5).abs() < 0.01,
            "relative x preserved, got {rel_x}"
        );
    }

    #[test]
    fn single_display_errors_for_display_presets_only() {
        let s = laptop();
        let w = win(s[0].visible, 0);
        assert_eq!(
            geometry(PresetId::NextDisplay, &w, 0, &s),
            Err(LayoutError::SingleDisplay)
        );
        assert_eq!(
            geometry(PresetId::PreviousDisplay, &w, 0, &s),
            Err(LayoutError::SingleDisplay)
        );
        // Normal presets still fine.
        assert!(geometry(PresetId::Left, &w, 0, &s).is_ok());
    }

    #[test]
    fn screen_of_picks_largest_overlap_and_center_fallback() {
        let s = laptop_plus_external();
        // Fully on external.
        let w1 = win(Rect::new(1600., 100., 400., 300.), 0);
        assert_eq!(screen_of(&w1, &s), 1);
        // Straddling but mostly on main.
        let w2 = win(Rect::new(1400., 100., 200., 300.), 0);
        assert_eq!(screen_of(&w2, &s), 0);
        // Degenerate zero-size window at center of external.
        let w3 = win(Rect::new(2400., 500., 0., 0.), 0);
        assert_eq!(screen_of(&w3, &s), 1);
    }

    #[test]
    fn clamped_results_are_integral() {
        let mut s = laptop();
        s[0].visible = Rect::new(0.3, 25.7, 1511.3, 956.3);
        let w = win(Rect::new(10.5, 30.2, 100.9, 50.1), 0);
        for id in [
            PresetId::Left,
            PresetId::CenterThird,
            PresetId::Maximize,
            PresetId::Center,
            PresetId::BottomRight,
        ] {
            let r = geometry(id, &w, 0, &s).unwrap();
            // Integral relative to the visible-frame origin (FR-1.5).
            let dx = r.x - s[0].visible.x;
            let dy = r.y - s[0].visible.y;
            assert_eq!(dx.round(), dx, "x integral for {id:?}: {r:?}");
            assert_eq!(dy.round(), dy, "y integral for {id:?}: {r:?}");
            assert_eq!(r.w.round(), r.w, "w integral for {id:?}: {r:?}");
        }
    }
}
