//! Server preferences: per-server config (`/api/server/config`),
//! and the unified GlobalConfig view (`/api/config`).
//!
//! The unified surface joins three backing stores:
//!
//!   - EditorPrefs (preferences.toml) for fonts / theme / pane widths
//!     / line spacing / date format
//!   - ServerConfig (server.toml) for attachments_dir / answers_dir
//!   - LlmConfig (llm.toml) for the assistant subtree
//!
//! plus the chan-drive registry for `default_drive_root` and the
//! known-drives list.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_llm::BackendKind;
use serde::{Deserialize, Serialize};

use crate::error::{err, err_settings_locked, Error};
use crate::state::AppState;
use crate::{FontPrefs, LineSpacing, PaneWidths, ThemeChoice};

/// Unified Preferences shape returned over /api/drive and
/// /api/config. The fields are owned by three different stores:
///
/// - fonts / theme / pane_widths / line_spacing / date_format:
///   EditorPrefs (preferences.toml)
/// - attachments_dir: ServerConfig (server.toml; the answers_dir
///   field there is mirrored into the assistant subtree below)
/// - assistant: LlmConfig (llm.toml) + ServerConfig.answers_dir
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesView {
    pub fonts: FontPrefs,
    pub assistant: AssistantPrefsView,
    pub attachments_dir: String,
    pub theme: ThemeChoice,
    pub pane_widths: PaneWidths,
    pub line_spacing: LineSpacing,
    pub date_format: String,
}

