#![cfg(not(target_os = "macos"))]

//! wnav-app is macOS-only in v1 (PRD OOS-4). This stub keeps
//! `cargo build/test --workspace` green on other platforms; the real app
//! lives in macos.rs and is cfg-selected by main.rs.
pub(crate) fn main() {
    eprintln!("wnav-app: not supported on this platform yet (v1 is macOS-only)");
    std::process::exit(2);
}
