//! Entry dispatcher: real menu-bar app on macOS, stub elsewhere (PRD OOS-4).
//! Modules are declared once; each file gates its own contents with
//! `#![cfg(target_os = "macos")]` so non-macOS builds compile an empty tree.

#[path = "actions.rs"]
pub(crate) mod actions;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod app;

#[path = "design.rs"]
pub(crate) mod design;
#[path = "keymap.rs"]
pub(crate) mod keymap;
#[path = "prefs.rs"]
pub(crate) mod prefs;
#[cfg(not(target_os = "macos"))]
#[path = "stub.rs"]
mod stub;

fn main() {
    #[cfg(target_os = "macos")]
    app::main();

    #[cfg(not(target_os = "macos"))]
    stub::main();
}
