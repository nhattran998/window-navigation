//! Menu-bar status item (FR-6.1): NSStatusItem + NSMenu built from the same
//! wnav-core preset table the CLI and hotkey engine use. Item selection is
//! forwarded over an mpsc channel; the app drains it on the main thread and
//! dispatches the same actions as hotkeys/menus (single code path, SRS §2.1).
use std::sync::mpsc::{channel, Receiver, Sender};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSImage, NSMenu, NSMenuItem, NSStatusBar, NSVariableStatusItemLength};
use objc2_foundation::{ns_string, NSInteger, NSString};

use wnav_core::{PresetId, PRESETS};

/// Runtime `&str` → NSString (ns_string! only accepts literals).
fn nsstring(s: &str) -> Retained<NSString> {
    let mut buf: Vec<u16> = s.encode_utf16().collect();
    let ptr = std::ptr::NonNull::new(buf.as_mut_ptr()).expect("vec ptr non-null");
    unsafe { NSString::stringWithCharacters_length(ptr, buf.len()) }
}

/// What the user picked in the status menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusMenuEvent {
    /// Apply a preset to the frontmost window.
    Apply(PresetId),
    /// Open the Preferences window.
    OpenPreferences,
    /// Quit the app.
    Quit,
}

static EVENT_TX: std::sync::OnceLock<Sender<StatusMenuEvent>> = std::sync::OnceLock::new();

// Menu-item tags: presets use their index + TAG_PRESET; fixed entries have
// their own tags so the target's single action method can discriminate.
const TAG_PREFS: NSInteger = 9_001;
const TAG_QUIT: NSInteger = 9_002;
const TAG_PRESET: NSInteger = 10_000;

define_class! {
    // Trivial target: `menuAction:` inspects the sender's tag and forwards.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "WnavMenuTarget"]
    #[allow(missing_docs)]
    pub struct WnavMenuTarget;

    impl WnavMenuTarget {
        #[unsafe(method(menuAction:))]
        #[allow(clippy::missing_safety_doc)]
        unsafe fn menu_action(&self, _sender: Option<&AnyObject>) {
            let tag: NSInteger = match _sender {
                Some(sender) => unsafe { msg_send![sender, tag] },
                None => -1,
            };
            let ev = match tag {
                TAG_PREFS => Some(StatusMenuEvent::OpenPreferences),
                TAG_QUIT => Some(StatusMenuEvent::Quit),
                t if t >= TAG_PRESET => PRESETS
                    .get((t - TAG_PRESET).cast_unsigned())
                    .map(|p| StatusMenuEvent::Apply(p.id)),
                _ => None,
            };
            if let (Some(ev), Some(tx)) = (ev, EVENT_TX.get()) {
                let _ = tx.send(ev);
            }
        }
    }
}
impl WnavMenuTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm);
        unsafe { msg_send![this, init] }
    }
}

/// A live status item. Dropping it removes the item from the menu bar.
#[allow(missing_debug_implementations)]
pub struct StatusItem {
    _item: Retained<objc2_app_kit::NSStatusItem>,
    _target: Retained<WnavMenuTarget>,
    _receiver: Receiver<StatusMenuEvent>,
}

impl StatusItem {
    /// Create the menu-bar icon + menu. Must run on the main thread.
    ///
    /// # Panics
    /// Panics if a second status item is created (single-item app).
    #[must_use]
    pub fn new(mtm: MainThreadMarker) -> Self {
        let (tx, rx) = channel();
        let _ = EVENT_TX.set(tx);

        let target = WnavMenuTarget::new(mtm);
        let bar = NSStatusBar::systemStatusBar();
        let item = bar.statusItemWithLength(NSVariableStatusItemLength);
        if let Some(button) = item.button(mtm) {
            let icon = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                ns_string!("square.grid.2x2"),
                Some(ns_string!("Wnav window presets")),
            );
            if let Some(icon) = icon {
                // Template = adapts to light/dark menu bar.
                icon.setTemplate(true);
                button.setImage(Some(&icon));
            } else {
                button.setTitle(ns_string!("Wnav"));
            }
        }

        let menu = build_menu(&target, mtm);
        item.setMenu(Some(&menu));

        Self {
            _item: item,
            _target: target,
            _receiver: rx,
        }
    }

    /// Drain menu events (non-blocking); call from the main runloop.
    #[must_use]
    #[allow(clippy::used_underscore_binding)]
    pub fn poll(&self) -> Vec<StatusMenuEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self._receiver.try_recv() {
            out.push(ev);
        }
        out
    }
}

/// Menu mirroring Image 1 / `build_menus` (FR-6.1).
fn build_menu(target: &WnavMenuTarget, mtm: MainThreadMarker) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);
    let single_display = crate::screens().len() < 2;

    let add = |label: &str, tag: NSInteger, key: Option<&str>, enabled: bool| {
        let item = NSMenuItem::new(mtm);
        item.setTitle(&nsstring(label));
        item.setTag(tag);
        if let Some(key) = key {
            item.setKeyEquivalent(&nsstring(key));
        }
        // Target/action are unsafe: target must outlive the menu (it does —
        // StatusItem owns the Retained).
        unsafe { item.setTarget(Some(target)) };
        unsafe { item.setAction(Some(sel!(menuAction:))) };
        item.setEnabled(enabled);
        menu.addItem(&item);
    };

    // Halves
    add("Left", TAG_PRESET, Some("left"), true);
    add("Right", TAG_PRESET + 1, Some("right"), true);
    add("Top", TAG_PRESET + 2, Some("up"), true);
    add("Bottom", TAG_PRESET + 3, Some("down"), true);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    // Corners
    add("Top Left", TAG_PRESET + 4, Some("u"), true);
    add("Top Right", TAG_PRESET + 5, Some("i"), true);
    add("Bottom Left", TAG_PRESET + 6, Some("j"), true);
    add("Bottom Right", TAG_PRESET + 7, Some("k"), true);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    // Thirds
    add("Left Third", TAG_PRESET + 8, Some("d"), true);
    add("Left Two Thirds", TAG_PRESET + 9, Some("e"), true);
    add("Center Third", TAG_PRESET + 10, Some("f"), true);
    add("Right Two Thirds", TAG_PRESET + 11, Some("t"), true);
    add("Right Third", TAG_PRESET + 12, Some("g"), true);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    // Maximize / Center / Displays
    add("Maximize", TAG_PRESET + 13, Some("\r"), true);
    add("Center", TAG_PRESET + 14, Some("c"), true);
    add("Next Display", TAG_PRESET + 15, None, !single_display);
    add("Previous Display", TAG_PRESET + 16, None, !single_display);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    add("Preferences…", TAG_PREFS, Some(","), true);
    add("Quit Wnav", TAG_QUIT, Some("q"), true);
    menu
}
