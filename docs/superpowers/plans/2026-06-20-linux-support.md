# Linux Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make AI Agents Orchestrator build and run on Linux (Ubuntu 24.04, GNOME/Wayland) without regressing macOS.

**Architecture:** Extract every OS-specific syscall (`open`, `osascript`, `ps`) out of `src-tauri/src/lib.rs` into a new `platform` module with per-OS files (`macos.rs`, `linux.rs`) behind one internal API. `lib.rs` keeps all input validation and `#[tauri::command]` wrappers; it calls `platform::*` only to execute. Every `#[cfg(target_os = ...)]` lives in `platform/mod.rs`.

**Tech Stack:** Rust (edition 2021, rustc ≥1.82), Tauri 2.11, `portable-pty`. No new crates.

## Global Constraints

- rustc ≥ 1.82 (from `src-tauri/Cargo.toml` `rust-version`).
- No new Cargo dependencies — all current deps (`dirs`, `portable-pty`, `trash`, `libc`, `tauri`) are already cross-platform.
- macOS behavior must not change: macOS code is **moved verbatim**, only relocated and (for `launch_in_terminal`) given its selector as a parameter instead of reading config inline.
- `lib.rs` `#[tauri::command]` names and signatures are unchanged — the renderer is not touched.
- Validation (URL scheme, leading-dash, shell-quoting, session-id / branch / category allowlists) stays in `lib.rs`. `platform` only performs syscalls.
- This machine can only compile the **Linux** target. macOS files (`#[cfg(target_os = "macos")]`) are not compiled here; their correctness rests on being a verbatim move. Do **not** attempt to verify the macOS build locally.
- Build/test commands assume `source "$HOME/.cargo/env"` is in effect (cargo 1.96 at `~/.cargo/bin`). Run cargo commands from `src-tauri/`.

---

## File Structure

- `src-tauri/src/platform/mod.rs` — **create.** Declares submodules + `#[cfg]` re-export. The only place `#[cfg(target_os)]` appears.
- `src-tauri/src/platform/macos.rs` — **create.** macOS impls moved verbatim from `lib.rs`.
- `src-tauri/src/platform/linux.rs` — **create.** New Linux impls + unit tests.
- `src-tauri/src/lib.rs` — **modify.** Add `mod platform;`; delete the moved helpers; point command wrappers at `platform::*`.

The platform API (identical fn set in both `macos.rs` and `linux.rs`):

```rust
pub fn open_uri(uri: &str) -> Result<(), String>;
pub fn open_path(path: &std::path::Path) -> Result<(), String>;
pub fn launch_in_terminal(cmd: &str, selector: &str) -> Result<(), String>;
pub fn can_reveal(pid: i64) -> bool;
pub fn reveal(pid: i64) -> Result<(), String>;
```

---

## Task 1: Platform module scaffold + browser/file-manager open

**Files:**
- Create: `src-tauri/src/platform/mod.rs`
- Create: `src-tauri/src/platform/macos.rs`
- Create: `src-tauri/src/platform/linux.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod platform;`; rewrite `open_external` and `open_path` bodies)

**Interfaces:**
- Produces: `platform::open_uri(&str) -> Result<(), String>`, `platform::open_path(&Path) -> Result<(), String>`.

- [ ] **Step 1: Create `platform/mod.rs`**

```rust
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
```

- [ ] **Step 2: Create `platform/macos.rs` with the two open fns moved verbatim**

Move the `open`-based bodies out of `lib.rs:14-19` (`open_external`) and `lib.rs:33-38` (`open_path`) into free functions. Validation stays in `lib.rs`; these take already-validated inputs:

```rust
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
```

- [ ] **Step 3: Create `platform/linux.rs` with `xdg-open` versions**

```rust
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
```

- [ ] **Step 4: Wire `lib.rs`** — add the module and delegate

At the top of `lib.rs`, after `mod reader;` (line 3), add:

```rust
mod platform;
```

Replace the body of `open_external` (`lib.rs:14-19`) so the function keeps its scheme check and calls platform:

```rust
    platform::open_uri(&url)
```

(i.e. the function becomes: the existing `if !(url.starts_with("http://") ...) { return Err(...) }` guard, then `platform::open_uri(&url)`.)

Replace the body of `open_path` (`lib.rs:33-38`) so it keeps the leading-dash check + canonicalize, then:

```rust
    platform::open_path(&abs)
```

- [ ] **Step 5: Build (Linux target)**