/// Frontend's `AssistantPrefs` view. The subtables (claude / ollama /
/// gemini) carry only model overrides today; per-backend ollama URL
/// is stubbed out (Some(None)) since chan-llm doesn't persist it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantPrefsView {
    pub enabled: bool,
    pub backend: AssistantBackendKind,
    pub answers_dir: String,
    pub auto_apply_writes: bool,
    pub claude: ProviderPrefsView,
    pub ollama: OllamaPrefsView,
    pub gemini: ProviderPrefsView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPrefsView {
    #[serde(default)]
    pub model: Option<String>,
    /// Per-backend max output tokens. None falls back to chan-llm's
    /// per-backend default (Anthropic 4096, Gemini 4096). claude_cli
    /// has no counterpart in chan-llm and ignores this field.
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OllamaPrefsView {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// Maps to Ollama's `options.num_predict`. None = uncapped.
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Frontend uses "claude" (display label) for what chan-llm types
/// internally as `BackendKind::Anthropic`. The "claude_cli" /
/// "gemini_cli" variants cover the shell-executor backends that
/// wrap the local `claude` and `gemini` CLIs. The "embedded" variant
/// is reserved for a future on-device backend (qwen2.5 via candle);
/// it has no chan-llm counterpart yet, so PATCHing it is treated as
/// a no-op when read back the value falls through to the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantBackendKind {
    Claude,
    Ollama,
    Gemini,
    ClaudeCli,
    GeminiCli,
    Embedded,
}

impl AssistantBackendKind {
    pub fn from_chan_llm(kind: BackendKind) -> Self {
        match kind {
            BackendKind::Anthropic => AssistantBackendKind::Claude,
            BackendKind::Ollama => AssistantBackendKind::Ollama,
            BackendKind::Gemini => AssistantBackendKind::Gemini,
            BackendKind::ClaudeCli => AssistantBackendKind::ClaudeCli,
            BackendKind::GeminiCli => AssistantBackendKind::GeminiCli,
        }
    }

    pub fn to_chan_llm(self) -> Option<BackendKind> {
        match self {
            AssistantBackendKind::Claude => Some(BackendKind::Anthropic),
            AssistantBackendKind::Ollama => Some(BackendKind::Ollama),
            AssistantBackendKind::Gemini => Some(BackendKind::Gemini),
            AssistantBackendKind::ClaudeCli => Some(BackendKind::ClaudeCli),
            AssistantBackendKind::GeminiCli => Some(BackendKind::GeminiCli),
            AssistantBackendKind::Embedded => None,
        }
    }
}

/// Build the unified Preferences view for the current state. Reads
/// each backing store under its own lock.
pub(super) fn preferences_view(state: &AppState) -> PreferencesView {
    let editor = state.editor_prefs.lock().expect("editor prefs poisoned");
    let server = state.server_config.lock().expect("server config poisoned");
    let llm = state.llm_config.lock().expect("llm config poisoned");
    let backend_kind = llm.backend.unwrap_or(BackendKind::Anthropic);
    PreferencesView {
        fonts: editor.fonts.clone(),
        assistant: AssistantPrefsView {
            enabled: llm.backend.is_some(),
            backend: AssistantBackendKind::from_chan_llm(backend_kind),
            answers_dir: server.answers_dir.clone(),
            auto_apply_writes: llm.auto_apply_writes,
            claude: ProviderPrefsView {
                model: llm.models.anthropic.clone(),
                max_tokens: llm.max_tokens.anthropic,
            },
            ollama: OllamaPrefsView {
                url: llm.urls.ollama.clone(),
                model: llm.models.ollama.clone(),
                max_tokens: llm.max_tokens.ollama,
            },
            gemini: ProviderPrefsView {
                model: llm.models.gemini.clone(),
                max_tokens: llm.max_tokens.gemini,
            },
        },
        attachments_dir: server.attachments_dir.clone(),
        theme: editor.theme,
        pane_widths: editor.pane_widths,
        line_spacing: editor.line_spacing,
        date_format: editor.date_format.clone(),
    }
}

// ----- /api/server/config ------------------------------------------------
//
// Holds chan-server-specific paths and toggles that aren't user
// content (those live in the drive) and aren't LLM-shaped (those
// live in chan-llm). The split:
//
//   /api/drive             chan-drive registry: name, root
//   /api/llm/status        chan-llm config: backend, model, keys
//   /api/server/config     this: attachments_dir, answers_dir

pub async fn api_get_server_config(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.server_config.lock().unwrap().clone();
    Json(cfg).into_response()
}

#[derive(Deserialize)]
pub struct PatchServerConfigBody {
    /// Drive-relative POSIX path. Empty string is rejected
    /// because the path is used as a prefix; an empty prefix
    /// would land attachments in the drive root, surprising
    /// the user.
    #[serde(default)]
    attachments_dir: Option<String>,
    #[serde(default)]
    answers_dir: Option<String>,
}

pub async fn api_patch_server_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PatchServerConfigBody>,
) -> Response {
    if state.settings_disabled {
        return err_settings_locked();
    }
    let mut cfg = state.server_config.lock().unwrap();
    if let Some(p) = body.attachments_dir {
        if p.is_empty() {
            return err(
                StatusCode::BAD_REQUEST,
                "attachments_dir must be non-empty".into(),
            );
        }
        cfg.attachments_dir = p;
    }
    if let Some(p) = body.answers_dir {
        if p.is_empty() {
            return err(
                StatusCode::BAD_REQUEST,
                "answers_dir must be non-empty".into(),
            );
        }
        cfg.answers_dir = p;
    }
    if let Err(e) = cfg.save() {
        return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    Json(cfg.clone()).into_response()
}

// ----- /api/config (unified GlobalConfig) --------------------------------
//
// Frontend treats Settings as a single round-trip surface: GET the
// whole GlobalConfig (preferences + drives + default_drive_root),
// PATCH the same shape on save. We assemble the view from three
// underlying stores (EditorPrefs, ServerConfig, LlmConfig) plus the
// chan-drive registry and route the writes back the same way.

#[derive(Serialize)]
struct GlobalConfigView {
    preferences: PreferencesView,
    /// Empty string serializes as None (the resolver falls back to
    /// the platform default).
    default_drive_root: Option<String>,
    drives: Vec<KnownDriveView>,
}

#[derive(Serialize)]
struct KnownDriveView {
    path: String,
    name: Option<String>,
    /// RFC3339 timestamp.
    last_opened: String,
}

#[derive(Deserialize)]
pub struct PatchConfigBody {
    /// Whole-block replacement. Frontend sends the entire
    /// GlobalConfig on every save.
    #[serde(default)]
    preferences: Option<PreferencesView>,
    #[serde(default)]
    default_drive_root: Option<Option<String>>,
    /// Read-only on PATCH: drives are managed via /api/drive PATCH
    /// (rename) and the CLI (`chan add` / `remove`). Frontend sends
    /// the field for round-tripping; we just ignore it.
    #[serde(default)]
    #[allow(dead_code)]
    drives: Option<serde_json::Value>,
}

fn global_config_view(state: &AppState) -> GlobalConfigView {
    let drives = state
        .library
        .list_drives()
        .into_iter()
        .map(|d| KnownDriveView {
            path: d.path.to_string_lossy().into_owned(),
            name: d.name,
            last_opened: d.last_opened.to_rfc3339(),
        })
        .collect();
    GlobalConfigView {
        preferences: preferences_view(state),
        default_drive_root: state
            .library
            .default_drive_root()
            .map(|p| p.to_string_lossy().into_owned()),
        drives,
    }
}

pub async fn api_get_config(State(state): State<Arc<AppState>>) -> Response {
    Json(global_config_view(&state)).into_response()
}

pub async fn api_patch_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PatchConfigBody>,
) -> Response {
    if state.settings_disabled {
        return err_settings_locked();
    }
    if let Some(prefs) = body.preferences {
        if let Err(e) = apply_preferences(&state, prefs) {
            return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    }
    if let Some(opt) = body.default_drive_root {
        let trimmed = opt.as_ref().map(|s| s.trim().to_string());
        let value = match trimmed {
            Some(s) if s.is_empty() => None,
            other => other,
        };
        if let Err(e) = state
            .library
            .set_default_drive_root(value.map(std::path::PathBuf::from))
        {
            return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    }
    Json(global_config_view(&state)).into_response()
}

/// Split the unified Preferences body across the three backing
/// stores. Each store saves itself; a partial failure leaves the
/// caller with whatever wrote successfully (no two-phase commit).
fn apply_preferences(state: &AppState, view: PreferencesView) -> Result<(), Error> {
    {
        let mut editor = state.editor_prefs.lock().expect("editor prefs poisoned");
        editor.fonts = view.fonts;
        editor.theme = view.theme;
        editor.pane_widths = view.pane_widths;
        editor.line_spacing = view.line_spacing;
        editor.date_format = view.date_format;
        editor.save()?;
    }
    {
        let mut server = state.server_config.lock().expect("server config poisoned");
        if !view.attachments_dir.is_empty() {
            server.attachments_dir = view.attachments_dir;
        }
        if !view.assistant.answers_dir.is_empty() {
            server.answers_dir = view.assistant.answers_dir;
        }
        server.save()?;
    }
    {
        let mut llm = state.llm_config.lock().expect("llm config poisoned");
        // The "embedded" backend has no chan-llm counterpart yet; a
        // PATCH carrying it is a no-op (the field round-trips as
        // the previous backend on the next read).
        if let Some(kind) = view.assistant.backend.to_chan_llm() {
            llm.backend = if view.assistant.enabled {
                Some(kind)
            } else {
                None
            };
        } else if !view.assistant.enabled {
            llm.backend = None;
        }
        llm.auto_apply_writes = view.assistant.auto_apply_writes;
        llm.models.anthropic = view.assistant.claude.model;
        llm.models.gemini = view.assistant.gemini.model;
        llm.models.ollama = view.assistant.ollama.model;
        // None clears the override so backends fall back to their
        // built-in defaults; see chan-llm `MaxTokens` resolution.
        llm.max_tokens.anthropic = view.assistant.claude.max_tokens;
        llm.max_tokens.gemini = view.assistant.gemini.max_tokens;
        llm.max_tokens.ollama = view.assistant.ollama.max_tokens;
        // Empty string from the form clears the override (back to
        // env or the hardcoded default). Trim before storing so a
        // copy-pasted URL with whitespace doesn't break the http
        // client.
        llm.urls.ollama = view
            .assistant
            .ollama
            .url
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty());
        llm.save()
            .map_err(|e| Error::Config(format!("save llm config: {e}")))?;
    }
    Ok(())
}
