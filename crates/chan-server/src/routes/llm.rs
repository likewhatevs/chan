//! `/api/llm/*` — assistant routes.
//!
//! Routes wrap chan_llm::LlmSession. Streaming events flow over the
//! shared /ws so the frontend has one socket to read from. The route
//! surface stays valid even when chan-llm's backends are stubs:
//! complete() emits an immediate llm.error + llm.done frame for the
//! configured backend.

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_llm::{BackendKind, LlmSession, Message as LlmMessage, Role as LlmRole, SessionListener};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::bus::LlmBroadcastListener;
use crate::cli_resolve::{api_keys_path_string, resolve_claude_cli, resolve_gemini_cli};
use crate::error::{err, err_llm, err_settings_locked};
use crate::state::AppState;

/// `/api/llm/status` view shape. Frontend's `LlmStatus` type is a
/// flat one-active-backend snapshot; the previous per-backend dict
/// shape didn't match (and threw at render time when SettingsPanel
/// reached for `key.set` on the missing field). One source of truth
/// per request: the configured backend, its effective model, and
/// the resolution status of its key.
#[derive(Serialize)]
struct LlmStatus {
    /// Frontend's display tag for the active backend.
    /// "claude" | "ollama" | "gemini". The "embedded" variant in
    /// the typescript type is reserved for a future on-device
    /// backend; not surfaced here yet.
    backend: &'static str,
    /// Effective model for the active backend (config override or
    /// the chan-llm default).
    model: Option<String>,
    /// Key resolution snapshot for the active backend.
    key: LlmKeyView,
    /// Whether a request would succeed today (active backend
    /// configured + key resolves, or Ollama which is keyless).
    ready: bool,
    /// Human-readable explanation when `ready = false`. Absent on
    /// the happy path so the UI knows there's nothing to surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    /// Mirror of LlmConfig.backend.is_some(). Settings hides the
    /// assistant button when this flips off.
    enabled: bool,
    /// Backend supports tool use. All three current backends do;
    /// retained as a typed field so future "chat-only" variants
    /// don't break the contract.
    supports_tools: bool,
}

#[derive(Serialize)]
struct LlmKeyView {
    /// True iff the key resolved through any of env / keychain /
    /// file. Settings disables the "refresh models" buttons when
    /// this is false.
    set: bool,
    /// Lowercase tag for where the key came from. None when not
    /// set (the union with `set: false`).
    source: Option<&'static str>,
    /// Where the on-disk fallback would land. Constant per machine;
    /// surfaced so the Settings tab can point the user at the file
    /// to edit on a headless box.
    path: Option<String>,
    /// True when the OS keychain backend is reachable. Settings
    /// hides keychain controls on headless boxes (no Secret
    /// Service / DBus session, locked keychain, etc.).
    keychain_available: bool,
}

/// Map the active chan-llm BackendKind to the frontend's display
/// tag. Anthropic surfaces as "claude" because that's the brand the
/// user picks from the dropdown.
fn backend_tag(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::Anthropic => "claude",
        BackendKind::Ollama => "ollama",
        BackendKind::Gemini => "gemini",
        BackendKind::ClaudeCli => "claude_cli",
        BackendKind::GeminiCli => "gemini_cli",
    }
}

fn key_status_tag(s: chan_llm::KeyStatus) -> Option<&'static str> {
    match s {
        chan_llm::KeyStatus::Env => Some("env"),
        chan_llm::KeyStatus::Keychain => Some("keychain"),
        chan_llm::KeyStatus::File => Some("file"),
        chan_llm::KeyStatus::Missing => None,
    }
}

/// Whether the OS keychain backend is reachable on this machine.
/// chan-llm doesn't expose a probe today; we report `true`
/// optimistically and let actual set / clear calls surface errors
/// through `/api/llm/keys/<provider>` when the backend isn't
/// usable. macOS Keychain, Windows Credential Manager, and
/// gnome-keyring / KWallet on a desktop Linux session all work
/// out of the box; the field is reserved as a future hook for
/// detecting headless boxes.
fn keychain_available() -> bool {
    true
}

