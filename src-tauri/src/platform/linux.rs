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
            .args(["-c", r#"command -v "$1" >/dev/null 2>&1"#, "sh", bin])
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

/// X11/Wayland give no portable way to focus a terminal window by its tty, so we
/// never offer the "reveal window" button on Linux.
pub fn can_reveal(_pid: i64) -> bool {
    false
}

/// Defensive: unreachable while `can_reveal` is false, but return a clear error.
pub fn reveal(_pid: i64) -> Result<(), String> {
    Err("revealing an existing terminal window isn't supported on Linux".into())
}

#[cfg(test)]
mod tests {
    use super::terminal_argv;

    #[test]
    fn terminal_argv_wraps_cmd_and_keeps_window_open() {
        // gnome-terminal uses `--` before the program
        let a = terminal_argv("gnome-terminal", "echo hi");
        assert_eq!(a, vec!["gnome-terminal", "--", "bash", "-lc", "echo hi; exec bash"]);
    }

    #[test]
    fn reveal_is_unsupported_on_linux() {
        assert_eq!(super::can_reveal(12345), false);
        assert!(super::reveal(12345).is_err());
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
