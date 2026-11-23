//! Window enumeration + move/resize via AX (SRS §4.2 WindowController, FR-2).

use std::ptr;

use wnav_core::{Rect, WindowInfo};

use crate::ax::{
    cfstring, from_cfstring, AXIsProcessTrusted, AXIsProcessTrustedWithOptions,
    AXUIElementCopyAttributeValue, AXUIElementCreateApplication, AXUIElementPerformAction,
    AXUIElementRef, AXUIElementSetAttributeValue, AXValueCreate, AXValueGetValue, AXValueRef,
    CFArrayRef, CFDictionaryRef, CFRelease, CFStringRef, CFTypeRef, CGPoint, CGSize,
    K_AX_ERROR_SUCCESS, K_AX_VALUE_CGPOINT_TYPE, K_AX_VALUE_CGSIZE_TYPE,
};

/// kAXWindowRole / attribute names.
const K_AX_WINDOWS_ATTRIBUTE: &str = "AXWindows";
const K_AX_FOCUSED_WINDOW_ATTRIBUTE: &str = "AXFocusedWindow";
const K_AX_POSITION_ATTRIBUTE: &str = "AXPosition";
const K_AX_SIZE_ATTRIBUTE: &str = "AXSize";
const K_AX_TITLE_ATTRIBUTE: &str = "AXTitle";
const K_AX_MINIMIZED_ATTRIBUTE: &str = "AXMinimized";
const K_AX_RAISE_ACTION: &str = "AXRaise";

/// Accessibility permission without prompting (CLI probe, FR-2/exit 4).
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Accessibility permission with macOS prompt (app first-run, AC-F1-11).
pub fn is_trusted_prompting() -> bool {
    unsafe {
        let key = cfstring("AXTrustedCheckOptionPrompt");
        let value = kCFBooleanTrue;
        let options = CFDictionaryCreate(
            ptr::null(),
            &key as *const CFStringRef as *const _,
            &value as *const CFTypeRef as *const _,
            1,
            &kCFTypeDictionaryKeyCallBacks as *const _,
            &kCFTypeDictionaryValueCallBacks as *const _,
        );
        let trusted = AXIsProcessTrustedWithOptions(options) != 0;
        CFRelease(options);
        CFRelease(key as *const _);
        trusted
    }
}

// CFDictionary constructors used by the permission probe.
extern "C" {
    /// Create a CFDictionary from parallel key/value arrays.
    pub fn CFDictionaryCreate(
        allocator: *const std::os::raw::c_void,
        keys: *const *const std::os::raw::c_void,
        values: *const *const std::os::raw::c_void,
        num_values: isize,
        key_callbacks: *const std::os::raw::c_void,
        value_callbacks: *const std::os::raw::c_void,
    ) -> CFDictionaryRef;
    /// Shared CFType callbacks — REQUIRED for CFDictionary (NULL makes HIServices crash).
    #[allow(missing_docs)]
    pub static kCFTypeDictionaryKeyCallBacks: std::os::raw::c_void;
    #[allow(missing_docs)]
    pub static kCFTypeDictionaryValueCallBacks: std::os::raw::c_void;
}

/// AXValue for AXPosition/AXSize set/get.
fn axvalue_point(x: f64, y: f64) -> AXValueRef {
    let p = CGPoint { x, y };
    unsafe { AXValueCreate(K_AX_VALUE_CGPOINT_TYPE, &p as *const CGPoint as *const _) }
}

fn axvalue_size(w: f64, h: f64) -> AXValueRef {
    let s = CGSize {
        width: w,
        height: h,
    };
    unsafe { AXValueCreate(K_AX_VALUE_CGSIZE_TYPE, &s as *const CGSize as *const _) }
}

fn get_position(el: AXUIElementRef) -> Option<(f64, f64)> {
    unsafe {
        let mut out: CFTypeRef = ptr::null();
        let name = cfstring(K_AX_POSITION_ATTRIBUTE);
        let err = AXUIElementCopyAttributeValue(el, name, &mut out);
        CFRelease(name);
        if err != K_AX_ERROR_SUCCESS || out.is_null() {
            return None;
        }
        let mut p = CGPoint { x: 0., y: 0. };
        let ok = AXValueGetValue(
            out as AXValueRef,
            K_AX_VALUE_CGPOINT_TYPE,
            &mut p as *mut _ as *mut _,
        );
        CFRelease(out);
        if ok != 0 {
            // AX reports -0.0 for x=0 on some windows; normalize (cosmetic).
            Some((
                if p.x == 0. { 0. } else { p.x },
                if p.y == 0. { 0. } else { p.y },
            ))
        } else {
            None
        }
    }
}

