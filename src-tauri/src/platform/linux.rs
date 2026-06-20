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

/// Terminals we know how to drive, in auto-detect preference order.
const KNOWN_TERMINALS: &[&str] = &[
    "gnome-terminal", "konsole", "xfce4-terminal", "kitty", "alacritty", "foot", "xterm",
];

/// Build the argv to run `cmd` in `bin`, keeping the window open after the
/// command exits (`; exec bash`) to match macOS `do script`. Each terminal has
/// its own "run this program" flag.
fn terminal_argv(bin: &str, cmd: &str) -> Vec<String> {
    let script = format!("{cmd}; exec bash");
    let head: Vec<&str> = match bin {
        "gnome-terminal" => vec![bin, "--"],
        "xfce4-terminal" => vec![bin, "-x"],
        "kitty" | "foot" => vec![bin],
        // konsole, xterm, alacritty, x-terminal-emulator, and any other -e style
        _ => vec![bin, "-e"],
    };
    let mut v: Vec<String> = head.into_iter().map(String::from).collect();
    v.extend(["bash".to_string(), "-lc".to_string(), script]);
    v
}

/// Pick a terminal: explicit selector (matched against KNOWN_TERMINALS) → $TERMINAL
/// → x-terminal-emulator → first installed of KNOWN_TERMINALS. None if nothing found.
fn resolve_terminal(selector: &str) -> Option<String> {
    fn on_path(bin: &str) -> bool {
        std::process::Command::new("sh")
            .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    let sel = selector.trim();
    if !sel.is_empty() && KNOWN_TERMINALS.contains(&sel) && on_path(sel) {
        return Some(sel.to_string());
    }
    if let Ok(t) = std::env::var("TERMINAL") {
        if !t.is_empty() && on_path(&t) {
            return Some(t);
        }
    }
    if on_path("x-terminal-emulator") {
        return Some("x-terminal-emulator".to_string());
    }
    KNOWN_TERMINALS.iter().find(|b| on_path(b)).map(|b| b.to_string())
}

/// Run `cmd` in an external terminal. `cmd` is already validated + shell-quoted by
/// the caller and is passed as a single `bash -lc` argument (no re-interpolation).
pub fn launch_in_terminal(cmd: &str, selector: &str) -> Result<(), String> {
    let bin = resolve_terminal(selector).ok_or_else(|| {
        format!(
            "no supported terminal found (tried selector, $TERMINAL, x-terminal-emulator, {})",
            KNOWN_TERMINALS.join(", ")
        )
    })?;
    let argv = terminal_argv(&bin, cmd);
    std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
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
    use super::terminal_argv;

    #[test]
    fn parse_tty_maps_pts_and_rejects_none() {
        assert_eq!(parse_tty("pts/3"), Some("/dev/pts/3".to_string()));
        assert_eq!(parse_tty("pts/0\n"), Some("/dev/pts/0".to_string()));
        assert_eq!(parse_tty("??"), None);
        assert_eq!(parse_tty(""), None);
        assert_eq!(parse_tty("tty1"), None); // a real console, not our pty
    }

    #[test]
    fn terminal_argv_wraps_cmd_and_keeps_window_open() {
        // gnome-terminal uses `--` before the program
        let a = terminal_argv("gnome-terminal", "echo hi");
        assert_eq!(a, vec!["gnome-terminal", "--", "bash", "-lc", "echo hi; exec bash"]);
    }

    #[test]
    fn terminal_argv_per_terminal_exec_flags() {
        assert_eq!(terminal_argv("konsole", "X"),   vec!["konsole", "-e", "bash", "-lc", "X; exec bash"]);
        assert_eq!(terminal_argv("xterm", "X"),     vec!["xterm", "-e", "bash", "-lc", "X; exec bash"]);
        assert_eq!(terminal_argv("alacritty", "X"), vec!["alacritty", "-e", "bash", "-lc", "X; exec bash"]);
        assert_eq!(terminal_argv("kitty", "X"),     vec!["kitty", "bash", "-lc", "X; exec bash"]);
        assert_eq!(terminal_argv("foot", "X"),      vec!["foot", "bash", "-lc", "X; exec bash"]);
        assert_eq!(terminal_argv("xfce4-terminal", "X"), vec!["xfce4-terminal", "-x", "bash", "-lc", "X; exec bash"]);
        assert_eq!(terminal_argv("x-terminal-emulator", "X"), vec!["x-terminal-emulator", "-e", "bash", "-lc", "X; exec bash"]);
    }
}
