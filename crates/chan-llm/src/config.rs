// LlmConfig: the cross-platform settings layer for the assistant.
//
// Persisted to `~/.config/chan/llm.toml` (mode 0600 on Unix). Apps
// that don't have a home dir (iOS, Android sandboxes) pass an
// explicit path via `load_from` / `save_to`.
//
// Only fields that are genuinely cross-platform live here:
//
//   - which backend is selected
//   - which model per backend
//   - the auto_apply_writes flag (whether the assistant's write
//     proposals hit disk without a per-call confirmation)
//   - optional PATH override used to find subprocess CLIs
//   - subprocess backend launch settings
//
// Editor preferences (font, theme, keyboard shortcuts) are NOT here.
// Those differ per platform and live in each app's native store
// (UserDefaults on iOS, SharedPreferences on Android, a TOML at the
// app level on web/CLI).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backends::BackendKind;
use crate::error::{LlmError, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Which backend the assistant uses by default. May be set even
    /// when the matching provider's `enabled` flag is false (the
    /// default is sticky across enable/disable toggles so user intent
    /// survives a "disable then re-enable" round-trip). Hosts should
    /// treat the assistant as configured and active only when
    /// `active_backend()` returns `Some`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendKind>,
    /// Per-agent enable flags. A provider's row in the SPA Settings
    /// UI toggles the matching field here; the resolver only honors
    /// `backend` when the corresponding flag is set. Decoupling this
    /// from `backend` lets the user keep tokens / URLs configured for
    /// multiple CLI agents concurrently and switch the default.
    #[serde(default, skip_serializing_if = "EnabledProviders::is_empty")]
    pub enabled: EnabledProviders,
    /// Per-backend model override. Falls back to the backend's
    /// default model when unset.
    #[serde(default, skip_serializing_if = "Models::is_empty")]
    pub models: Models,
    /// Optional PATH entries used to find agent CLI binaries. When
    /// unset, chan-llm searches the process `PATH`; in both cases it
    /// then appends conventional install directories for Linux,
    /// macOS, and Windows. Hosts like `chan` can populate this from
    /// their own settings instead of mutating the process env.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_path: Option<Vec<PathBuf>>,
    /// Allow CLI discovery to use executables located on risky
    /// remote/FUSE mount types. Default false: such candidates are
    /// rejected with a structured reason so hosts can tell the user
    /// exactly what happened. Upper layers may expose a `--force`
    /// or equivalent by setting this to true; discovery still returns
    /// a warning on the detection result.
    #[serde(default, skip_serializing_if = "is_false")]
    pub cli_allow_risky_mounts: bool,
    /// When true, the assistant's `write_file` tool calls go to disk
    /// without a per-call confirmation. When false, the consumer
    /// (web frontend, native shell) must surface a confirmation UI
    /// for each write. Default: false. Hard line: never silently
    /// flip to true.
    #[serde(default)]
    pub auto_apply_writes: bool,
    /// Hard cap on a single MCP `read_image` response, in bytes.
    /// `None` means "use the chan-llm default"
    /// (`mcp::DEFAULT_MCP_IMAGE_MAX_BYTES`,
    /// currently 10 MiB). Set this to widen for models that accept
    /// larger image attachments, or narrow it to keep tool results
    /// bounded on a metered network. The MCP server reads the file
    /// before checking the cap, so this also caps the worst-case
    /// memory the server allocates for a single image read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_image_max_bytes: Option<u64>,
    /// Settings for the ClaudeCli backend (subprocess command,
    /// extra args). Empty for any other backend.
    #[serde(default, skip_serializing_if = "ClaudeCli::is_empty")]
    pub claude_cli: ClaudeCli,
    /// Settings for the GeminiCli backend (subprocess command,
    /// extra args). Empty for any other backend.
    #[serde(default, skip_serializing_if = "GeminiCli::is_empty")]
    pub gemini_cli: GeminiCli,
    /// Settings for the CodexCli backend (subprocess command,
    /// extra args). Empty for any other backend.
    #[serde(default, skip_serializing_if = "CodexCli::is_empty")]
    pub codex_cli: CodexCli,
    /// Inactivity timeout (in seconds) between consecutive lines of
    /// streaming output from a subprocess backend (ClaudeCli,
    /// GeminiCli). `None` means "use the chan-llm default" (300
    /// seconds today). Set this lower on a fast local network to
    /// detect a wedged child sooner; raise it for slow remote
    /// inference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_inactivity_timeout_secs: Option<u32>,
    /// Hard cap on the number of tool-call rounds within a single
    /// `LlmSession::send`. `None` means "use the chan-llm default"
    /// (12 today). Raise this when the model legitimately needs
    /// more steps before answering (long agentic workflows over
    /// large drives); lower it to fail fast on runaway loops in a
    /// development environment. Zero is treated as one so a
    /// misconfigured value can't deadlock the orchestrator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tool_iterations: Option<u32>,
    /// Tighten the env filter applied to subprocess backends
    /// (`claude_cli`, `gemini_cli`, `codex_cli`). Default `false`:
    /// the loose prefix-based allowlist runs (every var matching
    /// `ANTHROPIC_` / `CLAUDE_` / `GEMINI_` / `GOOGLE_` / `CODEX_`
    /// / `OPENAI_` survives), suitable for an interactive shell
    /// where the user owns the parent env. Set `true` when chan-llm
    /// runs under a long-lived service host (chan-server, a future
    /// remote runner) whose parent env may carry tainted vars: a
    /// strict per-vendor name allowlist runs instead, so things like
    /// `ANTHROPIC_BEDROCK_BASE_URL` (could redirect to a hostile
    /// endpoint) or `GOOGLE_APPLICATION_CREDENTIALS` (path to a
    /// service-account JSON) no longer leak into the spawned CLI.
    /// The strict allowlist still forwards the primary credential
    /// names (`ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `OPENAI_API_KEY`,
    /// matching CLAUDE_CODE / CODEX OAuth tokens) so the CLIs can
    /// still authenticate from the shell.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hardened_subprocess_env: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

/// Per-provider enable flags. Default-false on every field so a fresh
/// install starts with zero providers active; the SPA's Settings UI
/// is the only place that flips them on.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnabledProviders {
    #[serde(default)]
    pub claude_cli: bool,
    #[serde(default)]
    pub gemini_cli: bool,
    #[serde(default)]
    pub codex_cli: bool,
}