fn get_size(el: AXUIElementRef) -> Option<(f64, f64)> {
    unsafe {
        let mut out: CFTypeRef = ptr::null();
        let name = cfstring(K_AX_SIZE_ATTRIBUTE);
        let err = AXUIElementCopyAttributeValue(el, name, &mut out);
        CFRelease(name);
        if err != K_AX_ERROR_SUCCESS || out.is_null() {
            return None;
        }
        let mut s = CGSize {
            width: 0.,
            height: 0.,
        };
        let ok = AXValueGetValue(
            out as AXValueRef,
            K_AX_VALUE_CGSIZE_TYPE,
            &mut s as *mut _ as *mut _,
        );
        CFRelease(out);
        if ok != 0 {
            Some((s.width, s.height))
        } else {
            None
        }
    }
}

/// Set position+size in one shot; returns the raw AX error (0 = ok).
///
/// # Safety
///
/// `el` must be a valid AXUIElementRef.
pub unsafe fn set_frame(
    el: AXUIElementRef,
    pos_x: f64,
    pos_y: f64,
    width: f64,
    height: f64,
) -> i32 {
    let pos_x = if pos_x == 0. { 0. } else { pos_x }; // normalize -0.0
    let pos_y = if pos_y == 0. { 0. } else { pos_y };
    unsafe {
        let pname = cfstring(K_AX_POSITION_ATTRIBUTE);
        let p = axvalue_point(pos_x, pos_y);
        let err1 = AXUIElementSetAttributeValue(el, pname, p);
        CFRelease(pname);
        CFRelease(p);
        if err1 != K_AX_ERROR_SUCCESS {
            return err1;
        }
        let sname = cfstring(K_AX_SIZE_ATTRIBUTE);
        let s = axvalue_size(width, height);
        let err2 = AXUIElementSetAttributeValue(el, sname, s);
        CFRelease(sname);
        CFRelease(s);
        err2
    }
}

/// One AX window element with its owning pid.
pub struct AxWindow {
    /// Retained AXUIElementRef; released in Drop.
    pub element: AXUIElementRef,
    /// Owning process id.
    pub pid: i32,
}

impl std::fmt::Debug for AxWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxWindow")
            .field("pid", &self.pid)
            .finish_non_exhaustive()
    }
}

impl Drop for AxWindow {
    fn drop(&mut self) {
        if !self.element.is_null() {
            unsafe { CFRelease(self.element) };
        }
    }
}

impl AxWindow {
    /// AX title of the window (may be empty for some apps).
    pub fn title(&self) -> String {
        unsafe {
            let mut out: CFTypeRef = ptr::null();
            let name = cfstring(K_AX_TITLE_ATTRIBUTE);
            let err = AXUIElementCopyAttributeValue(self.element, name, &mut out);
            CFRelease(name);
            if err == K_AX_ERROR_SUCCESS && !out.is_null() {
                let t = from_cfstring(out as CFStringRef).unwrap_or_default();
                CFRelease(out);
                t
            } else {
                String::new()
            }
        }
    }

    fn is_minimized(&self) -> bool {
        unsafe {
            let mut out: CFTypeRef = ptr::null();
            let name = cfstring(K_AX_MINIMIZED_ATTRIBUTE);
            let err = AXUIElementCopyAttributeValue(self.element, name, &mut out);
            CFRelease(name);
            if err == K_AX_ERROR_SUCCESS && !out.is_null() {
                let minimized = CFBooleanGetValue(out as *const std::os::raw::c_void) != 0;
                CFRelease(out);
                minimized
            } else {
                false
            }
        }
    }

    fn frame(&self) -> Option<Rect> {
        let (x, y) = get_position(self.element)?;
        let (w, h) = get_size(self.element)?;
        Some(Rect::new(x, y, w, h))
    }

    /// Snapshot into the platform-agnostic [`wnav_core::WindowInfo`].
    pub fn to_info(&self, all_screens: &[wnav_core::Screen]) -> WindowInfo {
        let frame = self.frame().unwrap_or(Rect::new(0., 0., 0., 0.));
        let pid = self.pid;
        WindowInfo {
            pid,
            title: self.title(),
            app: app_name(pid),
            frame,
            minimized: self.is_minimized(),
            screen: wnav_core::screen_of(
                &WindowInfo {
                    pid,
                    title: String::new(),
                    app: String::new(),
                    frame,
                    minimized: false,
                    screen: 0,
                },
                all_screens,
            ),
        }
    }

