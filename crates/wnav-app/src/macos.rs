//! wnav-app — menu bar app (SRS §4.4, FR-6): native macOS menu (Image 1),
//! Preferences window (Image 2), global hotkeys (FR-3), AX apply.
//!
//! Threading note (bisected on macOS 26): any raw thread spawned before the
//! NSApp runloop starts breaks window visibility. All platform work therefore
//! goes through gpui executors: the hotkey forwarder is a background task
//! polling global-hotkey's crossbeam receiver, and every event dispatches the
//! ApplyPreset action on the main thread (AX requires it).
#![cfg(target_os = "macos")]

use gpui_kit::assets::Assets;
use objc2::MainThreadMarker;
use std::time::Duration;

use gpui_kit::gpui::{App, AsyncApp, KeyBinding, Menu, MenuItem};
use gpui_kit::{application, init as kit_init};

use crate::actions::ApplyPreset;

/// App-wide state (hotkey engine lifetime).
struct WnavState {
    hotkeys: Option<wnav_macos::hotkeys::HotkeyEngine>,
    status_item: Option<wnav_macos::StatusItem>,
}
impl gpui_kit::gpui::Global for WnavState {}

/// Hotkey events: background poller → main-thread forwarder.
static MAIN_TX: LazyLock<std::sync::mpsc::Sender<global_hotkey::GlobalHotKeyEvent>> =
    LazyLock::new(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        MAIN_RX.get_or_init(|| std::sync::Mutex::new(rx));
        tx
    });

static MAIN_RX: std::sync::OnceLock<
    std::sync::Mutex<std::sync::mpsc::Receiver<global_hotkey::GlobalHotKeyEvent>>,
> = std::sync::OnceLock::new();

use std::sync::LazyLock;

pub(crate) fn main() {
    application().with_assets(Assets).run(|cx: &mut App| {
        kit_init(cx);

        // Permission gate first-run (AC-F1-11): prompt once at launch.
        if !wnav_macos::is_trusted() {
            wnav_macos::is_trusted_prompting();
        }

        // Global action handlers (menu items + keybindings dispatch these).
        cx.on_action(|_: &crate::actions::OpenPreferences, cx| crate::prefs::open(cx));
        cx.on_action(|_: &crate::actions::QuitWnav, cx| cx.quit());
        cx.on_action(|_: &crate::actions::ToggleLaunchAtLogin, cx| toggle_launch_at_login(cx));
        cx.on_action(|a: &ApplyPreset, _cx| apply_preset(a.0));

        // Keybindings = effective shortcuts (menu accelerators, FR-3.2).
        let cfg = load_config();
        let mut bindings = Vec::new();
        if let Ok(shortcuts) = cfg.effective_shortcuts() {
            for (id, sc) in &shortcuts {
                if let Ok(b) = KeyBinding::load(
                    &gpui_keystroke(*sc),
                    Box::new(ApplyPreset(*id)),
                    None,
                    false,
                    None,
                    cx,
                ) {
                    bindings.push(b);
                }
            }
        }
        bindings.push(KeyBinding::new(
            "cmd-,",
            crate::actions::OpenPreferences,
            None,
        ));
        cx.bind_keys(bindings);

        // Application menu = Image 1 (FR-6.1).
        cx.set_menus(build_menus());

        // Global hotkeys (FR-3): engine lives in the global state.
        let hotkeys = match cfg.effective_shortcuts() {
            Ok(shortcuts) => match wnav_macos::hotkeys::HotkeyEngine::new(&shortcuts) {
                Ok(h) => Some(h),
                Err(e) => {
                    eprintln!("wnav: hotkey engine failed: {e}");
                    None
                }
            },
            Err(e) => {
                eprintln!("wnav: config invalid, hotkeys disabled: {e}");
                None
            }
        };
        // Menu-bar icon + menu (FR-6.1): an LSUIElement app is unreachable
        // without it — `set_menus` only fills the never-shown app menu bar.
        let status_item = MainThreadMarker::new().map(wnav_macos::StatusItem::new);

        cx.set_global(WnavState {
            hotkeys,
            status_item,
        });

        // Background poller: global-hotkey's crossbeam receiver → MAIN_TX.
        cx.background_executor()
            .spawn(async move {
                let rx = global_hotkey::GlobalHotKeyEvent::receiver();
                loop {
                    match rx.try_recv() {
                        Ok(ev) => {
                            let _ = MAIN_TX.send(ev);
                        }
                        Err(crossbeam_channel::TryRecvError::Empty) => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(crossbeam_channel::TryRecvError::Disconnected) => break,
                    }
                }
            })
            .detach();

        // Main-thread forwarder: non-blocking poll each frame via spawn; the
        // blocking recv() variant starves the runloop (bisected).
        cx.spawn(async move |cx: &mut AsyncApp| {
            LazyLock::force(&MAIN_TX); // ensure MAIN_RX exists before reading it
            let Some(rx) = MAIN_RX.get() else { return };
            loop {
                // Lock, drain one event, drop the guard BEFORE awaiting.
                // A poisoned lock (panic in a holder) skips this tick; the
                // timer retries rather than crash the app.
                let ev = {
                    let ev = rx.lock().ok().and_then(|rx| rx.try_recv().ok());
                    if ev.is_none() {
                        cx.background_executor()
                            .timer(Duration::from_millis(20))
                            .await;
                    }
                    ev
                };
                // Status-menu picks: drain every tick (independent of hotkeys).
                cx.update(|cx: &mut App| {
                    let Some(item) = cx
                        .try_global::<WnavState>()
                        .and_then(|s| s.status_item.as_ref())
                    else {
                        return;
                    };
                    for ev in item.poll() {
                        match ev {
                            wnav_macos::StatusMenuEvent::Apply(id) => {
                                cx.dispatch_action(&ApplyPreset(id));
                            }
                            wnav_macos::StatusMenuEvent::OpenPreferences => {
                                cx.dispatch_action(&crate::actions::OpenPreferences);
                            }
                            wnav_macos::StatusMenuEvent::Quit => cx.quit(),
                        }
                    }
                });

                let Some(ev) = ev else { continue };
                cx.update(|cx: &mut App| {
                    let Some(id) = cx
                        .try_global::<WnavState>()
                        .and_then(|s| s.hotkeys.as_ref())
                        .and_then(|h| h.resolve(&ev))
                    else {
                        return;
                    };
                    cx.dispatch_action(&ApplyPreset(id));
                });
            }
        })
        .detach();

        // The app is LSUIElement (bundle target, M4): no Dock icon, no main
        // window. Preferences opens from the menu (cmd-,).
    });
}