impl EnabledProviders {
    fn is_empty(&self) -> bool {
        !(self.claude_cli || self.gemini_cli || self.codex_cli)
    }

    /// Whether the given backend is enabled. The resolver gates on
    /// this in addition to `LlmConfig::backend` being set so that a
    /// disabled provider never gets a request attempted against it.
    pub fn for_backend(&self, kind: BackendKind) -> bool {
        match kind {
            BackendKind::ClaudeCli => self.claude_cli,
            BackendKind::GeminiCli => self.gemini_cli,
            BackendKind::CodexCli => self.codex_cli,
        }
    }

    pub fn set_for_backend(&mut self, kind: BackendKind, value: bool) {
        match kind {
            BackendKind::ClaudeCli => self.claude_cli = value,
            BackendKind::GeminiCli => self.gemini_cli = value,
            BackendKind::CodexCli => self.codex_cli = value,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Models {
    /// Override for the `--model` flag passed to the `claude` CLI.
    /// When unset, claude picks whichever model its own config
    /// selects (we don't impose chan-llm defaults on it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_cli: Option<String>,
    /// Override for the `--model` flag passed to the `gemini` CLI.
    /// Same "let the CLI pick" semantics as claude_cli.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini_cli: Option<String>,
    /// Override for the `--model` flag passed to `codex exec`.
    /// When unset, codex picks whichever model its own config selects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_cli: Option<String>,
}

impl Models {
    fn is_empty(&self) -> bool {
        self.claude_cli.is_none() && self.gemini_cli.is_none() && self.codex_cli.is_none()
    }

    pub fn for_backend(&self, kind: BackendKind) -> Option<&str> {
        match kind {
            BackendKind::ClaudeCli => self.claude_cli.as_deref(),
            BackendKind::GeminiCli => self.gemini_cli.as_deref(),
            BackendKind::CodexCli => self.codex_cli.as_deref(),
        }
    }
}

/// Subprocess settings for the ClaudeCli backend. The default
/// `cmd` is `["claude"]` (resolved on PATH); set it to a fully
/// qualified path when claude is installed somewhere non-standard
/// or when wrapping a different agentic CLI that speaks the same
/// stream-json protocol.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeCli {
    /// Command + leading args used to launch claude. None falls
    /// back to `["claude"]`. Stored as a vec so users can wrap with
    /// `nix shell` / `flatpak run` / similar without quoting hell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<Vec<String>>,
    /// Extra args appended after chan-llm's own flags. Useful for
    /// forwarding things like `--add-dir` or claude permission
    /// flags that aren't covered by chan-llm's contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_args: Vec<String>,
    /// Host-injected MCP server command. When `Some`, the backend
    /// runs claude in v2 MCP-mediated mode: writes a temp
    /// `--mcp-config` pointing at this command, allowlists only
    /// chan-llm's MCP tools (plus claude's read-only tools), and
    /// drops `--permission-mode bypassPermissions`. The vector is
    /// the full argv (e.g. `["chan", "__mcp", "/path/to/drive"]`).
    /// Skipped from TOML so the host can re-inject the right
    /// binary path on every launch without stale paths leaking
    /// into config files.
    #[serde(skip)]
    pub mcp_command: Option<Vec<String>>,
}

impl ClaudeCli {
    fn is_empty(&self) -> bool {
        self.cmd.is_none() && self.extra_args.is_empty() && self.mcp_command.is_none()
    }
}

/// Subprocess settings for the GeminiCli backend. Mirrors `ClaudeCli`
/// (gemini-cli's headless contract is similar enough that the same
/// shape applies). The default `cmd` is `["gemini"]` resolved on PATH.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiCli {
    /// Command + leading args used to launch gemini. None falls back
    /// to `["gemini"]`. Same wrap-with-`nix shell` story as ClaudeCli.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<Vec<String>>,
    /// Extra args appended after chan-llm's own flags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_args: Vec<String>,
    /// Host-injected MCP server command. When `Some`, the backend
    /// runs gemini in v2 MCP-mediated mode: writes a temp
    /// `<home>/.gemini/settings.json` (with GEMINI_CLI_HOME pointed
    /// at that home) advertising chan-llm's MCP server, drops a
    /// `<home>/.gemini/policies/chan.toml` deny-policy for gemini's
    /// native edit/shell tools, and passes
    /// `--allowed-mcp-server-names chan`. Same skip-from-TOML reason
    /// as `ClaudeCli::mcp_command`: the host re-injects on every
    /// launch, never persisted.
    #[serde(skip)]
    pub mcp_command: Option<Vec<String>>,
}

impl GeminiCli {
    fn is_empty(&self) -> bool {
        self.cmd.is_none() && self.extra_args.is_empty() && self.mcp_command.is_none()
    }
}

/// Subprocess settings for the CodexCli backend. The default `cmd`
/// is `["codex"]` resolved on PATH. MCP wiring is injected with
/// per-run `-c mcp_servers.chan.*` overrides so chan never mutates
/// the user's `~/.codex/config.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexCli {
    /// Command + leading args used to launch codex. None falls back
    /// to `["codex"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<Vec<String>>,
    /// Extra args appended after chan-llm's own flags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_args: Vec<String>,
    /// Host-injected MCP server command. When `Some`, the backend
    /// runs `codex exec` with the chan MCP server configured for
    /// this invocation only. Skipped from TOML so hosts inject the
    /// correct binary path on every launch.
    #[serde(skip)]
    pub mcp_command: Option<Vec<String>>,
}

