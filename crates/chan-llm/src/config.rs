// LlmConfig: the cross-platform settings layer for the assistant.
//
// Persisted to `~/.config/chan/llm.toml` (mode 0600 on Unix, since
// the file may be the on-disk fallback for API keys when env vars
// and the OS keychain are unavailable). Apps that don't have a home
// dir (iOS, Android sandboxes) pass an explicit path via `load_from`
// / `save_to`.
//
// Only fields that are genuinely cross-platform live here:
//
//   - which backend is selected
//   - which model per backend
//   - the auto_apply_writes flag (whether the assistant's write
//     proposals hit disk without a per-call confirmation)
//   - per-backend API keys (when stored in the file fallback)
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
    /// Which backend the assistant uses by default. None = no
    /// assistant configured yet (the UI should show a "pick a
    /// backend" prompt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendKind>,
    /// Per-backend model override. Falls back to the backend's
    /// default model when unset.
    #[serde(default, skip_serializing_if = "Models::is_empty")]
    pub models: Models,
    /// Per-backend endpoint URL override. Today only Ollama
    /// surfaces a URL knob (cloud backends use fixed endpoints);
    /// shape is per-backend so adding self-hosted Anthropic-
    /// compatible gateways later is just a new field.
    #[serde(default, skip_serializing_if = "Urls::is_empty")]
    pub urls: Urls,
    /// Per-backend maximum output tokens. Falls back to the
    /// backend's default when unset (Anthropic 4096, Gemini 4096,
    /// Ollama uncapped). Use this when a model supports a higher
    /// ceiling (Claude Opus's long-form modes, Gemini 1M-context
    /// models) or to deliberately cap costs on a slow local model.
    /// claude_cli is omitted: claude picks its own ceiling.
    #[serde(default, skip_serializing_if = "MaxTokens::is_empty")]
    pub max_tokens: MaxTokens,
    /// When true, the assistant's `write_file` tool calls go to disk
    /// without a per-call confirmation. When false, the consumer
    /// (web frontend, native shell) must surface a confirmation UI
    /// for each write. Default: false. Hard line: never silently
    /// flip to true.
    #[serde(default)]
    pub auto_apply_writes: bool,
    /// Hard cap on a single MCP `read_image` response, in bytes.
    /// Mirrors the `MaxTokens` shape: `None` means "use the
    /// chan-llm default" (`mcp::DEFAULT_MCP_IMAGE_MAX_BYTES`,
    /// currently 10 MiB). Set this to widen for models that accept
    /// larger image attachments, or narrow it to keep tool results
    /// bounded on a metered network. The MCP server reads the file
    /// before checking the cap, so this also caps the worst-case
    /// memory the server allocates for a single image read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_image_max_bytes: Option<u64>,
    /// Per-backend API keys when stored in the on-disk fallback.
    /// Env vars and the OS keychain take precedence. Empty strings
    /// are treated as unset.
    #[serde(default, skip_serializing_if = "Keys::is_empty")]
    pub keys: Keys,
    /// Settings for the ClaudeCli backend (subprocess command,
    /// extra args). Empty for any other backend.
    #[serde(default, skip_serializing_if = "ClaudeCli::is_empty")]
    pub claude_cli: ClaudeCli,
    /// Settings for the GeminiCli backend (subprocess command,
    /// extra args). Empty for any other backend.
    #[serde(default, skip_serializing_if = "GeminiCli::is_empty")]
    pub gemini_cli: GeminiCli,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Urls {
    /// Override for the Ollama server URL. Falls back to the
    /// `OLLAMA_HOST` env var when unset, then the hardcoded
    /// `http://localhost:11434` default. Env wins over the file
    /// the same way it does for keys: a per-shell override should
    /// keep working even when a different URL is persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ollama: Option<String>,
}

impl Urls {
    fn is_empty(&self) -> bool {
        self.ollama.is_none()
    }
}

/// Per-backend output-token caps. Mirrors `Models`'s shape so the
/// "unset = backend default" rule reads consistently across
/// settings UIs. `for_backend` returns `None` when the user hasn't
/// pinned a value, and the resolver in `backends::build` falls
/// back to the per-backend default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaxTokens {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini: Option<u32>,
    /// Maps to Ollama's `options.num_predict`. -1 means "no cap"
    /// in Ollama wire-format; we don't surface that here, so set
    /// a positive number to opt into a cap on long local-model
    /// generations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ollama: Option<u32>,
}

impl MaxTokens {
    fn is_empty(&self) -> bool {
        self.anthropic.is_none() && self.gemini.is_none() && self.ollama.is_none()
    }

