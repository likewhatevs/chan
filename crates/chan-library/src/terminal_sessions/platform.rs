use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

use portable_pty::CommandBuilder;

use super::{CreateError, FdPressure};

const TERMINAL_FD_HEADROOM: u64 = 32;
pub(super) const TERMINAL_SESSION_FD_ESTIMATE: u64 = 8;

pub(super) fn reject_terminal_spawn_if_fd_pressure() -> Result<(), CreateError> {
    let Some((open, limit)) = fd_snapshot() else {
        return Ok(());
    };
    if fd_headroom_allows(open, limit, TERMINAL_SESSION_FD_ESTIMATE) {
        return Ok(());
    }
    Err(CreateError::FdPressure(FdPressure {
        open,
        limit,
        required: TERMINAL_SESSION_FD_ESTIMATE + TERMINAL_FD_HEADROOM,
    }))
}

pub(super) fn fd_headroom_allows(open: u64, limit: u64, new_fds: u64) -> bool {
    open.saturating_add(new_fds)
        .saturating_add(TERMINAL_FD_HEADROOM)
        < limit
}

#[cfg(unix)]
fn fd_snapshot() -> Option<(u64, u64)> {
    let open = std::fs::read_dir("/dev/fd").ok()?.count() as u64;
    let limit = nofile_limit()?;
    Some((open, limit))
}

#[cfg(not(unix))]
fn fd_snapshot() -> Option<(u64, u64)> {
    None
}

#[cfg(target_os = "linux")]
fn nofile_limit() -> Option<u64> {
    rustix::process::getrlimit(rustix::process::Resource::Nofile).current
}

#[cfg(target_os = "macos")]
fn nofile_limit() -> Option<u64> {
    rustix::process::getrlimit(rustix::process::Resource::Nofile).current
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn nofile_limit() -> Option<u64> {
    None
}

pub(super) fn path_inside_root(path: &Path, root: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path == root || path.starts_with(root)
}

#[cfg(target_os = "linux")]
pub(super) fn process_cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

#[cfg(target_os = "macos")]
pub(super) fn process_cwd(pid: u32) -> Option<PathBuf> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-a", "-d", "cwd", "-Fn", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n'))
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) fn process_cwd(_pid: u32) -> Option<PathBuf> {
    None
}

/// True when the requested or inherited environment already selects a UTF-8
/// codeset, following the standard LC_ALL > LC_CTYPE > LANG precedence. The
/// per-session overrides win over the server's own environment. When this is
/// false the spawned shell would fall back to the POSIX/C codeset and render
/// multibyte UTF-8 as raw bytes in pagers / editors like `less` and `vim`.
pub(super) fn locale_selects_utf8(requested: &BTreeMap<String, String>) -> bool {
    let lookup = |key: &str| -> Option<String> {
        requested
            .get(key)
            .cloned()
            .or_else(|| std::env::var(key).ok())
            .filter(|value| !value.is_empty())
    };
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Some(value) = lookup(key) {
            let value = value.to_ascii_lowercase();
            return value.contains("utf-8") || value.contains("utf8");
        }
    }
    false
}

/// Resolve the user's shell the same way an interactive terminal does:
/// `$SHELL` (when it points at an executable) → the passwd `pw_shell` →
/// `/bin/sh`. Single-sources the resolution so no caller hardcodes a fallback
/// shell. This is exactly `portable_pty`'s `new_default_prog().get_shell()`,
/// which performs and validates that lookup — reuse it rather than hand-rolling
/// `getpwuid`. Unix-only: `get_shell` is unix-only, and the Windows terminal
/// path is Git BASH, which never calls this.
#[cfg(unix)]
pub fn user_shell() -> String {
    CommandBuilder::new_default_prog().get_shell()
}

pub(super) fn command_builder(command: Option<&str>) -> CommandBuilder {
    let command = command.map(str::trim).filter(|command| !command.is_empty());
    #[cfg(windows)]
    {
        windows_shell().build(command)
    }
    #[cfg(not(windows))]
    {
        match command {
            // No command: the user's default interactive shell, exactly as
            // before (portable_pty resolves $SHELL / the passwd entry).
            None => CommandBuilder::new_default_prog(),
            // One-shot: run it through a login shell so profile-exported PATH
            // (where `cs` lives) is in scope. The shell is resolved via
            // `user_shell` ($SHELL → passwd → /bin/sh, validated) — single-sourced
            // with the interactive path above, never a hardcoded `/bin/sh`.
            Some(command) => {
                let mut cmd = CommandBuilder::new(user_shell());
                cmd.args(["-lc", command]);
                cmd
            }
        }
    }
}

/// The user's default Windows terminal shell, resolved once and cached.
#[cfg(windows)]
pub(super) struct WindowsShell {
    program: PathBuf,
    kind: WinShellKind,
}

/// How the resolved Windows shell takes its interactive / one-shot arguments.
#[cfg(windows)]
#[derive(Clone, Copy)]
enum WinShellKind {
    /// `powershell.exe` / `pwsh.exe`: `-NoLogo` interactive, `-NoLogo -Command`
    /// one-shot. No `-NoProfile` — we want the user's profile/PATH (the `-l`
    /// analog of the unix login shell).
    PowerShell,
    /// `cmd.exe`: no args interactive, `/C` one-shot.
    Cmd,
    /// Any other shell a user points `CHAN_SHELL` at (a POSIX `sh`/`bash`,
    /// including a Git BASH they install themselves): `-l` / `-lc`, matching the
    /// unix login-shell convention.
    Posix,
}

