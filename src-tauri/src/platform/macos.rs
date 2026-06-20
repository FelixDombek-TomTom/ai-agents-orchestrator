//! macOS platform syscalls.

use std::path::Path;

/// Open an http(s) URL in the default browser. URL already scheme-checked by caller.
pub fn open_uri(uri: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("--")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Reveal an already-canonicalized path in Finder.
pub fn open_path(path: &Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("--")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