    /// Un-minimize and focus (FR-2.4).
    pub fn raise(&self) {
        unsafe {
            let name = cfstring(K_AX_RAISE_ACTION);
            AXUIElementPerformAction(self.element, name);
            CFRelease(name);
            if self.is_minimized() {
                let name = cfstring(K_AX_MINIMIZED_ATTRIBUTE);
                let f = kCFBooleanFalse;
                AXUIElementSetAttributeValue(self.element, name, f as CFTypeRef);
                CFRelease(name);
            }
        }
    }

    /// Move/resize to `r`; raw AX error code (0 = ok).
    pub fn apply(&self, r: Rect) -> i32 {
        // SAFETY: self.element is a valid AXUIElementRef (retained on creation,
        // released in Drop) and set_frame only reads it.
        unsafe { set_frame(self.element, r.x, r.y, r.w, r.h) }
    }
}

// CFBoolean accessors used to un-minimize windows (doc comments are unused
// on extern items; plain comments here).
extern "C" {
    /// Unwrap a CFBoolean into 1/0.
    pub fn CFBooleanGetValue(b: *const std::os::raw::c_void) -> u8;
    // Shared CFBoolean constants (docs on extern statics are unused-lint).
    #[allow(missing_docs)]
    pub static kCFBooleanFalse: CFTypeRef;
    #[allow(missing_docs)]
    pub static kCFBooleanTrue: CFTypeRef;
}

/// All windows of one app (FR-2.2).
pub fn windows_of_pid(pid: i32) -> Vec<AxWindow> {
    unsafe {
        let app = AXUIElementCreateApplication(pid);
        let mut out: CFTypeRef = ptr::null();
        let name = cfstring(K_AX_WINDOWS_ATTRIBUTE);
        let err = AXUIElementCopyAttributeValue(app, name, &mut out);
        CFRelease(name);
        CFRelease(app);
        if err != K_AX_ERROR_SUCCESS || out.is_null() {
            return Vec::new();
        }
        // CFArray of AXUIElementRef
        let arr = out as CFArrayRef;
        let n = CFArrayGetCount(arr);
        let mut windows = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
        for i in 0..n {
            let el = CFArrayGetValueAtIndex(arr, i) as AXUIElementRef;
            if !el.is_null() {
                CFRetain(el as *const std::os::raw::c_void);
                windows.push(AxWindow { element: el, pid });
            }
        }
        CFRelease(out);
        windows
    }
}

// Core Foundation helpers used by the enumeration paths.
extern "C" {
    /// Increment the retain count.
    pub fn CFRetain(cf: *const std::os::raw::c_void);
    /// Number of elements in a CFArray.
    pub fn CFArrayGetCount(array: CFArrayRef) -> isize;
    /// Element at an index (0-based, no retain).
    pub fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: isize) -> *const std::os::raw::c_void;
}

/// All windows of all apps with on-screen presence (FR-2.3). Source of pids:
/// CGWindowList (window-server truth — includes apps NSWorkspace misses).
pub fn all_windows() -> Vec<AxWindow> {
    let mut out: Vec<AxWindow> = Vec::new();
    for pid in cgwindow_on_screen_pids_dedup() {
        out.extend(windows_of_pid(pid));
    }
    out
}

/// On-screen pids deduped, window-server ordered (used by all_windows).
fn cgwindow_on_screen_pids_dedup() -> Vec<i32> {
    let mut seen = std::collections::HashSet::new();
    let mut pids = Vec::new();
    for pid in cgwindow_on_screen_pids() {
        if seen.insert(pid) {
            pids.push(pid);
        }
    }
    pids
}

fn cgwindow_on_screen_pids() -> Vec<i32> {
    use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};
    unsafe {
        let list = CGWindowListCopyWindowInfo(
            K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
            0,
        );
        if list.is_null() {
            return Vec::new();
        }
        // Consume CGWindowListCopyWindowInfo's +1 into an NSArray handle.
        let arr =
            objc2::rc::Retained::from_raw(list as *mut NSArray<NSDictionary<NSString, NSNumber>>)
                .expect("CFArray is an NSArray");
        let key_pid = NSString::from_str("kCGWindowOwnerPID");
        let mut pids = Vec::new();
        for dict in &arr {
            if let Some(num) = dict.objectForKey(&key_pid) {
                pids.push(num.as_i32());
            }
        }
        pids
    }
}

// Window-server metadata list; returns a +1 retained CFArray.
extern "C" {
    /// Snapshot of on-screen windows (option mask, all windows).
    pub fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFArrayRef;
}

const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1;
const K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: u32 = 4;

