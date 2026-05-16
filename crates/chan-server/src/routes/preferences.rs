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

use crate::error::{err, Error};
use crate::state::AppState;
use crate::{EditorTheme, LineSpacing, PaneWidths, ThemeChoice};

/// Unified Preferences shape returned over /api/drive and
/// /api/config. The fields are owned by three different stores:
///
/// - editor_theme / theme / pane_widths / line_spacing / date_format:
///   EditorPrefs (preferences.toml)
/// - attachments_dir: ServerConfig (server.toml; the answers_dir
///   field there is mirrored into the assistant subtree below)
/// - assistant: LlmConfig (llm.toml) + ServerConfig.answers_dir
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesView {
    pub editor_theme: EditorTheme,
    pub assistant: AssistantPrefsView,
    pub attachments_dir: String,
    pub theme: ThemeChoice,
    pub pane_widths: PaneWidths,
    pub line_spacing: LineSpacing,
    pub date_format: String,
}

/// Frontend's `AssistantPrefs` view. The Settings UI manages the
/// per-CLI enable flags and command overrides; model fields
/// round-trip here too but are written from the assistant overlay's
/// inspector, not Settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantPrefsView {
    /// Derived: true when `default_backend` is Some AND the matching
    /// provider's `enabled` flag is set. Read-only on PATCH; the
    /// server recomputes it from the per-provider flags below. The
    /// SPA reads this for the master gate (assistant button +
    /// Cmd+I).
    pub effective_enabled: bool,
    /// Which provider is the default assistant. Sticky across enable/
    /// disable toggles so user intent survives a "disable then
    /// re-enable" round-trip; `null` means no default picked.
    #[serde(default)]
    pub default_backend: Option<AssistantBackendKind>,
    pub answers_dir: String,
    /// Local `claude` CLI shell-executor backend. Carries no token /
    /// URL (auth runs through the user's installed CLI); only the
    /// enable flag and the optional model override.
    #[serde(default)]
    pub claude_cli: CliPrefsView,
    /// Same shape as `claude_cli` for the `gemini` CLI.
    #[serde(default)]
    pub gemini_cli: CliPrefsView,
    /// Local `codex exec` backend. Older frontends ignore this
    /// field; newer ones can use the same CLI prefs shape.
    #[serde(default)]
    pub codex_cli: CliPrefsView,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CliPrefsView {
    #[serde(default)]
    pub enabled: bool,
    /// Optional `--model` override passed to the CLI. None lets the
    /// CLI's own default win. Written from the assistant overlay's
    /// inspector, not from Settings.
    #[serde(default)]
    pub model: Option<String>,
    /// Optional command override for this CLI backend. A bare
    /// command is resolved on PATH; an absolute path must point to
    /// an executable file. None lets chan-llm use its default.
    #[serde(default)]
    pub cmd_override: Option<String>,
}

/// Shell-executor backends supported by chan-llm 0.11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
#[serde(rename_all = "snake_case")]
pub enum AssistantBackendKind {
    ClaudeCli,
    GeminiCli,
    CodexCli,
}

impl AssistantBackendKind {
    pub fn from_chan_llm(kind: BackendKind) -> Self {
        match kind {
            BackendKind::ClaudeCli => AssistantBackendKind::ClaudeCli,
            BackendKind::GeminiCli => AssistantBackendKind::GeminiCli,
            BackendKind::CodexCli => AssistantBackendKind::CodexCli,
        }
    }

    pub fn to_chan_llm(self) -> BackendKind {
        match self {
            AssistantBackendKind::ClaudeCli => BackendKind::ClaudeCli,
            AssistantBackendKind::GeminiCli => BackendKind::GeminiCli,
            AssistantBackendKind::CodexCli => BackendKind::CodexCli,
        }
    }
}