Run: `cd src-tauri && cargo build`
Expected: compiles clean (warnings about not-yet-used `launch_in_terminal`/`session_tty` helpers in `lib.rs` are fine for now — they get moved in later tasks).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/platform src-tauri/src/lib.rs
git commit -m "refactor: extract browser/file open into platform module (+linux xdg-open)"
```

---

## Task 2: session_tty (pure parse + Linux pts support)

**Files:**
- Modify: `src-tauri/src/platform/macos.rs` (add `session_tty`, moved from `lib.rs:107-121`)
- Modify: `src-tauri/src/platform/linux.rs` (add `session_tty` + `parse_tty` + test)
- Modify: `src-tauri/src/lib.rs` (delete the inline `session_tty`, `lib.rs:107-121`)

**Interfaces:**
- Produces: `platform::session_tty(pid: i64) -> Option<String>` (returns `/dev/ttysN` on macOS, `/dev/pts/N` on Linux).

- [ ] **Step 1: Write the failing test in `platform/linux.rs`**

Add at the bottom of `linux.rs`:

```rust
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
```

- [ ] **Step 2: Run it — verify it fails to compile (no `parse_tty`)**

Run: `cd src-tauri && cargo test --lib platform::linux 2>&1 | head`
Expected: FAIL — `cannot find function parse_tty`.

- [ ] **Step 3: Implement `parse_tty` + `session_tty` in `linux.rs`**

Add above the `#[cfg(test)]` module:

```rust
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
```

- [ ] **Step 4: Run the test — verify it passes**

Run: `cd src-tauri && cargo test --lib platform::linux`
Expected: PASS (`parse_tty_maps_pts_and_rejects_none ... ok`).

- [ ] **Step 5: Add `session_tty` to `macos.rs` (verbatim move)**

Move `lib.rs:107-121` into `macos.rs` as `pub fn session_tty(pid: i64) -> Option<String>` — unchanged body (the `ps -o tty=` call and the `ttys`-prefix check that yields `/dev/ttysNNN`).

- [ ] **Step 6: Delete the inline `session_tty` from `lib.rs`**

Remove `lib.rs:104-121` (the `/// The controlling tty …` doc comment + `fn session_tty`). It is now only reached through `platform`. Build to confirm nothing else references the local one:

Run: `cd src-tauri && cargo build`
Expected: compiles (the only callers were `can_reveal_terminal`/`reveal_terminal`, rewired in Task 4; until then they still reference the local `scan_terminals`, which itself calls the local `session_tty` — so move `session_tty` only after confirming `scan_terminals`/`reveal_in` are NOT yet moved). If the build errors on a missing `session_tty`, leave a temporary `use platform::session_tty;` is **not** wanted — instead, defer this deletion: keep `lib.rs`'s `session_tty` until Task 4 removes `scan_terminals`. See note.

> **Ordering note:** `scan_terminals` (`lib.rs:191-196`) and `reveal_in` (`lib.rs:127-187`) still live in `lib.rs` after this task and call the local `session_tty` indirectly via `can_reveal_terminal`/`reveal_terminal`. To keep the build green, in this task **add** `session_tty` to both platform files and add the `parse_tty` test, but **do not delete** the `lib.rs` copy yet — its deletion happens in Task 4 alongside `reveal_in`/`scan_terminals`. Adjust Step 6 accordingly: just `cargo build` to confirm the additions compile.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/platform src-tauri/src/lib.rs
git commit -m "feat(linux): session_tty via pts parsing; mirror macOS into platform"
```

---

## Task 3: External terminal launch

**Files:**
- Modify: `src-tauri/src/platform/macos.rs` (add `launch_in_terminal` + `run_in_iterm_tab` + `run_in_terminal_app`, moved from `lib.rs:45-102`, selector passed as a param)
- Modify: `src-tauri/src/platform/linux.rs` (add `launch_in_terminal` + `resolve_terminal` + `terminal_argv` + tests)
- Modify: `src-tauri/src/lib.rs` (replace `run_in_iterm_tab`/`run_in_terminal_app`/`launch_in_terminal` with a thin wrapper)

**Interfaces:**
- Consumes: nothing new.
- Produces: `platform::launch_in_terminal(cmd: &str, selector: &str) -> Result<(), String>`. `selector` is the lowercased `config.terminalApp` (`""` = auto).

- [ ] **Step 1: Write failing tests in `platform/linux.rs`**

Add to the existing `#[cfg(test)] mod tests` in `linux.rs`:

