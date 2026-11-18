//! Global hotkeys via the `global-hotkey` crate (FR-3). Cross-platform API,
//! so wnav-app stays portable when wnav-windows/wnav-linux land (BG-4).

use std::collections::HashMap;

use global_hotkey::hotkey::{Code, HotKey};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use keyboard_types::Modifiers;

use wnav_core::shortcut::Key;
use wnav_core::{PresetId, Shortcut};

/// Registered hotkey engine. Keys are u32 ids from `HotKey`; values are presets.
/// Owns the global-hotkey manager; maps event ids to presets (FR-3).
pub struct HotkeyEngine {
    /// Held to keep the manager (and its registrations) alive.
    _manager: GlobalHotKeyManager,
    mapping: HashMap<u32, PresetId>,
    /// Presets whose registration failed (conflict) — surfaced in Preferences (FR-3.3).
    pub failed: Vec<PresetId>,
}

impl std::fmt::Debug for HotkeyEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotkeyEngine")
            .field("mapped", &self.mapping.len())
            .field("failed", &self.failed)
            .finish_non_exhaustive()
    }
}

impl HotkeyEngine {
    /// Register all shortcuts; failed ones recorded, others stay live (FR-3.3).
    /// Create the manager and register every shortcut; failures recorded (FR-3.3).
    pub fn new(shortcuts: &[(PresetId, Shortcut)]) -> Result<Self, global_hotkey::Error> {
        let manager = GlobalHotKeyManager::new()?;
        let mut mapping = HashMap::new();
        let mut failed = Vec::new();
        for (id, sc) in shortcuts {
            let hk = to_hotkey(*sc);
            match manager.register(hk) {
                Ok(()) => {
                    mapping.insert(hk.id(), *id);
                }
                Err(_) => failed.push(*id),
            }
        }
        Ok(Self {
            _manager: manager,
            mapping,
            failed,
        })
    }

    /// Preset for a received hotkey event.
    /// Preset bound to a fired hotkey event, if any.
    pub fn resolve(&self, ev: &GlobalHotKeyEvent) -> Option<PresetId> {
        self.mapping.get(&ev.id()).copied()
    }
}

fn to_hotkey(sc: Shortcut) -> HotKey {
    let mut mods = Modifiers::empty();
    if sc.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if sc.alt {
        mods |= Modifiers::ALT;
    }
    if sc.shift {
        mods |= Modifiers::SHIFT;
    }
    if sc.cmd {
        mods |= Modifiers::SUPER;
    }
    HotKey::new(Some(mods), code_of(sc.key))
}

fn code_of(k: Key) -> Code {
    // Exhaustiveness is enforced by the return type; glob keeps 60 arms short.
    #[allow(clippy::enum_glob_use)]
    use Code::*;
    match k {
        Key::A => KeyA,
        Key::B => KeyB,
        Key::C => KeyC,
        Key::D => KeyD,
        Key::E => KeyE,
        Key::F => KeyF,
        Key::G => KeyG,
        Key::H => KeyH,
        Key::I => KeyI,
        Key::J => KeyJ,
        Key::K => KeyK,
        Key::L => KeyL,
        Key::M => KeyM,
        Key::N => KeyN,
        Key::O => KeyO,
        Key::P => KeyP,
        Key::Q => KeyQ,
        Key::R => KeyR,
        Key::S => KeyS,
        Key::T => KeyT,
        Key::U => KeyU,
        Key::V => KeyV,
        Key::W => KeyW,
        Key::X => KeyX,
        Key::Y => KeyY,
        Key::Z => KeyZ,
        Key::Num0 => Digit0,
        Key::Num1 => Digit1,
        Key::Num2 => Digit2,
        Key::Num3 => Digit3,
        Key::Num4 => Digit4,
        Key::Num5 => Digit5,
        Key::Num6 => Digit6,
        Key::Num7 => Digit7,
        Key::Num8 => Digit8,
        Key::Num9 => Digit9,
        Key::F1 => F1,
        Key::F2 => F2,
        Key::F3 => F3,
        Key::F4 => F4,
        Key::F5 => F5,
        Key::F6 => F6,
        Key::F7 => F7,
        Key::F8 => F8,
        Key::F9 => F9,
        Key::F10 => F10,
        Key::F11 => F11,
        Key::F12 => F12,
        Key::F13 => F13,
        Key::F14 => F14,
        Key::F15 => F15,
        Key::F16 => F16,
        Key::F17 => F17,
        Key::F18 => F18,
        Key::F19 => F19,
        Key::Left => ArrowLeft,
        Key::Right => ArrowRight,
        Key::Up => ArrowUp,
        Key::Down => ArrowDown,
        Key::Enter => Enter,
        Key::Tab => Tab,
        Key::Space => Space,
        Key::Esc => Escape,
        Key::Minus => Minus,
        Key::Equal => Equal,
    }
}
