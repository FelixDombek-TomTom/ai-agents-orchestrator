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

/// Run a shell command in a new iTerm2 tab. The command is delivered to osascript
/// as an `on run argv` argument — never interpolated into the AppleScript body —
/// so there is no AppleScript/shell injection (mirrors the Electron ADR-005 shape).
/// Callers must shell-quote any values interpolated into `cmd`.
fn run_in_iterm_tab(cmd: &str) -> Result<(), String> {
    std::process::Command::new("osascript")
        .args([
            "-e", "on run argv",
            "-e", "set cmd to item 1 of argv",
            "-e", "tell application \"iTerm2\"",
            "-e", "activate",
            "-e", "if (count of windows) = 0 then create window with default profile",
            "-e", "set newTab to (create tab with default profile in current window)",
            "-e", "tell current session of newTab to write text cmd",
            "-e", "end tell",
            "-e", "end run",
        ])
        .arg(cmd)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Run a shell command in a new Terminal.app window. Same `on run argv` shape as
/// the iTerm adapter — the command is an argv arg, never interpolated into the
/// AppleScript body, so no injection. Terminal.app is always present on macOS,
/// so this is the bulletproof fallback.
fn run_in_terminal_app(cmd: &str) -> Result<(), String> {
    std::process::Command::new("osascript")
        .args([
            "-e", "on run argv",
            "-e", "tell application \"Terminal\"",
            "-e", "activate",
            "-e", "do script (item 1 of argv)",
            "-e", "end tell",
            "-e", "end run",
        ])
        .arg(cmd)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Launch `cmd` in the user's chosen terminal. `selector` is the lowercased
/// `config.terminalApp` value passed in by the caller. Blank or unknown → default:
/// iTerm if installed, else Terminal.app.
pub fn launch_in_terminal(cmd: &str, selector: &str) -> Result<(), String> {
    match selector {
        "iterm" => run_in_iterm_tab(cmd),
        "terminal" => run_in_terminal_app(cmd),
        _ if std::path::Path::new("/Applications/iTerm.app").exists() => run_in_iterm_tab(cmd),
        _ => run_in_terminal_app(cmd),
    }
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