```rust
    use super::terminal_argv;

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
```

- [ ] **Step 2: Run — verify it fails to compile (no `terminal_argv`)**

Run: `cd src-tauri && cargo test --lib platform::linux 2>&1 | head`
Expected: FAIL — `cannot find function terminal_argv`.

- [ ] **Step 3: Implement the Linux launcher in `linux.rs`**

```rust
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
```

- [ ] **Step 4: Run the tests — verify they pass**

Run: `cd src-tauri && cargo test --lib platform::linux`
Expected: PASS (all three `terminal_argv*` / `parse_tty*` tests ok).

- [ ] **Step 5: Move the macOS launcher into `macos.rs`**

Move `run_in_iterm_tab` (`lib.rs:45-62`) and `run_in_terminal_app` (`lib.rs:68-82`) verbatim into `macos.rs` as private fns. Move `launch_in_terminal` (`lib.rs:89-102`) into `macos.rs` as `pub fn launch_in_terminal(cmd: &str, selector: &str)`, **replacing** its internal config read (`lib.rs:90-94`) with the passed-in `selector`:

```rust
pub fn launch_in_terminal(cmd: &str, selector: &str) -> Result<(), String> {
    match selector {
        "iterm" => run_in_iterm_tab(cmd),
        "terminal" => run_in_terminal_app(cmd),
        _ if std::path::Path::new("/Applications/iTerm.app").exists() => run_in_iterm_tab(cmd),
        _ => run_in_terminal_app(cmd),
    }
}
```

- [ ] **Step 6: Replace the `lib.rs` launcher with a thin wrapper**

Delete `lib.rs:41-102` (`run_in_iterm_tab`, `run_in_terminal_app`, and the old `launch_in_terminal`). Add this wrapper in their place (call sites `open_in_terminal`, `start_session`, `restore_session` keep calling `launch_in_terminal(&cmd)` unchanged):

```rust
/// Read the terminal selector from config and execute `cmd` in an external
/// terminal via the platform layer. `cmd` is already validated + shell-quoted.
fn launch_in_terminal(cmd: &str) -> Result<(), String> {
    let sel = config::load()
        .get("terminalApp")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    platform::launch_in_terminal(cmd, &sel)
}
```

- [ ] **Step 7: Build + test**

Run: `cd src-tauri && cargo build && cargo test --lib`
Expected: compiles; all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/platform src-tauri/src/lib.rs
git commit -m "feat(linux): launch external terminals (auto-detect + selector)"
```

---

## Task 4: Reveal-terminal (macOS focus; Linux no-op)

**Files:**
- Modify: `src-tauri/src/platform/macos.rs` (add `reveal_in` + `scan_terminals` + `can_reveal` + `reveal`, moved from `lib.rs:127-219`)
- Modify: `src-tauri/src/platform/linux.rs` (add `can_reveal` + `reveal` + test)
- Modify: `src-tauri/src/lib.rs` (delete `reveal_in`/`scan_terminals`/`session_tty`; rewire the two commands)

**Interfaces:**
- Consumes: `platform::session_tty` (Task 2).
- Produces: `platform::can_reveal(pid: i64) -> bool`, `platform::reveal(pid: i64) -> Result<(), String>`.

- [ ] **Step 1: Write the failing Linux test in `linux.rs`**

Add to `linux.rs`'s `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn reveal_is_unsupported_on_linux() {
        assert_eq!(super::can_reveal(12345), false);
        assert!(super::reveal(12345).is_err());
    }
```

- [ ] **Step 2: Run — verify it fails to compile (no `can_reveal`/`reveal`)**

Run: `cd src-tauri && cargo test --lib platform::linux 2>&1 | head`
Expected: FAIL — `cannot find function can_reveal`.

- [ ] **Step 3: Implement the Linux stubs in `linux.rs`**

```rust
/// X11/Wayland give no portable way to focus a terminal window by its tty, so we
/// never offer the "reveal window" button on Linux.
pub fn can_reveal(_pid: i64) -> bool {
    false
}