impl CodexCli {
    fn is_empty(&self) -> bool {
        self.cmd.is_none() && self.extra_args.is_empty() && self.mcp_command.is_none()
    }
}

impl LlmConfig {
    pub fn load() -> Result<Self> {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        let mut cfg: Self = toml::from_str(&raw)?;
        cfg.migrate_legacy_enabled();
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&default_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)?;
        atomic_write_strict(path, body.as_bytes())?;
        Ok(())
    }

    /// The backend the resolver should use right now: `backend` only
    /// when the matching provider's enable flag is true. Returns None
    /// when no default is selected or the selected default has been
    /// disabled in the SPA. Every chan-llm caller that previously
    /// branched on `backend.is_some()` should branch on this instead.
    pub fn active_backend(&self) -> Option<BackendKind> {
        let kind = self.backend?;
        self.enabled.for_backend(kind).then_some(kind)
    }

    /// Bridge from pre-`enabled` config files. The 1.0 schema treats
    /// `backend` as the user's sticky default pick and gates the
    /// active backend on a separate `enabled` table. Configs written
    /// before this split assumed the configured backend was implicitly
    /// active; on load we detect that shape (no `enabled` flags set
    /// and `backend` populated) and flip on the matching provider so
    /// upgraded users don't have to revisit Settings to keep working.
    fn migrate_legacy_enabled(&mut self) {
        if self.enabled.is_empty() {
            if let Some(kind) = self.backend {
                self.enabled.set_for_backend(kind, true);
            }
        }
    }
}

