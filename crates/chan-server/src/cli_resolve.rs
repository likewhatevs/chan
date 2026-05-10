//! Resolve external CLI binaries (claude / gemini) for the agentic
//! backends.
//!
//! The status route needs to know whether the configured `cmd[0]`
//! resolves to an executable on this machine: chan can be launched
//! without the user's interactive-shell PATH (launchd, desktop
//! double-click, fresh systemd unit), and the agent backends inherit
//! that environment. We mirror `execvp` semantics — explicit paths
//! used as-is, bare names walk `$PATH` left-to-right — and on a
//! PATH miss fall back to well-known install locations for each
//! installer (Anthropic's, Homebrew, npm-global, etc).

use std::path::{Path, PathBuf};

/// Probe whether the resolved claude_cli `cmd[0]` is reachable from
/// this process. Mirrors `execvp` semantics first: when the name
/// carries a path separator, treat it as an explicit path and only
/// check that location; otherwise walk `$PATH` left-to-right and
/// return the first executable hit. When the PATH walk misses (or
/// PATH is unset), fall back to well-known install dirs so a chan
/// launched without the user's interactive-shell PATH (launchd,
/// desktop .app double-click, fresh systemd unit) can still locate
/// a claude installed by Anthropic's official installer or by
/// Homebrew. Returns None when not found anywhere.
pub fn resolve_claude_cli(cmd0: &str) -> Option<PathBuf> {
    resolve_cli_binary(cmd0, claude_cli_fallback_dirs)
}

/// Same shape as `resolve_claude_cli` but with a gemini-cli-shaped
/// fallback dir set (npm-global locations rather than `~/.claude`).
pub fn resolve_gemini_cli(cmd0: &str) -> Option<PathBuf> {
    resolve_cli_binary(cmd0, gemini_cli_fallback_dirs)
}

fn resolve_cli_binary(cmd0: &str, fallback_dirs: fn() -> Vec<PathBuf>) -> Option<PathBuf> {
    if cmd0.is_empty() {
        return None;
    }
    let p = Path::new(cmd0);
    if p.components().count() > 1 {
        return is_executable(p).then(|| p.to_path_buf());
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if let Some(hit) = probe_in_dir(&dir, cmd0) {
                return Some(hit);
            }
        }
    }
    for dir in fallback_dirs() {
        if let Some(hit) = probe_in_dir(&dir, cmd0) {
            return Some(hit);
        }
    }
    None
}

fn probe_in_dir(dir: &Path, cmd0: &str) -> Option<PathBuf> {
    let candidate = dir.join(cmd0);
    if is_executable(&candidate) {
        return Some(candidate);
    }
    // PATHEXT walk on Windows: the shell appends .exe / .cmd /
    // .bat when the bare name doesn't resolve. Skipped on Unix
    // where the executable bit is the only signal.
    #[cfg(windows)]
    {
        for ext in ["exe", "cmd", "bat", "com"] {
            let with_ext = candidate.with_extension(ext);
            if is_executable(&with_ext) {
                return Some(with_ext);
            }
        }
    }
    None
}

/// Well-known install locations checked after a PATH miss. Order
/// matters: Anthropic's official installer (`~/.claude/local/bin`)
/// wins over Homebrew, which wins over distro packages, so that an
/// explicit user install is preferred when more than one is
/// present. Windows entries cover the npm-global default
/// (`%APPDATA%\npm`) since the official installer ships claude as
/// an npm package on Windows.
fn claude_cli_fallback_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(unix)]
    {
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(
                PathBuf::from(&home)
                    .join(".claude")
                    .join("local")
                    .join("bin"),
            );
        }
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/usr/bin"));
    }
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            dirs.push(PathBuf::from(&appdata).join("npm"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(&local).join("Programs").join("claude"));
        }
    }
    dirs
}