pub async fn api_llm_status(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.llm_config.lock().unwrap().clone();
    let active = cfg.backend.unwrap_or(BackendKind::Anthropic);
    let model = cfg
        .models
        .for_backend(active)
        .map(str::to_owned)
        .or_else(|| Some(active.default_model().to_string()));
    let (active_key, status) = chan_llm::keys::resolve(active, &cfg);
    let key_set = active_key.is_some();
    let enabled = cfg.backend.is_some();
    // Resolve cmd[0] for the ClaudeCli backend so we can probe PATH.
    // Mirrors backends::build's resolution: explicit cfg overrides
    // win, otherwise chan-llm's `default_cmd()` (currently `claude`).
    let claude_cli_cmd0 = cfg
        .claude_cli
        .cmd
        .as_ref()
        .and_then(|v| v.first().cloned())
        .unwrap_or_else(|| {
            chan_llm::backends::claude_cli::default_cmd()
                .into_iter()
                .next()
                .unwrap_or_default()
        });
    let claude_cli_resolved = if active == BackendKind::ClaudeCli {
        resolve_claude_cli(&claude_cli_cmd0)
    } else {
        None
    };
    // Same shape for the GeminiCli backend.
    let gemini_cli_cmd0 = cfg
        .gemini_cli
        .cmd
        .as_ref()
        .and_then(|v| v.first().cloned())
        .unwrap_or_else(|| {
            chan_llm::backends::gemini_cli::default_cmd()
                .into_iter()
                .next()
                .unwrap_or_default()
        });
    let gemini_cli_resolved = if active == BackendKind::GeminiCli {
        resolve_gemini_cli(&gemini_cli_cmd0)
    } else {
        None
    };
    // Ollama is keyless (local); Anthropic and Gemini need a key.
    // ClaudeCli/GeminiCli inherit auth from the installed CLI, but we
    // still need to find the binary on PATH to consider them ready.
    // GeminiCli additionally fails if v2 launches without a stored
    // GEMINI_API_KEY (the redirected GEMINI_CLI_HOME blocks the
    // user's `gemini login` auth); we don't gate `ready` on that
    // here since v1 mode still works without a key.
    let ready = enabled
        && match active {
            BackendKind::Ollama => true,
            BackendKind::ClaudeCli => claude_cli_resolved.is_some(),
            BackendKind::GeminiCli => gemini_cli_resolved.is_some(),
            BackendKind::Anthropic | BackendKind::Gemini => key_set,
        };
    let reason = if !enabled {
        Some("no backend selected; pick one in Settings".to_string())
    } else if !ready {
        match active {
            BackendKind::ClaudeCli => Some(format!(
                "`{claude_cli_cmd0}` not found on PATH. Install the claude \
                 CLI, or set claude_cli.cmd in llm.toml to an absolute path."
            )),
            BackendKind::GeminiCli => Some(format!(
                "`{gemini_cli_cmd0}` not found on PATH. Install the gemini \
                 CLI (`npm i -g @google/gemini-cli`), or set gemini_cli.cmd \
                 in llm.toml to an absolute path."
            )),
            BackendKind::Ollama => {
                // Reachable only if a future change adds an Ollama
                // readiness gate; today the match arm above keeps
                // Ollama always-ready when enabled.
                Some("Ollama backend not ready.".to_string())
            }
            BackendKind::Anthropic | BackendKind::Gemini => {
                let env = match active {
                    BackendKind::Anthropic => "ANTHROPIC_API_KEY",
                    BackendKind::Gemini => "GEMINI_API_KEY",
                    _ => unreachable!(),
                };
                Some(format!(
                    "{} key not configured. Set {env} in your shell, or save \
                     the key from this Settings panel.",
                    backend_tag(active),
                ))
            }
        }
    } else {
        None
    };
    Json(LlmStatus {
        backend: backend_tag(active),
        model,
        key: LlmKeyView {
            set: key_set,
            source: key_status_tag(status),
            path: Some(api_keys_path_string()),
            keychain_available: keychain_available(),
        },
        ready,
        reason,
        enabled,
        supports_tools: true,
    })
    .into_response()
}

#[derive(Serialize)]
struct LlmToolSchema {
    name: &'static str,
    description: &'static str,
}