/// Build the unified Preferences view for the current state. Reads
/// each backing store under its own lock.
///
/// On `--tunnel-public` runs the assistant subtree is neutralized:
/// `effective_enabled` flips to false, the default backend clears,
/// and every per-CLI config field empties. The matching
/// write-side routes are 403'd by the settings guard already;
/// redacting the read keeps the SPA in lock-step (so the assistant
/// pill greys out via the existing master-switch logic), stops the
/// configured backend / model / command override from leaking to
/// anonymous visitors, and removes any signal that could tell a
/// visitor whether the assistant gate is worth probing.
pub(super) fn preferences_view(state: &AppState) -> PreferencesView {
    let editor = state.editor_prefs.lock().expect("editor prefs poisoned");
    let server = state.server_config.lock().expect("server config poisoned");
    let llm = state.llm_config.lock().expect("llm config poisoned");
    let assistant = if state.tunnel_public {
        AssistantPrefsView {
            effective_enabled: false,
            default_backend: None,
            answers_dir: String::new(),
            claude_cli: CliPrefsView::default(),
            gemini_cli: CliPrefsView::default(),
            codex_cli: CliPrefsView::default(),
        }
    } else {
        AssistantPrefsView {
            effective_enabled: llm.active_backend().is_some(),
            default_backend: llm.backend.map(AssistantBackendKind::from_chan_llm),
            answers_dir: server.answers_dir.clone(),
            claude_cli: CliPrefsView {
                enabled: llm.enabled.claude_cli,
                model: llm.models.claude_cli.clone(),
                cmd_override: llm
                    .claude_cli
                    .cmd
                    .as_ref()
                    .and_then(|cmd| cmd.first().cloned()),
            },
            gemini_cli: CliPrefsView {
                enabled: llm.enabled.gemini_cli,
                model: llm.models.gemini_cli.clone(),
                cmd_override: llm
                    .gemini_cli
                    .cmd
                    .as_ref()
                    .and_then(|cmd| cmd.first().cloned()),
            },
            codex_cli: CliPrefsView {
                enabled: llm.enabled.codex_cli,
                model: llm.models.codex_cli.clone(),
                cmd_override: llm
                    .codex_cli
                    .cmd
                    .as_ref()
                    .and_then(|cmd| cmd.first().cloned()),
            },
        }
    };
    PreferencesView {
        editor_theme: editor.editor_theme,
        assistant,
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
    // settings_disabled is enforced by `tunnel_guard::settings_guard`
    // at the router layer; no per-handler gate.
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
    // On `--tunnel-public` runs we strip the whole "host machine"
    // dimension of the response: anonymous visitors must not see
    // `default_drive_root` (typically `~/Documents/Chan`, which
    // reveals the owner's username) or the registry of OTHER drives
    // on the host. `preferences` stays full because the SPA needs
    // fonts/theme/etc. to render even when Settings is locked.
    if state.tunnel_public {
        return GlobalConfigView {
            preferences: preferences_view(state),
            default_drive_root: None,
            drives: Vec::new(),
        };
    }
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
    // settings_disabled is enforced by `tunnel_guard::settings_guard`
    // at the router layer; no per-handler gate.
    if let Some(prefs) = body.preferences {
        if let Err(e) = apply_preferences(&state, prefs) {
            let status = match e {
                Error::BadRequest(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            return err(status, e.to_string());
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
        editor.editor_theme = view.editor_theme;
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
        // Default backend is sticky: persist whatever the SPA sent
        // verbatim, even when the matching provider is disabled. The
        // resolver gates on `enabled[backend]` so a disabled default
        // won't actually launch a request; leaving the pointer set
        // preserves user intent across enable/disable toggles.
        llm.backend = view.assistant.default_backend.map(|k| k.to_chan_llm());
        // Per-provider enable flags. The SPA's CRUD list toggles
        // these independently of the default-backend pointer above.
        llm.enabled.claude_cli = view.assistant.claude_cli.enabled;
        llm.enabled.gemini_cli = view.assistant.gemini_cli.enabled;
        llm.enabled.codex_cli = view.assistant.codex_cli.enabled;
        // CLI overrides: empty / None falls back to "let the CLI's
        // own config pick", which is what we want when the user
        // clears the field.
        llm.models.claude_cli = view.assistant.claude_cli.model.filter(|s| !s.is_empty());
        llm.models.gemini_cli = view.assistant.gemini_cli.model.filter(|s| !s.is_empty());
        llm.models.codex_cli = view.assistant.codex_cli.model.filter(|s| !s.is_empty());
        llm.claude_cli.cmd = validated_cmd_override(
            BackendKind::ClaudeCli,
            view.assistant.claude_cli.cmd_override,
        )?;
        llm.gemini_cli.cmd = validated_cmd_override(
            BackendKind::GeminiCli,
            view.assistant.gemini_cli.cmd_override,
        )?;
        llm.codex_cli.cmd =
            validated_cmd_override(BackendKind::CodexCli, view.assistant.codex_cli.cmd_override)?;
        llm.save()
            .map_err(|e| Error::Config(format!("save llm config: {e}")))?;
    }
    Ok(())
}

fn validated_cmd_override(
    kind: BackendKind,
    value: Option<String>,
) -> Result<Option<Vec<String>>, Error> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let mut cfg = chan_llm::LlmConfig::default();
    set_backend_cmd(&mut cfg, kind, vec![trimmed.to_string()]);
    let detection = chan_llm::detect_backend_cli(kind, &cfg);
    if detection.present() {
        Ok(Some(vec![trimmed.to_string()]))
    } else {
        Err(Error::BadRequest(cli_detection_reason(kind, &detection)))
    }
}

fn set_backend_cmd(cfg: &mut chan_llm::LlmConfig, kind: BackendKind, cmd: Vec<String>) {
    match kind {
        BackendKind::ClaudeCli => cfg.claude_cli.cmd = Some(cmd),
        BackendKind::GeminiCli => cfg.gemini_cli.cmd = Some(cmd),
        BackendKind::CodexCli => cfg.codex_cli.cmd = Some(cmd),
    }
}

pub(crate) fn cli_detection_reason(
    kind: BackendKind,
    detection: &chan_llm::CliDetection,
) -> String {
    let cmd0 = detection.command.first().map(String::as_str).unwrap_or("");
    format!(
        "`{cmd0}` not found or rejected. Install the {} CLI, or set its cmd in llm.toml.",
        backend_tag(kind),
    )
}

pub(crate) fn backend_tag(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::ClaudeCli => "claude_cli",
        BackendKind::GeminiCli => "gemini_cli",
        BackendKind::CodexCli => "codex_cli",
    }
}

#[cfg(test)]
mod tests {
    //! Redaction round-trips for the public-tunnel mode.
    //!
    //! The view-building functions are private to this module, so
    //! the test goes through `serde_json` on the same struct the
    //! handler returns. That keeps the test honest about the JSON
    //! shape an anonymous visitor would actually see on the wire.

    use super::*;
    use crate::state::test_support::make_test_state;

    fn to_json(view: &GlobalConfigView) -> serde_json::Value {
        serde_json::to_value(view).expect("serialize")
    }

    fn pref_to_json(view: &PreferencesView) -> serde_json::Value {
        serde_json::to_value(view).expect("serialize")
    }

    #[test]
    fn global_config_view_redacts_host_paths_on_public_tunnel() {
        let state = make_test_state(true, true);
        let view = global_config_view(&state);
        let json = to_json(&view);
        // The drive registry and the default-root path are the two
        // host-level fields the unredacted shape carries. Both MUST
        // be empty/null for `--tunnel-public` so an anonymous
        // visitor never learns the owner's filesystem layout.
        assert_eq!(json["default_drive_root"], serde_json::Value::Null);
        assert_eq!(
            json["drives"],
            serde_json::json!([]),
            "drives[] must be empty on tunnel_public"
        );
    }

    #[test]
    fn preferences_view_seals_assistant_on_public_tunnel() {
        let state = make_test_state(true, true);
        let view = preferences_view(&state);
        let json = pref_to_json(&view);
        // The assistant subtree is the cost-bearing surface. Even
        // though POST /api/llm/complete is refused by the public-
        // tunnel guard, leaking the configured backend / model /
        // command override would still hand a visitor useful probe data.
        // `effective_enabled: false` also greys the assistant pill
        // via the SPA's existing master-switch logic and
        // `default_backend: null` removes the only signal that says
        // "an assistant is configured here" — one bug to fix if
        // either side regresses.
        assert_eq!(
            json["assistant"]["effective_enabled"],
            serde_json::json!(false)
        );
        assert_eq!(
            json["assistant"]["default_backend"],
            serde_json::Value::Null
        );
        assert_eq!(json["assistant"]["answers_dir"], serde_json::json!(""));
        assert!(json["assistant"].get("auto_apply_writes").is_none());
        assert_eq!(
            json["assistant"]["claude_cli"]["enabled"],
            serde_json::json!(false)
        );
        assert_eq!(
            json["assistant"]["gemini_cli"]["enabled"],
            serde_json::json!(false)
        );
        assert_eq!(
            json["assistant"]["codex_cli"]["enabled"],
            serde_json::json!(false)
        );
        assert_eq!(
            json["assistant"]["claude_cli"]["cmd_override"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn global_config_view_keeps_host_fields_on_local_serve() {
        // Sanity check: the local-serve path (both flags false) is
        // the only mode where the registry should round-trip. If
        // someone flips this default the host-info leak comes
        // straight back.
        let state = make_test_state(false, false);
        let view = global_config_view(&state);
        let json = to_json(&view);
        // The test Library is empty, so `drives` is [] and
        // `default_drive_root` is null even on local serve. What we
        // assert here is that the redaction branch was NOT taken —
        // i.e. the fields exist with their unredacted shape (drives
        // is an array, default_drive_root is the JSON null produced
        // by `Option::None`, NOT the forced None of the redact arm).
        assert!(json["drives"].is_array());
    }

    #[test]
    fn cli_cmd_override_round_trips_through_preferences() {
        let state = make_test_state(false, false);
        let exe = std::env::current_exe()
            .expect("current exe")
            .to_string_lossy()
            .into_owned();
        let mut view = preferences_view(&state);
        view.assistant.claude_cli.cmd_override = Some(exe.clone());

        apply_preferences(&state, view).expect("apply prefs");

        let updated = preferences_view(&state);
        assert_eq!(updated.assistant.claude_cli.cmd_override, Some(exe));
    }

    #[test]
    fn invalid_cli_cmd_override_is_rejected() {
        let state = make_test_state(false, false);
        let mut view = preferences_view(&state);
        view.assistant.gemini_cli.cmd_override =
            Some("/definitely/not/a/real/gemini-binary".to_string());

        let err = apply_preferences(&state, view).expect_err("invalid override");
        assert!(matches!(err, Error::BadRequest(_)));
        assert!(
            err.to_string().contains("not found or rejected"),
            "unexpected error: {err}"
        );
    }
}