/// Well-known install locations for the gemini CLI. gemini-cli ships
/// as an npm package (`@google/gemini-cli`), so the fallbacks target
/// npm-global bin dirs in addition to the standard system bins. Order
/// puts a user-prefix npm install first, then Homebrew, then system.
fn gemini_cli_fallback_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(unix)]
    {
        if let Some(home) = std::env::var_os("HOME") {
            // Common `npm config set prefix=~/.npm-global` layout.
            dirs.push(PathBuf::from(&home).join(".npm-global").join("bin"));
            // nvm's per-version bin dir; not enumerated per-version
            // (we'd need to read .nvmrc). The bare default-version
            // symlink lives in the user's PATH on most setups, so a
            // PATH miss here is rare.
            dirs.push(PathBuf::from(&home).join(".nvm").join("versions"));
        }
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/usr/bin"));
    }
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            dirs.push(PathBuf::from(&appdata).join("npm"));
        }
    }
    dirs
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// `~/.chan/api-keys.toml`-style path the on-disk fallback uses.
/// Hardcoded here because chan-llm doesn't expose a public path
/// helper; the Settings UI surfaces this so users on headless
/// boxes know which file to edit. Stays in lockstep with chan-llm's
/// internal `default_path()` for keys.
pub fn api_keys_path_string() -> String {
    chan_drive::paths::config_dir()
        .join("api-keys.toml")
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Process-env is global; cargo runs tests in parallel by
    /// default. Any test that mutates PATH / HOME holds this guard
    /// for its critical section so two probes can't observe each
    /// other's overrides.
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }

    #[cfg(unix)]
    #[test]
    fn resolve_claude_cli_finds_via_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("claude");
        std::fs::write(&bin, b"#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();

        let _g = env_lock();
        let prev = std::env::var_os("PATH");
        // Empty HOME so the fallback can't accidentally satisfy a
        // missing-PATH-entry test against the real ~/.claude tree.
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("PATH", dir.path());
        std::env::remove_var("HOME");
        let resolved = resolve_claude_cli("claude");
        match prev {
            Some(p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
        match prev_home {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        assert_eq!(resolved.as_deref(), Some(bin.as_path()));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_claude_cli_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let _g = env_lock();
        let prev = std::env::var_os("PATH");
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("PATH", dir.path());
        // Drop HOME and point the fallback dirs at a tempdir-derived
        // path that doesn't exist, so the well-known-paths walk
        // can't find a real claude installed on the test host.
        std::env::set_var("HOME", dir.path().join("nonexistent-home"));
        let resolved = resolve_claude_cli("definitely-not-a-real-binary-xyz");
        match prev {
            Some(p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
        match prev_home {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        assert!(resolved.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn resolve_claude_cli_explicit_path_must_exist() {
        // Names with a path separator skip the PATH walk: only the
        // exact location is checked. Both branches verified here.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("claude");
        assert!(resolve_claude_cli(missing.to_str().unwrap()).is_none());

        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&missing, b"#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = std::fs::metadata(&missing).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&missing, perms).unwrap();
        assert_eq!(
            resolve_claude_cli(missing.to_str().unwrap()).as_deref(),
            Some(missing.as_path())
        );
    }

    #[test]
    fn resolve_claude_cli_empty_returns_none() {
        assert!(resolve_claude_cli("").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn resolve_gemini_cli_fallback_npm_global() {
        // gemini-cli ships through npm; the resolver's fallback walk
        // hits ~/.npm-global/bin when the user's $PATH doesn't carry
        // their npm prefix (typical for launchd-spawned chan).
        use std::os::unix::fs::PermissionsExt;
        let path_dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let npm_bin = home.path().join(".npm-global").join("bin");
        std::fs::create_dir_all(&npm_bin).unwrap();
        let bin = npm_bin.join("gemini");
        std::fs::write(&bin, b"#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();

        let _g = env_lock();
        let prev_path = std::env::var_os("PATH");
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("PATH", path_dir.path());
        std::env::set_var("HOME", home.path());
        let resolved = resolve_gemini_cli("gemini");
        match prev_path {
            Some(p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
        match prev_home {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        assert_eq!(resolved.as_deref(), Some(bin.as_path()));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_claude_cli_fallback_when_path_misses() {
        // PATH points only at an empty dir (claude not there); the
        // ~/.claude/local/bin fallback should still find the binary.
        // We override $HOME to point at a tempdir so the fallback's
        // `~/.claude/local/bin` resolves into a location we control.
        use std::os::unix::fs::PermissionsExt;
        let path_dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let claude_dir = home.path().join(".claude").join("local").join("bin");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let bin = claude_dir.join("claude");
        std::fs::write(&bin, b"#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();

        let _g = env_lock();
        let prev_path = std::env::var_os("PATH");
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("PATH", path_dir.path());
        std::env::set_var("HOME", home.path());
        let resolved = resolve_claude_cli("claude");
        match prev_path {
            Some(p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
        match prev_home {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        assert_eq!(resolved.as_deref(), Some(bin.as_path()));
    }
}