pub async fn api_llm_tools() -> Response {
    Json([
        LlmToolSchema {
            name: "read_file",
            description: chan_llm::prompts::READ_FILE_DESC,
        },
        LlmToolSchema {
            name: "write_file",
            description: chan_llm::prompts::WRITE_FILE_DESC,
        },
        LlmToolSchema {
            name: "list_files",
            description: chan_llm::prompts::LIST_FILES_DESC,
        },
        LlmToolSchema {
            name: "search_content",
            description: chan_llm::prompts::SEARCH_CONTENT_DESC,
        },
    ])
    .into_response()
}

#[derive(Deserialize)]
pub struct CompleteBody {
    /// Client-generated correlation id. Echoed on every llm.* WS
    /// frame so the frontend can match streaming events to its
    /// pending turn. Optional: when absent, the server generates
    /// a random one (callers that consume only the synchronous
    /// JSON response don't need to track an id at all).
    #[serde(default)]
    session_id: Option<String>,
    /// Convenience: a single user message. The server wraps this
    /// in a one-element Vec<Message> for the LLM. Use
    /// `messages` instead when the host needs to pass full
    /// transcript / tool-result turns.
    #[serde(default)]
    message: Option<String>,
    /// Full conversation transcript. Wins over `message` when
    /// both are set. The frontend builds this from its persisted
    /// assistant blob (see /api/assistant/conversation) and
    /// passes the full history each turn so chan-llm stays
    /// stateless.
    #[serde(default)]
    messages: Vec<ApiMessage>,
    /// Tools the caller wants to expose to this turn. Optional;
    /// chan-llm prepends its own standard tool schemas
    /// internally. Today this field is observed for forward
    /// compatibility but not actually plumbed (chan-llm's send()
    /// uses standard_tool_schemas unconditionally).
    #[serde(default)]
    #[allow(dead_code)]
    tools: Option<serde_json::Value>,
    /// Output cap. Per-backend defaults are sane; passed through
    /// for forward compatibility but currently ignored.
    #[serde(default)]
    #[allow(dead_code)]
    max_tokens: Option<u32>,
    /// Sampling temperature. Ignored today (extended-thinking
    /// models reject explicit values; we let backends pick).
    #[serde(default)]
    #[allow(dead_code)]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct ApiMessage {
    role: ApiRole,
    content: String,
    #[serde(default)]
    tool_call_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ApiRole {
    System,
    User,
    Assistant,
    Tool,
}

impl From<ApiMessage> for LlmMessage {
    fn from(m: ApiMessage) -> Self {
        let role = match m.role {
            ApiRole::System => LlmRole::System,
            ApiRole::User => LlmRole::User,
            ApiRole::Assistant => LlmRole::Assistant,
            ApiRole::Tool => LlmRole::Tool,
        };
        LlmMessage {
            role,
            content: m.content,
            tool_call_id: m.tool_call_id,
            tool_calls: Vec::new(),
        }
    }
}

/// Frontend's expected response shape for /api/llm/complete:
/// the full assistant turn returned synchronously. Streaming
/// deltas still flow over /ws as a side channel, but the simple
/// non-streaming caller (InlineAssist's submit path) just awaits
/// the JSON body and reads it once.
#[derive(Serialize)]
struct LlmCompletionResponse {
    content: String,
    tool_calls: Vec<LlmToolCallView>,
    /// Frontend's LlmStopReason: "end_turn" | "max_tokens" |
    /// "tool_use" | "stop_sequence" | "other".
    stop_reason: &'static str,
    model: String,
}

#[derive(Serialize)]
struct LlmToolCallView {
    id: String,
    name: String,
    /// chan-llm's struct uses `args`; the frontend types the same
    /// JSON value as `input` per Anthropic's convention. Renamed
    /// at the seam so neither side needs to know about the other.
    input: serde_json::Value,
}

fn stop_reason_tag(r: chan_llm::StopReason) -> &'static str {
    match r {
        chan_llm::StopReason::EndOfTurn => "end_turn",
        chan_llm::StopReason::MaxTokens => "max_tokens",
        chan_llm::StopReason::StopSequence => "stop_sequence",
        chan_llm::StopReason::ToolUse => "tool_use",
        chan_llm::StopReason::Error => "other",
        // chan-llm 0.5.1 added Cancelled for `CancelHandle::cancel`.
        // Surface it on the wire so the frontend can distinguish a
        // user-aborted turn from an upstream error.
        chan_llm::StopReason::Cancelled => "cancelled",
    }
}