    pub fn for_backend(&self, kind: BackendKind) -> Option<u32> {
        match kind {
            BackendKind::Anthropic => self.anthropic,
            BackendKind::Gemini => self.gemini,
            BackendKind::Ollama => self.ollama,
            // The agentic CLIs pick their own ceiling.
            BackendKind::ClaudeCli | BackendKind::GeminiCli => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Models {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ollama: Option<String>,
    /// Override for the `--model` flag passed to the `claude` CLI.
    /// When unset, claude picks whichever model its own config
    /// selects (we don't impose chan-llm defaults on it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_cli: Option<String>,
    /// Override for the `--model` flag passed to the `gemini` CLI.
    /// Same "let the CLI pick" semantics as claude_cli.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini_cli: Option<String>,
}

impl Models {
    fn is_empty(&self) -> bool {
        self.anthropic.is_none()
            && self.gemini.is_none()
            && self.ollama.is_none()
            && self.claude_cli.is_none()
            && self.gemini_cli.is_none()
    }

    pub fn for_backend(&self, kind: BackendKind) -> Option<&str> {
        match kind {
            BackendKind::Anthropic => self.anthropic.as_deref(),
            BackendKind::Gemini => self.gemini.as_deref(),
            BackendKind::Ollama => self.ollama.as_deref(),
            BackendKind::ClaudeCli => self.claude_cli.as_deref(),
            BackendKind::GeminiCli => self.gemini_cli.as_deref(),
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keys {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini: Option<String>,
}

impl Keys {
    fn is_empty(&self) -> bool {
        self.anthropic.is_none() && self.gemini.is_none()
    }

    pub fn for_backend(&self, kind: BackendKind) -> Option<&str> {
        match kind {
            BackendKind::Anthropic => self.anthropic.as_deref(),
            BackendKind::Gemini => self.gemini.as_deref(),
            // Ollama is keyless (local server). The agentic CLIs
            // pull their own auth from the user's installed CLI
            // (claude via ~/.claude, gemini via ~/.gemini or env).
            BackendKind::Ollama | BackendKind::ClaudeCli | BackendKind::GeminiCli => None,
        }
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
        Ok(toml::from_str(&raw)?)
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
}

/// Default config path: `~/.chan/llm.toml` on desktop, co-located
/// under the platform sandbox dir on iOS / Android. Routed through
/// `chan_drive::paths::config_dir` so the on-disk layout matches
/// chan-drive's registry (`~/.chan/config.toml`). iOS / Android
/// callers pass an explicit path via `load_from` / `save_to`.
fn default_path() -> PathBuf {
    chan_drive::paths::config_dir().join("llm.toml")
}

/// Atomic write + 0600 perms on Unix. The file may hold API keys,
/// so set perms before the rename so there's no readable-by-others
/// window. Also fsyncs the parent dir after rename so the new
/// dirent survives a power loss; without that step ext4 / xfs /
/// APFS / btrfs can drop the rename even though the data was
/// sync'd. Mirrors `chan_drive::fs_ops::atomic_write`.
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
            backend: Some(BackendKind::Anthropic),
            models: Models {
                anthropic: Some("claude-opus-4-7".into()),
                ..Default::default()
            },
            urls: Urls::default(),
            auto_apply_writes: true,
            keys: Keys {
                anthropic: Some("sk-ant-...".into()),
                ..Default::default()
            },
            claude_cli: ClaudeCli::default(),
            gemini_cli: GeminiCli::default(),
            max_tokens: MaxTokens::default(),
            mcp_image_max_bytes: None,
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn ollama_url_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            backend: Some(BackendKind::Ollama),
            urls: Urls {
                ollama: Some("http://192.168.1.10:11434".into()),
            },
            ..Default::default()
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(
            loaded.urls.ollama.as_deref(),
            Some("http://192.168.1.10:11434")
        );
    }

    #[test]
    fn empty_urls_skipped_in_serialized_output() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig::default();
        cfg.save_to(&p).unwrap();
        // Default is empty; serializer should skip the [urls] table
        // entirely so a fresh chan install doesn't grow noise in
        // llm.toml.
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(!raw.contains("[urls]"), "got: {raw}");
        assert!(!raw.contains("ollama"), "got: {raw}");
    }

    #[test]
    fn claude_cli_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            backend: Some(BackendKind::ClaudeCli),
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
    fn max_tokens_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            backend: Some(BackendKind::Anthropic),
            max_tokens: MaxTokens {
                anthropic: Some(8192),
                gemini: Some(2048),
                ollama: None,
            },
            ..Default::default()
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(
            loaded.max_tokens.for_backend(BackendKind::Anthropic),
            Some(8192)
        );
        assert_eq!(
            loaded.max_tokens.for_backend(BackendKind::Gemini),
            Some(2048)
        );
        assert_eq!(loaded.max_tokens.for_backend(BackendKind::Ollama), None);
    }

    #[test]
    fn empty_max_tokens_skipped_in_serialized_output() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        LlmConfig::default().save_to(&p).unwrap();
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(!raw.contains("[max_tokens]"), "got: {raw}");
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
