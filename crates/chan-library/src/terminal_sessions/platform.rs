use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

use portable_pty::CommandBuilder;

use super::{CreateError, FdPressure};

const TERMINAL_FD_HEADROOM: u64 = 32;
pub(super) const TERMINAL_SESSION_FD_ESTIMATE: u64 = 8;

/// Gate every spawn path on the Git BASH hard dependency (Windows only). When
/// it is absent there is no POSIX shell to spawn, so reject with the
/// structured [`CreateError::GitBashMissing`] the frontend turns into the
/// install gate — rather than silently falling back to `cmd` or surfacing an
/// opaque spawn error. A no-op on every other platform.
pub(super) fn reject_terminal_spawn_if_git_bash_missing() -> Result<(), CreateError> {
    #[cfg(windows)]
    if git_bash().is_none() {
        return Err(CreateError::GitBashMissing);
    }
    Ok(())
}

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
        windows_command_builder(command)
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

/// Windows terminal shell: **Git BASH** (a hard dependency of the Windows
/// design). Spawn it as a login shell (`bash -l`, `-lc` for one-shots) so its
/// MSYS `/etc/profile` sets up the POSIX environment. The PATH prepend that
/// makes `git`/coreutils/`cs` resolve happens in [`Session::spawn`] (it needs
/// the live env). Callers gate on [`git_bash`] via
/// [`reject_terminal_spawn_if_git_bash_missing`] before reaching here; the
/// `cmd` fallback is purely defensive for any unguarded path.
#[cfg(windows)]
fn windows_command_builder(command: Option<&str>) -> CommandBuilder {
    match git_bash() {
        Some(git) => {
            let mut cmd = CommandBuilder::new(&git.bash);
            match command {
                Some(command) => cmd.args(["-lc", command]),
                None => cmd.args(["-l"]),
            }
            cmd
        }
        None => match command {
            Some(command) => {
                let mut cmd = CommandBuilder::new("cmd");
                cmd.args(["/C", command]);
                cmd
            }
            None => CommandBuilder::new_default_prog(),
        },
    }
}

/// A resolved Git for Windows BASH install.
#[cfg(windows)]
pub(super) struct GitBashInstall {
    /// `<root>\bin\bash.exe` — the launcher that initialises the MSYS env.
    pub(super) bash: PathBuf,
    /// Extra Windows PATH entries (`<root>\usr\bin`, `<root>\mingw64\bin`) so
    /// `git`, the coreutils, and the `cs` shim resolve for the login shell and
    /// anything it spawns.
    pub(super) path_prepend: Vec<PathBuf>,
}

/// Resolve Git BASH once and cache the result (present or absent) for the
/// process lifetime — discovery shells out, and a terminal spawn is on the
/// interactive path.
#[cfg(windows)]
pub(super) fn git_bash() -> Option<&'static GitBashInstall> {
    static CACHE: std::sync::OnceLock<Option<GitBashInstall>> = std::sync::OnceLock::new();
    CACHE.get_or_init(resolve_git_bash).as_ref()
}

/// Force the [`git_bash`] cache to resolve eagerly, off the async request path.
/// [`resolve_git_bash`] shells out (`git --exec-path`, `reg query` ×2, `where
/// bash`) with blocking `std::process::Command`; resolving it lazily on the
/// first terminal create — which runs on a tokio worker (the embedded server
/// hosts the SPA, API, and WS on one runtime) — would block that worker and
/// freeze the SPA (W1). The server primes this once on a blocking thread at
/// startup, so the inline spawn gate
/// ([`reject_terminal_spawn_if_git_bash_missing`]) and [`windows_command_builder`]
/// only ever read the warm `OnceLock`.
// `pub` (not `pub(crate)`) because chan-server's route layer calls it
// cross-crate to prime the cache at server startup.
#[cfg(windows)]
pub fn prime_git_bash() {
    let _ = git_bash();
}

/// Discovery order (most reliable first), returning the first root whose
/// `bin\bash.exe` exists:
///   1. `git --exec-path` → walk up to the install root (skips WSL entirely).
///   2. Well-known install dirs under Program Files / per-user.
///   3. Registry `HKLM\...\GitForWindows\InstallPath` via `reg query`.
///   4. `where bash`, filtering out System32 / WindowsApps (the WSL `bash.exe`
///      launcher, which is NOT Git BASH).
///
/// No registry/winapi crate is pulled — `git`/`reg`/`where` are shelled out.
#[cfg(windows)]
fn resolve_git_bash() -> Option<GitBashInstall> {
    use std::process::Command;

    // 1. Derive the root from `git --exec-path`
    //    (`<root>\mingw64\libexec\git-core`): walk ancestors for `bin\bash.exe`.
    if let Ok(output) = Command::new("git").arg("--exec-path").output() {
        if output.status.success() {
            let exec_path = String::from_utf8_lossy(&output.stdout);
            let exec_path = PathBuf::from(exec_path.trim());
            for root in exec_path.ancestors() {
                if let Some(install) = git_bash_from_root(root) {
                    return Some(install);
                }
            }
        }
    }

    // 2. Well-known install roots.
    let mut roots: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Some(dir) = std::env::var_os(var) {
            roots.push(PathBuf::from(dir).join("Git"));
        }
    }
    if let Some(local) = std::env::var_os("LocalAppData") {
        roots.push(PathBuf::from(local).join("Programs").join("Git"));
    }
    for root in &roots {
        if let Some(install) = git_bash_from_root(root) {
            return Some(install);
        }
    }

    // 3. Registry InstallPath (32- and 64-bit views).
    for key in [
        r"HKLM\SOFTWARE\GitForWindows",
        r"HKLM\SOFTWARE\WOW6432Node\GitForWindows",
    ] {
        if let Ok(output) = Command::new("reg")
            .args([key, "/v", "InstallPath"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                // `    InstallPath    REG_SZ    C:\Program Files\Git`
                if let Some(path) = text
                    .lines()
                    .find_map(|line| line.split("REG_SZ").nth(1))
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                {
                    if let Some(install) = git_bash_from_root(Path::new(path)) {
                        return Some(install);
                    }
                }
            }
        }
    }

    // 4. `where bash`, skipping the WSL launcher under System32 / WindowsApps.
    if let Ok(output) = Command::new("where").arg("bash").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
                let lower = line.to_ascii_lowercase();
                if lower.contains(r"\system32\") || lower.contains(r"\windowsapps\") {
                    continue;
                }
                // `where bash` points at `<root>\bin\bash.exe`, so the install
                // root is two levels up.
                if let Some(root) = Path::new(line).parent().and_then(Path::parent) {
                    if let Some(install) = git_bash_from_root(root) {
                        return Some(install);
                    }
                }
            }
        }
    }

    None
}

/// Build a [`GitBashInstall`] from a candidate install root, or `None` if it
/// has no `bin\bash.exe`.
#[cfg(windows)]
fn git_bash_from_root(root: &Path) -> Option<GitBashInstall> {
    let bash = root.join("bin").join("bash.exe");
    if !bash.is_file() {
        return None;
    }
    let mut path_prepend = Vec::new();
    for sub in [["usr", "bin"], ["mingw64", "bin"], ["mingw32", "bin"]] {
        let dir = root.join(sub[0]).join(sub[1]);
        if dir.is_dir() {
            path_prepend.push(dir);
        }
    }
    Some(GitBashInstall { bash, path_prepend })
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
