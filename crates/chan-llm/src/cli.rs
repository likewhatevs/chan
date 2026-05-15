//! CLI agent discovery.
//!
//! Backends shell out to installed agent CLIs. Discovery resolves
//! the first argv element by checking a host-provided PATH override,
//! then the process PATH, then conventional install directories for
//! the current platform. A discovered executable is canonicalized and
//! validated before the resolved path is handed to subprocess spawn.
//! The resolved command still keeps any leading wrapper args the user
//! configured in `LlmConfig`.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backends::BackendKind;
use crate::config::LlmConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CliStatus {
    Present,
    NotFound,
    Rejected {
        reason: CliRejectReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliRejectReason {
    EmptyCommand,
    MetadataFailed,
    CanonicalizeFailed,
    NotRegularFile,
    NotExecutable,
    UnsafePermissions,
    WorldWritableDir,
    RiskyMount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWarning {
    pub reason: CliWarningReason,
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliWarningReason {
    RiskyMountAllowed,
}

/// Result of probing one CLI backend's command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliDetection {
    pub backend: BackendKind,
    /// Effective argv after resolving the executable. Empty only if
    /// the user explicitly configured an empty command vector.
    pub command: Vec<String>,
    /// Resolved executable path when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub status: CliStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<CliWarning>,
    /// Search directories considered, in order, after de-duplication.
    pub searched: Vec<PathBuf>,
}

impl CliDetection {
    pub fn present(&self) -> bool {
        matches!(self.status, CliStatus::Present)
    }

    pub fn rejection(&self) -> Option<(&CliRejectReason, Option<&Path>, &str)> {
        match &self.status {
            CliStatus::Rejected {
                reason,
                path,
                message,
            } => Some((reason, path.as_deref(), message.as_str())),
            _ => None,
        }
    }
}

/// Resolve the effective command for a backend. Keeps configured
/// leading args, replacing only argv[0] with a resolved path when
/// found.
pub(crate) fn resolve_backend_command(kind: BackendKind, config: &LlmConfig) -> CliDetection {
    let configured = match kind {
        BackendKind::ClaudeCli => config.claude_cli.cmd.clone(),
        BackendKind::GeminiCli => config.gemini_cli.cmd.clone(),
        BackendKind::CodexCli => config.codex_cli.cmd.clone(),
    };
    detect_command(
        kind,
        configured,
        default_binary(kind),
        CliPolicy::from_config(config),
    )
}

/// Public probe for settings UIs and host-side preflight checks.
pub fn detect_backend_cli(kind: BackendKind, config: &LlmConfig) -> CliDetection {
    resolve_backend_command(kind, config)
}

/// Probe every supported CLI backend.
pub fn detect_all(config: &LlmConfig) -> Vec<CliDetection> {
    [
        BackendKind::ClaudeCli,
        BackendKind::GeminiCli,
        BackendKind::CodexCli,
    ]
    .into_iter()
    .map(|kind| detect_backend_cli(kind, config))
    .collect()
}

fn detect_command(
    kind: BackendKind,
    configured: Option<Vec<String>>,
    fallback_bin: &str,
    policy: CliPolicy<'_>,
) -> CliDetection {
    let mut command = configured.unwrap_or_else(|| vec![fallback_bin.to_string()]);
    if command.is_empty() {
        return CliDetection {
            backend: kind,
            command,
            path: None,
            status: CliStatus::Rejected {
                reason: CliRejectReason::EmptyCommand,
                path: None,
                message: "configured CLI command is empty".to_string(),
            },
            warnings: Vec::new(),
            searched: search_dirs(policy.host_path),
        };
    }

    let searched = search_dirs(policy.host_path);
    match resolve_executable(&command[0], &searched, policy) {
        ProbeResult::Present { path, warnings } => {
            command[0] = path.to_string_lossy().into_owned();
            CliDetection {
                backend: kind,
                command,
                path: Some(path),
                status: CliStatus::Present,
                warnings,
                searched,
            }
        }
        ProbeResult::Missing => CliDetection {
            backend: kind,
            command,
            path: None,
            status: CliStatus::NotFound,
            warnings: Vec::new(),
            searched,
        },
        ProbeResult::Rejected {
            reason,
            path,
            message,
        } => CliDetection {
            backend: kind,
            command,
            path: None,
            status: CliStatus::Rejected {
                reason,
                path,
                message,
            },
            warnings: Vec::new(),
            searched,
        },
    }
}

fn default_binary(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::ClaudeCli => "claude",
        BackendKind::GeminiCli => "gemini",
        BackendKind::CodexCli => "codex",
    }
}

#[derive(Debug, Clone, Copy)]
struct CliPolicy<'a> {
    host_path: Option<&'a [PathBuf]>,
    allow_risky_mounts: bool,
    mounts: Option<&'a [MountInfo]>,
}

impl<'a> CliPolicy<'a> {
    fn from_config(config: &'a LlmConfig) -> Self {
        Self {
            host_path: config.cli_path.as_deref(),
            allow_risky_mounts: config.cli_allow_risky_mounts,
            mounts: None,
        }
    }
}

#[derive(Debug)]
enum ProbeResult {
    Present {
        path: PathBuf,
        warnings: Vec<CliWarning>,
    },
    Missing,
    Rejected {
        reason: CliRejectReason,
        path: Option<PathBuf>,
        message: String,
    },
}

fn resolve_executable(cmd: &str, dirs: &[PathBuf], policy: CliPolicy<'_>) -> ProbeResult {
    let cmd_path = Path::new(cmd);
    if has_path_separator(cmd) {
        return executable_candidate(cmd_path, policy);
    }

    for dir in dirs {
        for candidate in executable_names(cmd) {
            let path = dir.join(candidate);
            match executable_candidate(&path, policy) {
                ProbeResult::Missing => {}
                other => return other,
            }
        }
    }
    ProbeResult::Missing
}

fn has_path_separator(s: &str) -> bool {
    s.contains('/') || s.contains('\\')
}

fn executable_candidate(path: &Path, policy: CliPolicy<'_>) -> ProbeResult {
    // First touch the exact candidate without following the final
    // component. This rules out missing paths and gives us a bounded
    // place to add final-component policy without recursively
    // walking a tree. We still canonicalize below so a normal
    // package-manager symlink resolves to the concrete file we exec.
    let _candidate_meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return ProbeResult::Missing,
        Err(e) => {
            return ProbeResult::Rejected {
                reason: CliRejectReason::MetadataFailed,
                path: Some(path.to_path_buf()),
                message: format!("could not stat CLI candidate: {e}"),
            };
        }
    };
    let path = match std::fs::canonicalize(path) {
        Ok(path) => path,
        Err(e) => {
            return ProbeResult::Rejected {
                reason: CliRejectReason::CanonicalizeFailed,
                path: Some(path.to_path_buf()),
                message: format!("could not canonicalize CLI candidate: {e}"),
            };
        }
    };
    let meta = match std::fs::metadata(&path) {
        Ok(meta) => meta,
        Err(e) => {
            return ProbeResult::Rejected {
                reason: CliRejectReason::MetadataFailed,
                path: Some(path),
                message: format!("could not stat canonical CLI candidate: {e}"),
            };
        }
    };
    if !meta.is_file() {
        return ProbeResult::Rejected {
            reason: CliRejectReason::NotRegularFile,
            path: Some(path),
            message: "CLI candidate is not a regular file".to_string(),
        };
    }
    if let Some((reason, message)) = unsafe_executable_reason(&path, &meta) {
        return ProbeResult::Rejected {
            reason,
            path: Some(path),
            message,
        };
    }
    let mut warnings = Vec::new();
    if let Some(mount) = risky_mount_for_path(&path, policy) {
        let message = format!(
            "CLI candidate is on risky mount type {} at {}",
            mount.fs_type,
            mount.mount_point.display()
        );
        if !policy.allow_risky_mounts {
            return ProbeResult::Rejected {
                reason: CliRejectReason::RiskyMount,
                path: Some(path),
                message,
            };
        }
        warnings.push(CliWarning {
            reason: CliWarningReason::RiskyMountAllowed,
            path: path.clone(),
            message,
        });
    }
    ProbeResult::Present { path, warnings }
}