/// Listener that forwards events to the broadcast channel (so /ws
/// subscribers see them live) AND collects the final assistant
/// text + tool calls in memory so the HTTP handler can return
/// them synchronously. Completion is signalled via a Notify that
/// the handler awaits before responding.
struct CollectListener {
    forward: LlmBroadcastListener,
    state: Mutex<CollectState>,
    done: tokio::sync::Notify,
}

struct CollectState {
    text: String,
    tool_calls: Vec<chan_llm::ToolCall>,
    stop_reason: Option<chan_llm::StopReason>,
    error: Option<String>,
    finished: bool,
}

impl CollectListener {
    fn new(forward: LlmBroadcastListener) -> Self {
        Self {
            forward,
            state: Mutex::new(CollectState {
                text: String::new(),
                tool_calls: Vec::new(),
                stop_reason: None,
                error: None,
                finished: false,
            }),
            done: tokio::sync::Notify::new(),
        }
    }
}

impl SessionListener for CollectListener {
    fn on_delta(&self, delta: chan_llm::Delta) {
        self.state
            .lock()
            .expect("collect state poisoned")
            .text
            .push_str(&delta.text);
        self.forward.on_delta(delta);
    }
    fn on_tool_call(&self, call: chan_llm::ToolCall) {
        self.state
            .lock()
            .expect("collect state poisoned")
            .tool_calls
            .push(call.clone());
        self.forward.on_tool_call(call);
    }
    fn on_tool_result(&self, result: chan_llm::ToolResult) {
        self.forward.on_tool_result(result);
    }
    fn on_done(&self, reason: chan_llm::StopReason) {
        {
            let mut s = self.state.lock().expect("collect state poisoned");
            s.stop_reason = Some(reason);
            s.finished = true;
        }
        self.done.notify_waiters();
        self.forward.on_done(reason);
    }
    fn on_error(&self, error: String) {
        {
            let mut s = self.state.lock().expect("collect state poisoned");
            if s.error.is_none() {
                s.error = Some(error.clone());
            }
        }
        self.forward.on_error(error);
    }
}

/// Build the argv chan-llm hands to claude / gemini as the chan
/// MCP server command. Resolves to `chan __mcp-proxy <socket>`,
/// where `<socket>` is the per-server Unix-domain socket the
/// in-process MCP bridge listens on. The proxy just relays stdio
/// to/from the socket: it doesn't reopen the drive, so the agent's
/// MCP child sidesteps chan-drive's per-drive flock that
/// chan-server already holds.
///
/// Returns `None` when `current_exe()` fails (we don't know how to
/// re-invoke ourselves), the path is non-UTF-8 (the gemini settings
/// JSON / claude --mcp-config JSON is text), or the bridge failed
/// to bind a socket at boot (read-only tmpdir, exotic platform).
/// Callers fall back to v1 black-box mode in that case.
fn mcp_subcommand_for(socket_path: Option<&Path>) -> Option<Vec<String>> {
    let socket = socket_path?.to_str()?.to_string();
    let exe = std::env::current_exe().ok()?;
    Some(vec![
        exe.to_str()?.to_string(),
        "__mcp-proxy".to_string(),
        socket,
    ])
}