/// Defensive: unreachable while `can_reveal` is false, but return a clear error.
pub fn reveal(_pid: i64) -> Result<(), String> {
    Err("revealing an existing terminal window isn't supported on Linux".into())
}
```

- [ ] **Step 4: Run the test — verify it passes**

Run: `cd src-tauri && cargo test --lib platform::linux`
Expected: PASS.

- [ ] **Step 5: Move the macOS reveal stack into `macos.rs`**

Move verbatim into `macos.rs` as private fns: `reveal_in` (`lib.rs:127-187`) and `scan_terminals` (`lib.rs:191-196`). Then add the two public entry points (bodies moved from the old `can_reveal_terminal` `lib.rs:202-206` and `reveal_terminal` `lib.rs:212-218`), now using the macOS `session_tty` already in this file:

```rust
pub fn can_reveal(pid: i64) -> bool {
    match session_tty(pid) {
        Some(tty) => scan_terminals(&tty, false),
        None => false,
    }
}

pub fn reveal(pid: i64) -> Result<(), String> {
    let tty = session_tty(pid).ok_or("session has no terminal tty")?;
    if scan_terminals(&tty, true) {
        Ok(())
    } else {
        Err("couldn't find that terminal window".into())
    }
}
```

- [ ] **Step 6: Delete the moved code from `lib.rs` and rewire the commands**

Delete from `lib.rs`: `session_tty` (`lib.rs:104-121`, deferred from Task 2), `reveal_in` (`lib.rs:123-187`), `scan_terminals` (`lib.rs:189-196`). Keep the two `#[tauri::command]` wrappers but replace their bodies:

```rust
#[tauri::command(async)]
fn can_reveal_terminal(pid: i64) -> bool {
    platform::can_reveal(pid)
}

#[tauri::command(async)]
fn reveal_terminal(pid: i64) -> Result<(), String> {
    platform::reveal(pid)
}
```

(Keep their existing doc comments and the `#[tauri::command(async)]` attribute. They remain registered in `generate_handler!` at `lib.rs:852-853` — do not touch that list.)

- [ ] **Step 7: Build + full test run**

Run: `cd src-tauri && cargo build && cargo test --lib`
Expected: compiles with no warnings about unused `session_tty`/`reveal_in`/`scan_terminals` (all gone from `lib.rs`); all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/platform src-tauri/src/lib.rs
git commit -m "refactor: move reveal-terminal into platform; linux returns unsupported"
```

---

## Task 5: Build + run verification

**Files:** none (verification only). Any fix needed lands in the relevant earlier file with its own commit.

- [ ] **Step 1: Release build compiles**

Run: `cd src-tauri && cargo build --release`
Expected: builds clean.

- [ ] **Step 2: Full lib test suite green**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass (existing `lib.rs` injection-boundary tests + new `platform::linux` tests).

- [ ] **Step 3: App launches (manual, GUI)**

Run (from repo root): `source "$HOME/.cargo/env" && cargo tauri dev`
Expected: the dashboard window opens; sessions from `~/.claude` are listed. If `scripts/install.sh` has not been run, run `bash scripts/install.sh` first so a config + category folders exist.

- [ ] **Step 4: Manual behavior checks** (tick each)

  - [ ] Embedded-terminal Resume on a session starts a real `claude --resume` in the in-app terminal.
  - [ ] "Open in your own terminal" opens `gnome-terminal` running the resume command, and the window stays open after exit.
  - [ ] A session's "reveal window" button is **absent** (because `can_reveal` is false).
  - [ ] Clicking a Jira/PR link opens the system browser (via `xdg-open`).

- [ ] **Step 5: Final commit (only if any fix was needed)**

```bash
git add -A && git commit -m "fix: linux run verification adjustments"
```

---

## Self-Review

- **Spec coverage:** module layout ✓ (T1), internal API ✓ (T1–T4), macOS verbatim move ✓ (T1–T4), Linux `xdg-open` ✓ (T1), `session_tty` pts ✓ (T2), terminal resolution order + argv ✓ (T3), `can_reveal=false` / `reveal=Err` ✓ (T4), build/deps + manual checks ✓ (T5). titleBarStyle/bundle: no code change — covered by spec "left untouched", nothing to implement.
- **Placeholder scan:** none — every code step has complete code; the one ordering subtlety (deleting `lib.rs` `session_tty`) is resolved explicitly in T2 Step 6's note and executed in T4 Step 6.
- **Type consistency:** `launch_in_terminal(cmd, selector)`, `open_uri(&str)`, `open_path(&Path)`, `session_tty(i64)->Option<String>`, `can_reveal(i64)->bool`, `reveal(i64)->Result<(),String>` used identically across `mod.rs`, `macos.rs`, `linux.rs`, and `lib.rs` call sites.