/// Pids of regular on-screen apps (NSWorkspace runningApplications, filter:
/// has windows, not itself).
pub fn on_screen_pids() -> Vec<i32> {
    let ws = objc2_app_kit::NSWorkspace::sharedWorkspace();
    ws.runningApplications()
        .iter()
        .filter(|a| {
            a.activationPolicy() == objc2_app_kit::NSApplicationActivationPolicy::Regular
                && !a.isTerminated()
                && !a.isHidden()
        })
        .map(|a| a.processIdentifier())
        .collect()
}

/// Frontmost app's focused window, else its first window (FR-2.1).
pub fn frontmost_window() -> Option<AxWindow> {
    unsafe {
        let ws = objc2_app_kit::NSWorkspace::sharedWorkspace();
        let app = ws.frontmostApplication()?;
        let pid = app.processIdentifier();
        let app_el = AXUIElementCreateApplication(pid);
        let mut out: CFTypeRef = ptr::null();

        let name = cfstring(K_AX_FOCUSED_WINDOW_ATTRIBUTE);
        let err = AXUIElementCopyAttributeValue(app_el, name, &mut out);
        CFRelease(name);
        if err == K_AX_ERROR_SUCCESS && !out.is_null() {
            CFRelease(app_el);
            return Some(AxWindow {
                element: out as AXUIElementRef,
                pid,
            });
        }

        let name = cfstring(K_AX_WINDOWS_ATTRIBUTE);
        let err = AXUIElementCopyAttributeValue(app_el, name, &mut out);
        CFRelease(name);
        CFRelease(app_el);
        if err == K_AX_ERROR_SUCCESS && !out.is_null() {
            let arr = out as CFArrayRef;
            if CFArrayGetCount(arr) > 0 {
                let el = CFArrayGetValueAtIndex(arr, 0) as AXUIElementRef;
                CFRetain(el as *const std::os::raw::c_void);
                CFRelease(out);
                return Some(AxWindow { element: el, pid });
            }
            CFRelease(out);
        }
        None
    }
}

/// App display name for pid (FR output "app" field).
pub fn app_name(pid: i32) -> String {
    let ws = objc2_app_kit::NSWorkspace::sharedWorkspace();
    ws.runningApplications()
        .iter()
        .find(|a| a.processIdentifier() == pid)
        .and_then(|a| a.localizedName().map(|n| n.to_string()))
        .unwrap_or_default()
}

/// Window list snapshot for CLI (SRS 2.4 list-windows).
pub fn list_windows() -> Vec<WindowInfo> {
    let all = crate::screens();
    let mut out = Vec::new();
    for w in all_windows() {
        let info = w.to_info(&all);
        if info.frame.w > 1. && info.frame.h > 1. {
            out.push(info);
        }
    }
    out
}

/// Outcome of resolving a window target.
#[derive(Debug)]
pub enum TargetResolution {
    /// Exactly one window matched.
    Found(AxWindow),
    /// Nothing matched.
    None,
    /// Multiple title matches; carries the titles (exit 7).
    Ambiguous(Vec<String>),
}

/// Resolve the target window by pid or title substring (FR-2.2/2.3); falls
/// back to the frontmost window when both filters are `None` (FR-2.1).
pub fn resolve_window(pid: Option<i32>, title: Option<&str>) -> TargetResolution {
    let candidates: Vec<AxWindow> = match (pid, title) {
        (Some(p), _) => windows_of_pid(p),
        (None, Some(t)) => {
            let needle = t.to_lowercase();
            all_windows()
                .into_iter()
                .filter(|w| w.title().to_lowercase().contains(&needle))
                .collect()
        }
        (None, None) => {
            return match frontmost_window() {
                Some(w) => TargetResolution::Found(w),
                None => TargetResolution::None,
            }
        }
    };
    if candidates.is_empty() {
        return TargetResolution::None;
    }
    if title.is_some() && candidates.len() > 1 {
        return TargetResolution::Ambiguous(candidates.iter().map(AxWindow::title).collect());
    }
    TargetResolution::Found(
        // Non-empty is checked above; first is the AX-focused candidate.
        candidates
            .into_iter()
            .next()
            .expect("candidates checked non-empty"),
    )
}

/// Apply a computed wnav-core rect to a resolved window; raises first (FR-2.4).
pub fn apply_rect(w: &AxWindow, r: Rect) -> i32 {
    w.raise();
    // Normalize -0.0 (AX accepts it, but reports and JSON should be clean).
    w.apply(Rect::new(r.x + 0., r.y + 0., r.w + 0., r.h + 0.))
}