pub async fn api_llm_complete(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CompleteBody>,
) -> Response {
    let mut config = state.llm_config.lock().unwrap().clone();
    // Active backend determines the model echoed back in the
    // response. Falls through the same way /api/llm/status does
    // (config override > backend default).
    let active = config.backend.unwrap_or(BackendKind::Anthropic);
    let model = config
        .models
        .for_backend(active)
        .map(str::to_owned)
        .unwrap_or_else(|| active.default_model().to_string());

    // session_id is optional now; generate one when absent so the
    // /ws side channel still has a correlatable id without
    // requiring the simple sync caller to track one.
    let session_id = body.session_id.clone().unwrap_or_else(random_session_id);

    // For the agentic CLI backends, point the backend at our own
    // binary as the chan-llm MCP server. chan-llm launches
    // claude / gemini with the appropriate v2 wiring (claude:
    // `--mcp-config` file; gemini: redirected GEMINI_CLI_HOME with
    // an mcpServers entry) so writes flow back through chan-drive's
    // gates (chan-llm issue #1, v0.5.0; gemini_cli v2 added in 0.7.0).
    // On any failure to resolve the current exe path we leave
    // mcp_command empty: chan-llm falls back to v1 black-box mode
    // (auto-apply forced on) and the user still gets a working
    // assistant.
    let socket = state.mcp_socket_path.as_deref();
    match active {
        BackendKind::ClaudeCli => {
            if let Some(cmd) = mcp_subcommand_for(socket) {
                config.claude_cli.mcp_command = Some(cmd);
            }
        }
        BackendKind::GeminiCli => {
            if let Some(cmd) = mcp_subcommand_for(socket) {
                config.gemini_cli.mcp_command = Some(cmd);
            }
        }
        _ => {}
    }

    let session = LlmSession::new(state.drive().clone(), config);
    let collect = Arc::new(CollectListener::new(LlmBroadcastListener {
        tx: state.events_tx.clone(),
        session_id,
    }));

    // Prefer the full `messages` array; fall back to wrapping a
    // single `message` string as one user turn. The frontend
    // sends the array form once it has chat history; the simpler
    // form is for one-shot prompts without context.
    let messages: Vec<LlmMessage> = if !body.messages.is_empty() {
        body.messages.into_iter().map(LlmMessage::from).collect()
    } else if let Some(text) = body.message {
        vec![LlmMessage::user(text)]
    } else {
        return err(
            StatusCode::BAD_REQUEST,
            "either `message` or `messages` is required".into(),
        );
    };

    // chan-llm's send is fire-and-forget (spawns the run_loop on
    // the ambient runtime); we wait on CollectListener's Notify
    // for the on_done signal. Events still fan out to /ws live;
    // this handler just blocks until the turn completes.
    let listener: Arc<dyn SessionListener> = collect.clone();
    session.send(messages, listener);
    collect.done.notified().await;

    let snapshot = collect.state.lock().expect("collect state poisoned");
    if let Some(err_msg) = snapshot.error.clone() {
        // chan-llm reports backend / network failures via on_error
        // before on_done(Error). Surface the original message at
        // 502 so the chat UI can show "anthropic 401: ..." instead
        // of a vague "other".
        return err(StatusCode::BAD_GATEWAY, err_msg);
    }
    let stop = snapshot
        .stop_reason
        .unwrap_or(chan_llm::StopReason::EndOfTurn);
    let tool_calls = snapshot
        .tool_calls
        .iter()
        .map(|c| LlmToolCallView {
            id: c.id.clone(),
            name: c.name.clone(),
            input: c.args.clone(),
        })
        .collect();
    Json(LlmCompletionResponse {
        content: snapshot.text.clone(),
        tool_calls,
        stop_reason: stop_reason_tag(stop),
        model,
    })
    .into_response()
}

/// Random session id for the WS correlation channel. Used when the
/// caller didn't supply one. Same alphabet as the auth token; the
/// id is opaque so the exact shape doesn't matter as long as it's
/// unlikely to collide on the same socket.
fn random_session_id() -> String {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| {
            const A: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
            A[(*b as usize) % A.len()] as char
        })
        .collect()
}

#[derive(Deserialize)]
pub struct SetKeyBody {
    key: String,
}