#[cfg(unix)]
fn unsafe_executable_reason(
    path: &Path,
    meta: &std::fs::Metadata,
) -> Option<(CliRejectReason, String)> {
    use std::os::unix::fs::PermissionsExt;
    let mode = meta.permissions().mode();
    if mode & 0o111 == 0 {
        return Some((
            CliRejectReason::NotExecutable,
            "CLI candidate has no executable permission bits".to_string(),
        ));
    }
    // A CLI binary writable by group/other is too easy to swap under
    // a long-lived app process after discovery.
    if mode & 0o022 != 0 {
        return Some((
            CliRejectReason::UnsafePermissions,
            "CLI candidate is writable by group or other".to_string(),
        ));
    }
    let Some(parent) = path.parent() else {
        return Some((
            CliRejectReason::MetadataFailed,
            "CLI candidate has no parent directory".to_string(),
        ));
    };
    let Ok(parent_meta) = std::fs::metadata(parent) else {
        return Some((
            CliRejectReason::MetadataFailed,
            "could not stat CLI candidate parent directory".to_string(),
        ));
    };
    if !parent_meta.is_dir() {
        return Some((
            CliRejectReason::MetadataFailed,
            "CLI candidate parent is not a directory".to_string(),
        ));
    }
    // Reject immediately world-writable containing directories. We
    // intentionally allow group-writable dirs because Homebrew on
    // macOS commonly uses group-writable admin-owned prefixes.
    if parent_meta.permissions().mode() & 0o002 != 0 {
        return Some((
            CliRejectReason::WorldWritableDir,
            "CLI candidate parent directory is world-writable".to_string(),
        ));
    }
    None
}