fn load_config() -> wnav_core::Config {
    wnav_core::Config::load(&wnav_core::config::default_config_path()).unwrap_or_default()
}

/// Menu structure identical to Image 1 groups (FR-6.1).
fn build_menus() -> Vec<Menu> {
    let single_display = wnav_macos::screens().len() < 2;
    let preset_items: Vec<MenuItem> = wnav_core::PRESETS
        .iter()
        .map(|p| {
            let item = MenuItem::action(p.label, ApplyPreset(p.id));
            if p.needs_two_screens && single_display {
                item.disabled(true)
            } else {
                item
            }
        })
        .collect();

    vec![
        Menu::new("Wnav").items([
            MenuItem::action("About Wnav", crate::actions::OpenPreferences),
            MenuItem::separator(),
            MenuItem::action("Preferences…", crate::actions::OpenPreferences),
            MenuItem::action("Launch at Login", crate::actions::ToggleLaunchAtLogin),
            MenuItem::separator(),
            MenuItem::action("Quit Wnav", crate::actions::QuitWnav),
        ]),
        Menu::new("Window").items(preset_items),
    ]
}

/// Toggle Launch-at-login from the app menu; persists to config (FR-5.4).
fn toggle_launch_at_login(_cx: &mut App) {
    let path = wnav_core::config::default_config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let raw = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = raw
        .parse::<toml_edit::DocumentMut>()
        .expect("existing config parses");
    let current = doc
        .get("general")
        .and_then(|g| g.get("launch-at-login"))
        .and_then(toml_edit::Item::as_bool)
        .unwrap_or(true);
    let general = doc
        .entry("general")
        .or_insert(toml_edit::table())
        .as_table_mut()
        .expect("general is a table");
    general.insert("launch-at-login", toml_edit::value(!current));
    let _ = std::fs::write(&path, doc.to_string());
}

/// Apply a preset to the frontmost window (AX on main thread, FR-2.1).
fn apply_preset(id: wnav_core::PresetId) {
    let screens = wnav_macos::screens();
    let Some(win) = wnav_macos::frontmost_window() else {
        return;
    };
    let info = win.to_info(&screens);
    let idx = wnav_core::screen_of(&info, &screens);
    if let Ok(target) = wnav_core::geometry(id, &info, idx, &screens) {
        let _ = wnav_macos::apply_rect(&win, target);
    }
}

/// Convert a wnav-core shortcut to gpui keystroke syntax ("ctrl-alt-cmd-right").
fn gpui_keystroke(sc: wnav_core::Shortcut) -> String {
    let mut s = String::new();
    if sc.ctrl {
        s.push_str("ctrl-");
    }
    if sc.alt {
        s.push_str("alt-");
    }
    if sc.shift {
        s.push_str("shift-");
    }
    if sc.cmd {
        s.push_str("cmd-");
    }
    s.push_str(sc.key.canonical());
    s
}
