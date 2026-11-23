//! wnav-macos: AX/CG/NSScreen backend implementing wnav-core's porting traits
//! (SRS §4.2). Compiled only on macOS.

// Raw-pointer domain: CF/AX APIs traffic in `*const c_void` and toll-free
// bridges between CF and NS types.
#![allow(
    clippy::ptr_as_ptr,
    clippy::as_ptr_cast_mut,
    clippy::borrow_as_ptr,
    clippy::ref_as_ptr,
    clippy::ptr_arg
)]

#[cfg(target_os = "macos")]
pub mod ax;
#[cfg(target_os = "macos")]
pub mod hotkeys;
#[cfg(target_os = "macos")]
pub mod screens;
#[cfg(target_os = "macos")]
pub mod status_item;
#[cfg(target_os = "macos")]
pub mod windows;
#[cfg(target_os = "macos")]
pub use status_item::{StatusItem, StatusMenuEvent};

#[cfg(target_os = "macos")]
pub use screens::screens;
#[cfg(target_os = "macos")]
pub use windows::{
    all_windows, app_name, apply_rect, frontmost_window, is_trusted, is_trusted_prompting,
    list_windows, on_screen_pids, resolve_window, windows_of_pid, AxWindow, TargetResolution,
};