/// Default config path: `~/.chan/llm.toml` on desktop, co-located
/// under the platform sandbox dir on iOS / Android. Routed through
/// `chan_drive::paths::config_dir` so the on-disk layout matches
/// chan-drive's registry (`~/.chan/config.toml`). iOS / Android
/// callers pass an explicit path via `load_from` / `save_to`.
fn default_path() -> PathBuf {
    chan_drive::paths::config_dir().join("llm.toml")
}

/// Atomic write + 0600 perms on Unix. Set perms before the rename so
/// there's no readable-by-others window. Also fsyncs the parent dir
/// after rename so the new dirent survives a power loss; without
/// that step ext4 / xfs / APFS / btrfs can drop the rename even
/// though the data was sync'd. Mirrors
/// `chan_drive::fs_ops::atomic_write`.
fn atomic_write_strict(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| LlmError::Io("path has no parent".into()))?;
    std::fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600));
    }
    tmp.persist(path)
        .map_err(|e| LlmError::Io(e.error.to_string()))?;
    sync_dir(parent)?;
    Ok(())
}

/// fsync a directory so a fresh dirent inside it becomes durable.
/// Unix-only; Windows commits dirent changes through NTFS's journal
/// as part of the rename itself, so this is a no-op there.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<()> {
    let f = std::fs::File::open(dir)?;
    f.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig::default();
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn populated_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            backend: Some(BackendKind::ClaudeCli),
            enabled: EnabledProviders {
                claude_cli: true,
                ..Default::default()
            },
            cli_path: None,
            cli_allow_risky_mounts: false,
            models: Models {
                claude_cli: Some("opus".into()),
                ..Default::default()
            },
            auto_apply_writes: true,
            claude_cli: ClaudeCli::default(),
            gemini_cli: GeminiCli::default(),
            codex_cli: CodexCli::default(),
            mcp_image_max_bytes: None,
            stream_inactivity_timeout_secs: None,
            max_tool_iterations: None,
            hardened_subprocess_env: false,
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn active_backend_gates_on_enabled() {
        // Sticky default with no enable flag is "configured but
        // disabled": the resolver must refuse to launch the backend.
        let mut cfg = LlmConfig {
            backend: Some(BackendKind::ClaudeCli),
            ..Default::default()
        };
        assert_eq!(cfg.active_backend(), None);
        cfg.enabled.claude_cli = true;
        assert_eq!(cfg.active_backend(), Some(BackendKind::ClaudeCli));
        // Disable again: the default stays sticky, but active flips
        // back to None.
        cfg.enabled.claude_cli = false;
        assert_eq!(cfg.active_backend(), None);
        assert_eq!(cfg.backend, Some(BackendKind::ClaudeCli));
    }

    #[test]
    fn legacy_config_migrates_enabled_from_backend() {
        // Configs written before the per-provider `enabled` table
        // existed assumed the configured backend was implicitly
        // active. load_from must detect that shape and flip on the
        // matching flag so upgraded installs keep working without a
        // forced trip through Settings.
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        std::fs::write(
            &p,
            "backend = \"claude_cli\"\n[models]\nclaude_cli = \"opus\"\n",
        )
        .unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(loaded.backend, Some(BackendKind::ClaudeCli));
        assert!(loaded.enabled.claude_cli);
        assert_eq!(loaded.active_backend(), Some(BackendKind::ClaudeCli));
    }

    #[test]
    fn legacy_migration_skips_when_no_backend() {
        // A fresh config (no backend set) must NOT auto-enable
        // anything: migration only fires when there's a sticky pick
        // to honor.
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        std::fs::write(&p, "").unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert!(loaded.enabled.is_empty());
        assert_eq!(loaded.active_backend(), None);
    }

    #[test]
    fn max_tool_iterations_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            max_tool_iterations: Some(24),
            ..Default::default()
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(loaded.max_tool_iterations, Some(24));
    }

    #[test]
    fn unset_max_tool_iterations_skipped_in_serialized_output() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        LlmConfig::default().save_to(&p).unwrap();
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(!raw.contains("max_tool_iterations"), "got: {raw}");
    }

    #[test]
    fn cli_path_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            cli_path: Some(vec![PathBuf::from("/opt/chan/bin"), PathBuf::from("/x")]),
            ..Default::default()
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(loaded.cli_path, cfg.cli_path);
    }

    #[test]
    fn unset_cli_path_skipped_in_serialized_output() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        LlmConfig::default().save_to(&p).unwrap();
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(!raw.contains("cli_path"), "got: {raw}");
    }

    #[test]
    fn cli_allow_risky_mounts_round_trips_when_set() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            cli_allow_risky_mounts: true,
            ..Default::default()
        };
        cfg.save_to(&p).unwrap();
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(raw.contains("cli_allow_risky_mounts"), "got: {raw}");
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert!(loaded.cli_allow_risky_mounts);
    }

    #[test]
    fn claude_cli_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            backend: Some(BackendKind::ClaudeCli),
            enabled: EnabledProviders {
                claude_cli: true,
                ..Default::default()
            },
            models: Models {
                claude_cli: Some("claude-sonnet-4-6".into()),
                ..Default::default()
            },
            claude_cli: ClaudeCli {
                cmd: Some(vec!["/usr/local/bin/claude".into()]),
                extra_args: vec!["--add-dir".into(), "/extra".into()],
                mcp_command: None,
            },
            ..Default::default()
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn empty_claude_cli_skipped_in_serialized_output() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        LlmConfig::default().save_to(&p).unwrap();
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(!raw.contains("[claude_cli]"), "got: {raw}");
    }

    #[test]
    fn codex_cli_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            backend: Some(BackendKind::CodexCli),
            enabled: EnabledProviders {
                codex_cli: true,
                ..Default::default()
            },
            models: Models {
                codex_cli: Some("gpt-5.2-codex".into()),
                ..Default::default()
            },
            codex_cli: CodexCli {
                cmd: Some(vec!["/usr/local/bin/codex".into()]),
                extra_args: vec!["--ignore-rules".into()],
                mcp_command: None,
            },
            ..Default::default()
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn mcp_image_max_bytes_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            mcp_image_max_bytes: Some(20 * 1024 * 1024),
            ..Default::default()
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(loaded.mcp_image_max_bytes, Some(20 * 1024 * 1024));
    }

    #[test]
    fn unset_mcp_image_max_bytes_skipped_in_serialized_output() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        LlmConfig::default().save_to(&p).unwrap();
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(!raw.contains("mcp_image_max_bytes"), "got: {raw}");
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("nothing.toml");
        let cfg = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, LlmConfig::default());
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig::default();
        cfg.save_to(&p).unwrap();
        let mode = std::fs::metadata(&p).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "got {mode:o}");
    }
}
