//! GPUI actions for wnav-app menu dispatch (namespace `wnav`).
#![cfg(target_os = "macos")]

// gpui-kit's macros expand to `::gpui::...` paths; the kit docs require the
// app to alias its crate so those paths resolve (kit/src/lib.rs:94-117).
extern crate gpui_kit as gpui;

use gpui_kit::gpui::{actions, Action};
use wnav_core::PresetId;

actions!(wnav, [OpenPreferences, QuitWnav, ToggleLaunchAtLogin]);

/// Apply a window preset; carries the preset id (menu + hotkey + keybinding).
// Deserialize/JsonSchema are required by the Action derive machinery; the
// struct carries no unsafe data, so the unsafe_derive_deserialize heuristic
// (which looks at the surrounding crate) is a false positive here.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, PartialEq, Debug, serde::Deserialize, schemars::JsonSchema, Action)]
#[action(namespace = wnav)]
pub(crate) struct ApplyPreset(pub PresetId);
