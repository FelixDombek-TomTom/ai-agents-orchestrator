//! Linux platform syscalls (GNOME/Wayland verified; other desktops best-effort).

use std::path::Path;

/// Open an http(s) URL in the default browser. URL already scheme-checked by caller.
pub fn open_uri(uri: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg("--")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Reveal an already-canonicalized path in the file manager.
pub fn open_path(path: &Path) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg("--")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Map a `ps -o tty=` value to a device path. Linux prints `pts/N` for a pty;
/// `?` / empty / a console `ttyN` mean no controlling pty we can act on.
fn parse_tty(raw: &str) -> Option<String> {
    let t = raw.trim();
    if let Some(n) = t.strip_prefix("pts/") {
        if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) {
            return Some(format!("/dev/pts/{n}"));
        }
    }
    None
}

/// Controlling tty of a pid, or None when it has no pty (e.g. it runs in our
/// embedded pty rather than an external terminal window).
pub fn session_tty(pid: i64) -> Option<String> {
    if pid <= 0 {
        return None;
    }
    let out = std::process::Command::new("ps")
        .args(["-o", "tty=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    parse_tty(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::parse_tty;

    #[test]
    fn parse_tty_maps_pts_and_rejects_none() {
        assert_eq!(parse_tty("pts/3"), Some("/dev/pts/3".to_string()));
        assert_eq!(parse_tty("pts/0\n"), Some("/dev/pts/0".to_string()));
        assert_eq!(parse_tty("??"), None);
        assert_eq!(parse_tty(""), None);
        assert_eq!(parse_tty("tty1"), None); // a real console, not our pty
    }
}