#[cfg(not(unix))]
fn unsafe_executable_reason(
    _path: &Path,
    _meta: &std::fs::Metadata,
) -> Option<(CliRejectReason, String)> {
    None
}

#[cfg(test)]
fn command_path_env_entries(config: &LlmConfig) -> Vec<PathBuf> {
    command_path_env(config)
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default()
}

pub(crate) fn command_path_env(config: &LlmConfig) -> Option<OsString> {
    join_path(&search_dirs(config.cli_path.as_deref()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MountInfo {
    mount_point: PathBuf,
    fs_type: String,
}

fn risky_mount_for_path(path: &Path, policy: CliPolicy<'_>) -> Option<MountInfo> {
    if let Some(mounts) = policy.mounts {
        risky_mount_for_path_with(path, mounts)
    } else {
        risky_mount_for_path_with(path, &current_mounts())
    }
}

fn risky_mount_for_path_with(path: &Path, mounts: &[MountInfo]) -> Option<MountInfo> {
    mounts
        .iter()
        .filter(|mount| path.starts_with(&mount.mount_point) && is_risky_fs_type(&mount.fs_type))
        .max_by_key(|mount| mount.mount_point.components().count())
        .cloned()
}

fn is_risky_fs_type(fs_type: &str) -> bool {
    let fs = fs_type.to_ascii_lowercase();
    fs == "fuse"
        || fs.starts_with("fuse.")
        || fs == "nfs"
        || fs == "nfs4"
        || fs == "cifs"
        || fs == "smb3"
        || fs == "smbfs"
        || fs == "sshfs"
}

#[cfg(target_os = "linux")]
fn current_mounts() -> Vec<MountInfo> {
    std::fs::read_to_string("/proc/self/mountinfo")
        .map(|raw| parse_linux_mountinfo(&raw))
        .unwrap_or_default()
}

#[cfg(not(target_os = "linux"))]
fn current_mounts() -> Vec<MountInfo> {
    Vec::new()
}

#[cfg(target_os = "linux")]
fn parse_linux_mountinfo(raw: &str) -> Vec<MountInfo> {
    raw.lines()
        .filter_map(|line| {
            let (pre, post) = line.split_once(" - ")?;
            let mut pre_fields = pre.split_whitespace();
            let mount_point = pre_fields.nth(4)?;
            let fs_type = post.split_whitespace().next()?;
            Some(MountInfo {
                mount_point: PathBuf::from(unescape_mountinfo_path(mount_point)),
                fs_type: fs_type.to_string(),
            })
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn unescape_mountinfo_path(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let digits: String = chars.by_ref().take(3).collect();
        if digits.len() == 3 {
            if let Ok(byte) = u8::from_str_radix(&digits, 8) {
                out.push(byte as char);
                continue;
            }
        }
        out.push('\\');
        out.push_str(&digits);
    }
    out
}

fn executable_names(cmd: &str) -> Vec<OsString> {
    let base = OsString::from(cmd);
    #[cfg(not(windows))]
    {
        vec![base]
    }
    #[cfg(windows)]
    {
        let path = Path::new(cmd);
        if path.extension().is_some() {
            return vec![base];
        }
        let mut out = Vec::new();
        let pathext =
            std::env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
        for ext in pathext.to_string_lossy().split(';') {
            if ext.is_empty() {
                continue;
            }
            let mut name = OsString::from(cmd);
            name.push(ext);
            out.push(name);
        }
        out.push(base);
        out
    }
}

fn search_dirs(host_path: Option<&[PathBuf]>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    if let Some(paths) = host_path {
        for p in paths {
            push_unique(&mut out, &mut seen, p.clone());
        }
    } else if let Some(path) = std::env::var_os("PATH") {
        for p in std::env::split_paths(&path) {
            push_unique(&mut out, &mut seen, p);
        }
    }

    for p in conventional_dirs() {
        push_unique(&mut out, &mut seen, p);
    }
    out
}

fn push_unique(out: &mut Vec<PathBuf>, seen: &mut HashSet<OsString>, path: PathBuf) {
    if path.as_os_str().is_empty() {
        return;
    }
    if !path.is_absolute() {
        return;
    }
    let key = path.as_os_str().to_os_string();
    if seen.insert(key) {
        out.push(path);
    }
}

fn conventional_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(unix)]
    {
        dirs.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/opt/homebrew/sbin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/local/sbin"),
            PathBuf::from("/opt/local/bin"),
            PathBuf::from("/opt/local/sbin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/sbin"),
        ]);
    }

    if let Some(home) = dirs::home_dir() {
        dirs.extend(home_relative_dirs(&home));
    }

    #[cfg(windows)]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local = PathBuf::from(local);
            dirs.push(local.join("Programs"));
            dirs.push(local.join("Microsoft").join("WindowsApps"));
            dirs.push(local.join("pnpm"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            dirs.push(PathBuf::from(appdata).join("npm"));
        }
        dirs.push(PathBuf::from(r"C:\Program Files\nodejs"));
        dirs.push(PathBuf::from(r"C:\Program Files\Git\usr\bin"));
    }

    dirs
}

fn home_relative_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        home.join("bin"),
        home.join(".local").join("bin"),
        home.join(".cargo").join("bin"),
        home.join(".bun").join("bin"),
        home.join(".npm-global").join("bin"),
        home.join(".volta").join("bin"),
        home.join(".deno").join("bin"),
        home.join(".nix-profile").join("bin"),
    ];

    #[cfg(target_os = "macos")]
    {
        dirs.push(home.join("Library").join("pnpm"));
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("pnpm"),
        );
    }

    #[cfg(windows)]
    {
        dirs.push(home.join("scoop").join("shims"));
        dirs.push(home.join(".local").join("bin"));
    }

    dirs
}

