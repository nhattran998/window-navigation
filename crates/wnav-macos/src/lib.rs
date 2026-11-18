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

pub mod ax;
pub mod hotkeys;
pub mod screens;
pub mod windows;

/// Debug exports (used by probe/tests).
pub use screens::screens;
pub use windows::{
    all_windows, app_name, apply_rect, frontmost_window, is_trusted, is_trusted_prompting,
    list_windows, on_screen_pids, resolve_window, windows_of_pid, AxWindow, TargetResolution,
};
