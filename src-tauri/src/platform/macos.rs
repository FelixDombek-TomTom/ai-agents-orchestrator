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

/// The controlling tty of a pid (e.g. "/dev/ttys003"), via `ps`. None when the
/// process has no terminal tty — e.g. it runs in our embedded pty rather than a
/// real terminal window (so there's nothing to reveal).
pub fn session_tty(pid: i64) -> Option<String> {
    if pid <= 0 {
        return None;
    }
    let out = std::process::Command::new("ps")
        .args(["-o", "tty=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // ps prints "ttys003", or "??" / "" when there is no controlling terminal.
    if !t.starts_with("ttys") {
        return None;
    }
    Some(format!("/dev/{t}"))
}