/// Set a per-backend key with a verify + file-fallback flow:
///
///   1. Try keychain set. On Linux / Windows / signed-binary macOS
///      this is the secure path and is what we want to land.
///   2. Verify the keychain actually persisted: a known macOS issue
///      with unsigned dev binaries is that Security.framework
///      silently no-ops some operations (set_password returns Ok
///      but get_password returns NoEntry afterward). We
///      `keychain_lookup` to detect that case.
///   3. When the keychain didn't stick, write to the on-disk file
///      tier (`<config>/chan/llm.toml`'s [keys] section, mode 0600).
///      That tier is keyed off LlmConfig.keys and walked by
///      `keys::resolve` last; either way the key reaches the
///      backend.
///
/// On a properly-signed install the file tier never gets touched;
/// on dev binaries it's the working path until signing lands.
async fn set_backend_key(state: &Arc<AppState>, kind: BackendKind, key: String) -> Response {
    if let Err(e) = chan_llm::keys::set(kind, &key) {
        return err_llm(&e);
    }
    let kept = chan_llm::keys::keychain_lookup(kind).is_some();
    if !kept {
        let mut cfg = state.llm_config.lock().expect("llm config poisoned");
        match kind {
            BackendKind::Anthropic => cfg.keys.anthropic = Some(key),
            BackendKind::Gemini => cfg.keys.gemini = Some(key),
            // Ollama and ClaudeCli are keyless; the routes shouldn't
            // call this path for them, but if they do we drop the
            // value silently rather than poison the file.
            BackendKind::Ollama | BackendKind::ClaudeCli | BackendKind::GeminiCli => {}
        }
        if let Err(e) = cfg.save() {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("save llm config: {e}"),
            );
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

/// Clear a per-backend key. Mirrors set_backend_key: drop the
/// keychain entry AND zero the file fallback so the next
/// resolve() walks back to env-or-missing.
async fn clear_backend_key(state: &Arc<AppState>, kind: BackendKind) -> Response {
    if let Err(e) = chan_llm::keys::clear(kind) {
        return err_llm(&e);
    }
    let mut cfg = state.llm_config.lock().expect("llm config poisoned");
    match kind {
        BackendKind::Anthropic => cfg.keys.anthropic = None,
        BackendKind::Gemini => cfg.keys.gemini = None,
        BackendKind::Ollama | BackendKind::ClaudeCli | BackendKind::GeminiCli => {}
    }
    if let Err(e) = cfg.save() {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("save llm config: {e}"),
        );
    }
    StatusCode::NO_CONTENT.into_response()
}

pub async fn api_llm_set_anthropic_key(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetKeyBody>,
) -> Response {
    if state.settings_disabled {
        return err_settings_locked();
    }
    set_backend_key(&state, BackendKind::Anthropic, body.key).await
}

pub async fn api_llm_clear_anthropic_key(State(state): State<Arc<AppState>>) -> Response {
    if state.settings_disabled {
        return err_settings_locked();
    }
    clear_backend_key(&state, BackendKind::Anthropic).await
}

pub async fn api_llm_set_gemini_key(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetKeyBody>,
) -> Response {
    if state.settings_disabled {
        return err_settings_locked();
    }
    set_backend_key(&state, BackendKind::Gemini, body.key).await
}

pub async fn api_llm_clear_gemini_key(State(state): State<Arc<AppState>>) -> Response {
    if state.settings_disabled {
        return err_settings_locked();
    }
    clear_backend_key(&state, BackendKind::Gemini).await
}

/// One model entry in a catalog response. `supports_tools` is
/// hardcoded true for Anthropic and Gemini today (their entire
/// chat catalog supports function calling); future non-tool
/// variants would narrow this by name.
#[derive(Serialize)]
struct LlmModelEntry {
    name: &'static str,
    supports_tools: bool,
}

#[derive(Serialize)]
struct LlmModelEntryOwned {
    name: String,
    supports_tools: bool,
}

#[derive(Serialize)]
struct CatalogResponse {
    models: Vec<LlmModelEntryOwned>,
    /// Provenance tag for the Settings UI's "why is this list
    /// short" copy. live = fetched from upstream, curated = no
    /// key set so we returned a static shortlist, fallback = key
    /// set but live fetch failed.
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Curated Anthropic shortlist. Used when no key is configured
/// (so the dropdown isn't empty) and as the fallback when the
/// `/v1/models` call fails. Sorted newest-first; trim when older
/// generations stop being recommended.
const CURATED_ANTHROPIC: &[LlmModelEntry] = &[
    LlmModelEntry {
        name: "claude-opus-4-7",
        supports_tools: true,
    },
    LlmModelEntry {
        name: "claude-sonnet-4-6",
        supports_tools: true,
    },
    LlmModelEntry {
        name: "claude-haiku-4-5",
        supports_tools: true,
    },
];

/// Curated Gemini shortlist. Same purpose as the Anthropic one.
const CURATED_GEMINI: &[LlmModelEntry] = &[
    LlmModelEntry {
        name: "gemini-2.5-pro",
        supports_tools: true,
    },
    LlmModelEntry {
        name: "gemini-2.5-flash",
        supports_tools: true,
    },
];

fn curated_to_owned(curated: &[LlmModelEntry]) -> Vec<LlmModelEntryOwned> {
    curated
        .iter()
        .map(|e| LlmModelEntryOwned {
            name: e.name.to_string(),
            supports_tools: e.supports_tools,
        })
        .collect()
}

pub async fn api_llm_anthropic_models(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.llm_config.lock().unwrap().clone();
    let (key, _) = chan_llm::keys::resolve(BackendKind::Anthropic, &cfg);
    let Some(key) = key else {
        return Json(CatalogResponse {
            models: curated_to_owned(CURATED_ANTHROPIC),
            source: "curated",
            error: None,
        })
        .into_response();
    };
    match chan_llm::backends::anthropic::list_models(&key).await {
        Ok(models) => Json(CatalogResponse {
            models: models
                .into_iter()
                .map(|name| LlmModelEntryOwned {
                    name,
                    supports_tools: true,
                })
                .collect(),
            source: "live",
            error: None,
        })
        .into_response(),
        Err(e) => Json(CatalogResponse {
            models: curated_to_owned(CURATED_ANTHROPIC),
            source: "fallback",
            error: Some(e.to_string()),
        })
        .into_response(),
    }
}

pub async fn api_llm_gemini_models(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.llm_config.lock().unwrap().clone();
    let (key, _) = chan_llm::keys::resolve(BackendKind::Gemini, &cfg);
    let Some(key) = key else {
        return Json(CatalogResponse {
            models: curated_to_owned(CURATED_GEMINI),
            source: "curated",
            error: None,
        })
        .into_response();
    };
    match chan_llm::backends::gemini::list_models(&key).await {
        Ok(models) => Json(CatalogResponse {
            models: models
                .into_iter()
                .map(|name| LlmModelEntryOwned {
                    name,
                    supports_tools: true,
                })
                .collect(),
            source: "live",
            error: None,
        })
        .into_response(),
        Err(e) => Json(CatalogResponse {
            models: curated_to_owned(CURATED_GEMINI),
            source: "fallback",
            error: Some(e.to_string()),
        })
        .into_response(),
    }
}

/// Ollama URL probe query: the Settings UI passes the user's typed
/// URL so the dropdown can refresh against a remote daemon without
/// persisting the URL first. Empty / absent falls through to the
/// same precedence chan-llm uses at request time
/// (env OLLAMA_HOST > config > hardcoded default).
#[derive(Deserialize)]
pub struct OllamaModelsQuery {
    #[serde(default)]
    url: Option<String>,
}

pub async fn api_llm_ollama_models(
    State(state): State<Arc<AppState>>,
    Query(q): Query<OllamaModelsQuery>,
) -> Response {
    let cfg = state.llm_config.lock().unwrap().clone();
    // Resolution mirrors backends::build's Ollama branch:
    //   1. ?url= query (the user's typed value in Settings)
    //   2. OLLAMA_HOST env (per-shell override)
    //   3. config.urls.ollama (Settings UI persistence)
    //   4. hardcoded default
    let url = q
        .url
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("OLLAMA_HOST").ok().filter(|s| !s.is_empty()))
        .or_else(|| cfg.urls.ollama.clone())
        .unwrap_or_else(|| chan_llm::backends::ollama::DEFAULT_URL.to_string());
    match chan_llm::backends::ollama::list_models(&url).await {
        Ok(models) => Json(
            models
                .into_iter()
                .map(|name| LlmModelEntryOwned {
                    name,
                    supports_tools: true,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        // The frontend types ollamaModels as `LlmModelEntry[]` (no
        // wrapper) and treats request errors as "daemon unreachable".
        // Surface a 503 so the Settings UI's catch arm fires the
        // standard error toast with the upstream message.
        Err(e) => err(StatusCode::SERVICE_UNAVAILABLE, e.to_string()),
    }
}
