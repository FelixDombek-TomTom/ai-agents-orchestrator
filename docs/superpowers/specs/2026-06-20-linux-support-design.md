# Linux support for AI Agents Orchestrator

**Date:** 2026-06-20
**Status:** Approved (pending spec review)

## Goal

Make the app build and run on Linux (Ubuntu 24.04, GNOME/Wayland) without
regressing macOS. The app is a Tauri 2 app, so the framework is already
cross-platform; "macOS only" is an artifact of a handful of OS-specific syscalls
inlined in `src-tauri/src/lib.rs` (`open`, `osascript`, `ps`) plus toolchain
assumptions. The embedded terminal (xterm.js + `portable-pty`), the `~/.claude`
reader, config, and all UI are already portable and unchanged.

## Architecture

All OS-specific *execution* is currently inlined in `lib.rs`. Extract it into a
new `platform` module with per-OS implementations behind one internal API.
`lib.rs` keeps all *validation* (URL scheme checks, leading-dash guards,
shell-quoting, regex allowlists for session id / branch / category) and calls
`platform::*` only for the syscall. This isolates every `#[cfg(target_os = ...)]`
in one place instead of scattering branches through the command handlers.

```
src-tauri/src/platform/mod.rs    — internal API + #[cfg] re-export of macos|linux
src-tauri/src/platform/macos.rs  — existing impls moved verbatim
src-tauri/src/platform/linux.rs  — new implementations
```

### Internal API (platform::)

| Function | Purpose |
|---|---|
| `open_uri(uri: &str) -> Result<(), String>` | open an http(s) URL in the browser |
| `open_path(path: &Path) -> Result<(), String>` | reveal a path in the file manager |
| `launch_in_terminal(cmd: &str, selector: &str) -> Result<(), String>` | run a shell command in an external terminal; `selector` = `config.terminalApp` |
| `session_tty(pid: i64) -> Option<String>` | controlling tty of a pid, or None |
| `can_reveal(pid: i64) -> bool` | can we focus an existing terminal window for this session? |
| `reveal(pid: i64) -> Result<(), String>` | bring that window to the front |

The `lib.rs` `#[tauri::command]` wrappers (`open_external`, `open_path`,
`open_in_terminal`, `can_reveal_terminal`, `reveal_terminal`, and the
`start_session` path) keep their current validation and delegate execution to
these functions. Command names and signatures are unchanged, so the renderer is
untouched.

### macOS implementation (platform/macos.rs)

Existing functions moved verbatim — no behavior change:
- `open_uri`/`open_path` → `open -- <arg>`
- `launch_in_terminal` → `osascript` into iTerm2 or Terminal.app (selector
  allowlist: `iterm` | `terminal` | default = iTerm if installed else Terminal)
- `session_tty` → `ps -o tty=`, accept `ttysNNN` → `/dev/ttysNNN`
- `can_reveal`/`reveal` → AppleScript `scan_terminals` / `reveal_in`

### Linux implementation (platform/linux.rs)

| Function | Behavior |
|---|---|
| `open_uri` / `open_path` | `xdg-open -- <arg>` |
| `launch_in_terminal` | resolve a terminal, then spawn it running `bash -lc '<cmd>; exec bash'` so the window stays open after `claude` exits (matches macOS `do script`). |
| `session_tty` | `ps -o tty=`, accept `pts/N` → `/dev/pts/N` (Linux `ps` prints `pts/0`, not `ttys003`) |
| `can_reveal` | always `false` — there is no portable way to focus a terminal window by tty on X11/Wayland, so the UI never offers the button |
| `reveal` | `Err("revealing an existing terminal window isn't supported on Linux")` — defensive; should be unreachable since `can_reveal` is false |

**Terminal resolution order** (first match wins):
1. `selector` mapped to a known binary when non-empty:
   `gnome-terminal` | `konsole` | `xterm` | `xfce4-terminal` | `kitty` |
   `alacritty` | `foot` (the selector string is matched against this
   allowlist; unknown selector falls through to auto-detect)
2. `$TERMINAL` env var, if set and on `PATH`
3. `x-terminal-emulator` (Debian/Ubuntu update-alternatives wrapper)
4. First available of: `gnome-terminal`, `konsole`, `xfce4-terminal`, `kitty`,
   `alacritty`, `foot`, `xterm`

If none found → `Err` listing what was tried.

**Argv construction** is split into a pure helper
`terminal_argv(bin, cmd) -> Vec<String>` (no spawn) so it is unit-testable.
Each known terminal needs its own exec-flag shape:
- `gnome-terminal -- bash -lc <script>`
- `konsole -e bash -lc <script>`
- `xfce4-terminal -x bash -lc <script>` (note: `-x` consumes the rest)
- `kitty bash -lc <script>`
- `alacritty -e bash -lc <script>`
- `foot bash -lc <script>`
- `xterm -e bash -lc <script>`
- `x-terminal-emulator -e bash -lc <script>`

where `<script>` = `<cmd>; exec bash`. `cmd` is already validated and
shell-quoted by the caller; it is passed as a single argv element to `bash -lc`,
never re-interpolated, so no injection.

## Build / dependencies

- **System (apt, done):** `libwebkit2gtk-4.1-dev libsoup-3.0-dev build-essential
  curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  pkg-config`
- **Rust:** rustup stable + `cargo install tauri-cli`
- `tauri.conf.json` `app.windows[].titleBarStyle: "Overlay"` is macOS-only and
  silently ignored on Linux — left untouched.
- `bundle.targets: "all"` produces `.deb`/`.AppImage`/`.rpm` on Linux
  automatically — no config change.

## Testing

- **Unit (`#[cfg(target_os = "linux")]`):**
  - `session_tty` pts parsing: `pts/3` → `/dev/pts/3`; `??`/empty → None
  - `terminal_argv` mapping: each known binary produces the expected argv shape
    with `<cmd>; exec bash` as the final element
- **Existing macOS tests** stay under `#[cfg(target_os = "macos")]` where they
  reference mac-only helpers; shared/pure tests stay platform-neutral.
- **Manual on this machine:**
  1. `cargo tauri dev` launches the dashboard
  2. sessions auto-discovered from `~/.claude`
  3. embedded-terminal Resume works (resumes a real `claude --resume`)
  4. "open in your own terminal" opens `gnome-terminal` running the resume
  5. the "reveal window" button is absent (can_reveal = false)
  6. clicking a Jira/PR link opens the system browser via `xdg-open`

## Out of scope

- Signed/notarized or polished `.deb`/`.AppImage` release packaging
- CI matrix changes (adding a Linux runner)
- Focusing an existing external terminal window on Linux
- Non-GNOME desktop verification (code supports KDE/others via the resolution
  list, but only GNOME/Wayland is verified here)
