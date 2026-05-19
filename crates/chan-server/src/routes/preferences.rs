//! Server preferences: per-server config (`/api/server/config`) and
//! the unified GlobalConfig view (`/api/config`).
//!
//! The unified surface joins EditorPrefs, ServerConfig, and the
//! chan-drive registry. Agent/assistant preferences were removed with
//! the assistant overlay; MCP access is configured through the server
//! runtime, not through global user preferences.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_drive::SearchAggression;
use serde::{Deserialize, Serialize};

use crate::config::TerminalConfig;
use crate::error::{err, Error};
use crate::preferences::BubbleOverlayMode;
use crate::state::AppState;
use crate::{BrowserSidePanes, EditorTheme, LineSpacing, PaneWidths, ThemeChoice};

/// Unified preferences shape returned over /api/drive and /api/config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesView {
    pub editor_theme: EditorTheme,
    pub attachments_dir: String,
    pub theme: ThemeChoice,
    pub pane_widths: PaneWidths,
    #[serde(default)]
    pub browser_side_panes: BrowserSidePanes,
    pub line_spacing: LineSpacing,
    pub date_format: String,
    pub strip_trailing_whitespace_on_save: bool,
    pub search_aggression: SearchAggression,
    pub terminal: TerminalConfig,
    #[serde(default)]
    pub bubble_overlay_mode: BubbleOverlayMode,
    #[serde(default = "default_empty_pane_carousel_cycling")]
    pub empty_pane_carousel_cycling: bool,
}

fn default_empty_pane_carousel_cycling() -> bool {
    true
}

pub(super) fn preferences_view(state: &AppState) -> PreferencesView {
    let editor = state.editor_prefs.lock().expect("editor prefs poisoned");
    let server = state.server_config.lock().expect("server config poisoned");
    PreferencesView {
        editor_theme: editor.editor_theme,
        attachments_dir: server.attachments_dir.clone(),
        theme: editor.theme,
        pane_widths: editor.pane_widths,
        browser_side_panes: editor.browser_side_panes,
        line_spacing: editor.line_spacing,
        date_format: editor.date_format.clone(),
        strip_trailing_whitespace_on_save: editor.strip_trailing_whitespace_on_save,
        search_aggression: server.search.aggression,
        terminal: server.terminal.clone(),
        bubble_overlay_mode: editor.bubble_overlay_mode,
        empty_pane_carousel_cycling: editor.empty_pane_carousel_cycling,
    }
}

// ----- /api/server/config ------------------------------------------------

pub async fn api_get_server_config(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.server_config.lock().unwrap().clone();
    Json(cfg).into_response()
}

#[derive(Deserialize)]
pub struct PatchServerConfigBody {
    /// Drive-relative POSIX path. Empty string is rejected because the
    /// path is used as a prefix; an empty prefix would land attachments
    /// in the drive root, surprising the user.
    #[serde(default)]
    attachments_dir: Option<String>,
    #[serde(default)]
    search: Option<crate::config::SearchConfig>,
    #[serde(default)]
    terminal: Option<TerminalConfig>,
}

pub async fn api_patch_server_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PatchServerConfigBody>,
) -> Response {
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
    if let Some(search) = body.search {
        cfg.search = search;
    }
    if let Some(terminal) = body.terminal {
        cfg.terminal = sanitize_terminal_config(terminal);
    }
    if let Err(e) = cfg.save() {
        return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    Json(cfg.clone()).into_response()
}

// ----- /api/config (unified GlobalConfig) --------------------------------

#[derive(Serialize)]
struct GlobalConfigView {
    preferences: PreferencesView,
    /// Empty string serializes as None (the resolver falls back to the
    /// platform default).
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
    /// Whole-block replacement. Frontend sends the entire GlobalConfig
    /// on every save.
    #[serde(default)]
    preferences: Option<PreferencesView>,
    #[serde(default)]
    default_drive_root: Option<Option<String>>,
    /// Read-only on PATCH: drives are managed via /api/drive PATCH and
    /// the CLI (`chan add` / `remove`). Frontend sends the field for
    /// round-tripping; we just ignore it.
    #[serde(default)]
    #[allow(dead_code)]
    drives: Option<serde_json::Value>,
}

fn global_config_view(state: &AppState) -> GlobalConfigView {
    // On `--tunnel-public` runs we strip the whole "host machine"
    // dimension of the response: anonymous visitors must not see
    // `default_drive_root` or the registry of other drives on the host.
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

fn apply_preferences(state: &AppState, view: PreferencesView) -> Result<(), Error> {
    {
        let mut editor = state.editor_prefs.lock().expect("editor prefs poisoned");
        editor.editor_theme = view.editor_theme;
        editor.theme = view.theme;
        editor.pane_widths = view.pane_widths;
        editor.browser_side_panes = view.browser_side_panes;
        editor.line_spacing = view.line_spacing;
        editor.date_format = view.date_format;
        editor.strip_trailing_whitespace_on_save = view.strip_trailing_whitespace_on_save;
        editor.bubble_overlay_mode = view.bubble_overlay_mode;
        editor.empty_pane_carousel_cycling = view.empty_pane_carousel_cycling;
        editor.save()?;
    }
    {
        let mut server = state.server_config.lock().expect("server config poisoned");
        if !view.attachments_dir.is_empty() {
            server.attachments_dir = view.attachments_dir;
        }
        server.search.aggression = view.search_aggression;
        server.terminal = sanitize_terminal_config(view.terminal);
        server.save()?;
    }
    Ok(())
}

fn sanitize_terminal_config(mut cfg: TerminalConfig) -> TerminalConfig {
    if cfg.idle_timeout_secs == 0 {
        cfg.idle_timeout_secs = TerminalConfig::default().idle_timeout_secs;
    }
    if cfg.session_cap == 0 {
        cfg.session_cap = TerminalConfig::default().session_cap;
    }
    if cfg.ring_bytes == 0 {
        cfg.ring_bytes = TerminalConfig::default().ring_bytes;
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_support::make_test_state;

    fn to_json(view: &GlobalConfigView) -> serde_json::Value {
        serde_json::to_value(view).expect("serialize")
    }

    #[test]
    fn global_config_view_redacts_host_paths_on_public_tunnel() {
        let state = make_test_state(true, true);
        let view = global_config_view(&state);
        let json = to_json(&view);
        assert_eq!(json["default_drive_root"], serde_json::Value::Null);
        assert_eq!(json["drives"], serde_json::json!([]));
    }

    #[test]
    fn preferences_view_has_no_assistant_subtree() {
        let state = make_test_state(false, false);
        let view = preferences_view(&state);
        let json = serde_json::to_value(view).expect("serialize");
        assert!(json.get("assistant").is_none());
    }

    #[test]
    fn global_config_view_keeps_host_fields_on_local_serve() {
        let state = make_test_state(false, false);
        let view = global_config_view(&state);
        let json = to_json(&view);
        assert!(json["drives"].is_array());
    }
}
