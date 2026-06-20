//! OS-specific syscalls (browser/file-manager open, external terminal launch,
//! terminal-window discovery). The only place `#[cfg(target_os)]` lives —
//! `lib.rs` keeps all validation and calls these to execute.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