#[cfg(windows)]
impl WindowsShell {
    fn build(&self, command: Option<&str>) -> CommandBuilder {
        let mut cmd = CommandBuilder::new(&self.program);
        match (self.kind, command) {
            (WinShellKind::PowerShell, None) => {
                cmd.arg("-NoLogo");
            }
            (WinShellKind::PowerShell, Some(c)) => {
                cmd.args(["-NoLogo", "-Command", c]);
            }
            (WinShellKind::Cmd, None) => {}
            (WinShellKind::Cmd, Some(c)) => {
                cmd.args(["/C", c]);
            }
            (WinShellKind::Posix, None) => {
                cmd.arg("-l");
            }
            (WinShellKind::Posix, Some(c)) => {
                cmd.args(["-lc", c]);
            }
        }
        cmd
    }
}

/// Resolve the user's default Windows shell once and cache it for the process
/// lifetime — resolution shells out (`where pwsh`), and a terminal spawn is on
/// the interactive path.
#[cfg(windows)]
pub(super) fn windows_shell() -> &'static WindowsShell {
    static CACHE: std::sync::OnceLock<WindowsShell> = std::sync::OnceLock::new();
    CACHE.get_or_init(resolve_windows_shell)
}

/// Force the [`windows_shell`] cache to resolve eagerly, off the async request
/// path. Resolution may shell out (`where pwsh`) with blocking
/// `std::process::Command`; resolving it lazily on the first terminal create —
/// which runs on a tokio worker (the embedded server hosts the SPA, API, and WS
/// on one runtime) — would block that worker and freeze the SPA. The server
/// primes this once on a blocking thread at startup, so [`windows_shell`] only
/// ever reads the warm `OnceLock`.
// `pub` (not `pub(crate)`) because chan-server's route layer calls it
// cross-crate to prime the cache at server startup.
#[cfg(windows)]
pub fn prime_windows_shell() {
    let _ = windows_shell();
}

/// Classify a shell program by its file stem so a `CHAN_SHELL` override gets the
/// right argument convention. Unknown stems are treated as POSIX (`-lc`), which
/// is the useful fallback for a user who points `CHAN_SHELL` at a `bash`/`sh`.
#[cfg(windows)]
fn classify_windows_shell(program: &Path) -> WinShellKind {
    let stem = program
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match stem.as_str() {
        "pwsh" | "powershell" => WinShellKind::PowerShell,
        "cmd" => WinShellKind::Cmd,
        _ => WinShellKind::Posix,
    }
}

/// Resolve the Windows shell, first match wins:
///   1. `CHAN_SHELL` (verbatim path/name; arg convention inferred from the stem).
///   2. `pwsh.exe` (PowerShell 7) if on PATH.
///   3. `powershell.exe` (Windows PowerShell 5, in-box on every supported Windows).
///   4. `%ComSpec%` / `cmd.exe`.
#[cfg(windows)]
fn resolve_windows_shell() -> WindowsShell {
    use std::process::Command;

    // 1. Explicit override.
    if let Some(raw) = std::env::var_os("CHAN_SHELL").filter(|v| !v.is_empty()) {
        let program = PathBuf::from(raw);
        let kind = classify_windows_shell(&program);
        return WindowsShell { program, kind };
    }

    // 2. PowerShell 7 (pwsh) if installed.
    if let Ok(output) = Command::new("where").arg("pwsh").output() {
        if output.status.success() {
            if let Some(path) = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
            {
                return WindowsShell {
                    program: PathBuf::from(path),
                    kind: WinShellKind::PowerShell,
                };
            }
        }
    }

    // 3. Windows PowerShell 5, the in-box default. Prefer the full System32
    //    path so a modified PATH can't shadow it; fall back to the bare name.
    let powershell = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("windir"))
        .map(|root| PathBuf::from(root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe"))
        .filter(|p| p.is_file());
    if let Some(program) = powershell {
        return WindowsShell {
            program,
            kind: WinShellKind::PowerShell,
        };
    }

    // 4. %ComSpec% / cmd.exe — the last-resort default.
    let comspec = std::env::var_os("ComSpec")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows\System32\cmd.exe"));
    WindowsShell {
        program: comspec,
        kind: WinShellKind::Cmd,
    }
}

pub(crate) fn set_mcp_env(cmd: &mut CommandBuilder, socket_path: &std::path::Path) {
    let Some(socket) = socket_path.to_str() else {
        return;
    };
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(exe) = exe.to_str() else {
        return;
    };
    let argv_json = serde_json::json!([exe, "__mcp-proxy", socket]).to_string();
    let server_json = serde_json::json!({
        "name": "chan",
        "command": exe,
        "args": ["__mcp-proxy", socket],
    })
    .to_string();

    cmd.env("CHAN_MCP_SERVER_NAME", "chan");
    cmd.env("CHAN_MCP_SOCKET", socket);
    cmd.env("CHAN_MCP_COMMAND", format!("{exe} __mcp-proxy {socket}"));
    cmd.env("CHAN_MCP_COMMAND_JSON", argv_json);
    cmd.env("CHAN_MCP_SERVER_JSON", server_json);
}

pub(super) fn clear_mcp_env(cmd: &mut CommandBuilder) {
    for key in [
        "CHAN_MCP_SERVER_NAME",
        "CHAN_MCP_SOCKET",
        "CHAN_MCP_COMMAND",
        "CHAN_MCP_COMMAND_JSON",
        "CHAN_MCP_SERVER_JSON",
        "CHAN_TAB_GROUP",
        "CHAN_WINDOW_ID",
        "CHAN_CONTROL_SOCKET",
        "CHAN_WORKSPACE_NAME",
        "CHAN_WORKSPACE_PATH",
    ] {
        cmd.env_remove(key);
    }
}

pub(crate) fn terminal_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}