/// Join path entries for setting a child PATH.
pub(crate) fn join_path(paths: &[PathBuf]) -> Option<OsString> {
    std::env::join_paths(paths.iter().map(PathBuf::as_path)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, "#!/bin/sh\n").unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(path: &Path) {
        std::fs::write(path, "").unwrap();
    }

    #[test]
    fn host_path_wins_before_conventional_dirs() {
        let tmp = TempDir::new().unwrap();
        let bin = tmp.path().join(if cfg!(windows) {
            "claude.cmd"
        } else {
            "claude"
        });
        make_executable(&bin);
        let cfg = LlmConfig {
            cli_path: Some(vec![tmp.path().to_path_buf()]),
            ..Default::default()
        };
        let got = detect_backend_cli(BackendKind::ClaudeCli, &cfg);
        let expected = bin.canonicalize().unwrap();
        assert_eq!(got.path.as_deref(), Some(expected.as_path()));
        assert_eq!(got.command[0], expected.to_string_lossy());
    }

    #[test]
    fn configured_wrapper_keeps_args_and_resolves_first_argv() {
        let tmp = TempDir::new().unwrap();
        let wrapper = tmp
            .path()
            .join(if cfg!(windows) { "nix.cmd" } else { "nix" });
        make_executable(&wrapper);
        let cfg = LlmConfig {
            cli_path: Some(vec![tmp.path().to_path_buf()]),
            claude_cli: crate::config::ClaudeCli {
                cmd: Some(vec![
                    "nix".into(),
                    "shell".into(),
                    "-c".into(),
                    "claude".into(),
                ]),
                ..Default::default()
            },
            ..Default::default()
        };
        let got = detect_backend_cli(BackendKind::ClaudeCli, &cfg);
        let expected = wrapper.canonicalize().unwrap();
        assert_eq!(got.command[0], expected.to_string_lossy());
        assert_eq!(&got.command[1..], ["shell", "-c", "claude"]);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_cli_resolves_to_canonical_target_before_exec() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("real-claude");
        make_executable(&target);
        let link = tmp.path().join("claude");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let cfg = LlmConfig {
            cli_path: Some(vec![tmp.path().to_path_buf()]),
            ..Default::default()
        };
        let got = detect_backend_cli(BackendKind::ClaudeCli, &cfg);
        let expected = target.canonicalize().unwrap();
        assert_eq!(got.path.as_deref(), Some(expected.as_path()));
        assert_eq!(got.command[0], expected.to_string_lossy());
    }

    #[cfg(unix)]
    #[test]
    fn writable_cli_file_is_rejected() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let bin = tmp.path().join("claude-bad");
        make_executable(&bin);
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o775)).unwrap();
        let cfg = LlmConfig {
            cli_path: Some(vec![tmp.path().to_path_buf()]),
            claude_cli: crate::config::ClaudeCli {
                cmd: Some(vec!["claude-bad".into()]),
                ..Default::default()
            },
            ..Default::default()
        };
        let got = detect_backend_cli(BackendKind::ClaudeCli, &cfg);
        assert!(!got.present());
        assert!(matches!(
            got.rejection(),
            Some((CliRejectReason::UnsafePermissions, _, _))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn world_writable_containing_dir_is_rejected() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let bin = tmp.path().join("claude-bad");
        make_executable(&bin);
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
        let cfg = LlmConfig {
            cli_path: Some(vec![tmp.path().to_path_buf()]),
            claude_cli: crate::config::ClaudeCli {
                cmd: Some(vec!["claude-bad".into()]),
                ..Default::default()
            },
            ..Default::default()
        };
        let got = detect_backend_cli(BackendKind::ClaudeCli, &cfg);
        assert!(!got.present());
        assert!(matches!(
            got.rejection(),
            Some((CliRejectReason::WorldWritableDir, _, _))
        ));
    }

    #[test]
    fn risky_mount_is_rejected_with_structured_reason() {
        let tmp = TempDir::new().unwrap();
        let bin = tmp.path().join(if cfg!(windows) {
            "claude.cmd"
        } else {
            "claude"
        });
        make_executable(&bin);
        let host_path = vec![tmp.path().to_path_buf()];
        let mount_point = tmp.path().canonicalize().unwrap();
        let mounts = vec![MountInfo {
            mount_point,
            fs_type: "fuse.sshfs".to_string(),
        }];
        let got = detect_command(
            BackendKind::ClaudeCli,
            None,
            "claude",
            CliPolicy {
                host_path: Some(&host_path),
                allow_risky_mounts: false,
                mounts: Some(&mounts),
            },
        );
        assert!(!got.present());
        assert!(matches!(
            got.rejection(),
            Some((CliRejectReason::RiskyMount, _, msg)) if msg.contains("fuse.sshfs")
        ));
    }

    #[test]
    fn force_allows_risky_mount_with_warning() {
        let tmp = TempDir::new().unwrap();
        let bin = tmp.path().join(if cfg!(windows) {
            "claude.cmd"
        } else {
            "claude"
        });
        make_executable(&bin);
        let host_path = vec![tmp.path().to_path_buf()];
        let mount_point = tmp.path().canonicalize().unwrap();
        let mounts = vec![MountInfo {
            mount_point,
            fs_type: "fuse.sshfs".to_string(),
        }];
        let got = detect_command(
            BackendKind::ClaudeCli,
            None,
            "claude",
            CliPolicy {
                host_path: Some(&host_path),
                allow_risky_mounts: true,
                mounts: Some(&mounts),
            },
        );
        assert!(got.present());
        assert!(matches!(
            got.warnings.as_slice(),
            [CliWarning {
                reason: CliWarningReason::RiskyMountAllowed,
                ..
            }]
        ));
    }

    #[test]
    fn risky_mount_detection_picks_deepest_mount() {
        let mounts = vec![
            MountInfo {
                mount_point: PathBuf::from("/mnt"),
                fs_type: "ext4".to_string(),
            },
            MountInfo {
                mount_point: PathBuf::from("/mnt/ssh"),
                fs_type: "fuse.sshfs".to_string(),
            },
        ];
        let got = risky_mount_for_path_with(Path::new("/mnt/ssh/bin/claude"), &mounts).unwrap();
        assert_eq!(got.fs_type, "fuse.sshfs");
        assert_eq!(got.mount_point, PathBuf::from("/mnt/ssh"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_linux_mountinfo_and_unescapes_paths() {
        let raw = "36 25 0:32 / / rw,relatime - ext4 /dev/root rw\n\
                   44 36 0:47 / /mnt/ssh\\040drive rw,nosuid - fuse.sshfs sshfs rw\n";
        let mounts = parse_linux_mountinfo(raw);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[1].mount_point, PathBuf::from("/mnt/ssh drive"));
        assert_eq!(mounts[1].fs_type, "fuse.sshfs");
    }

    #[test]
    fn empty_configured_command_is_not_present() {
        let cfg = LlmConfig {
            codex_cli: crate::config::CodexCli {
                cmd: Some(Vec::new()),
                ..Default::default()
            },
            ..Default::default()
        };
        let got = detect_backend_cli(BackendKind::CodexCli, &cfg);
        assert!(!got.present());
        assert!(got.command.is_empty());
    }

    #[test]
    fn conventional_dirs_are_appended_to_host_path() {
        let tmp = TempDir::new().unwrap();
        let cfg = LlmConfig {
            cli_path: Some(vec![tmp.path().to_path_buf()]),
            ..Default::default()
        };
        let got = detect_backend_cli(BackendKind::GeminiCli, &cfg);
        assert_eq!(got.searched.first(), Some(&tmp.path().to_path_buf()));
        assert!(got.searched.len() > 1);
    }

    #[test]
    fn relative_path_entries_are_ignored() {
        let cfg = LlmConfig {
            cli_path: Some(vec![PathBuf::from("."), PathBuf::from("bin")]),
            ..Default::default()
        };
        let got = detect_backend_cli(BackendKind::GeminiCli, &cfg);
        assert!(!got.searched.iter().any(|p| p == Path::new(".")));
        assert!(!got.searched.iter().any(|p| p == Path::new("bin")));
    }

    #[test]
    fn child_path_uses_host_path_plus_conventional_dirs() {
        let tmp = TempDir::new().unwrap();
        let cfg = LlmConfig {
            cli_path: Some(vec![tmp.path().to_path_buf()]),
            ..Default::default()
        };
        let entries = command_path_env_entries(&cfg);
        assert_eq!(entries.first(), Some(&tmp.path().to_path_buf()));
        assert!(entries.len() > 1);
    }

    #[test]
    fn join_path_round_trips_multiple_entries() {
        let paths = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        let joined = join_path(&paths).unwrap();
        let split: Vec<_> = std::env::split_paths(&joined).collect();
        assert_eq!(split, paths);
    }
}
