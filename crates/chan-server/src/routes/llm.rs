//! `/api/llm/*` — assistant routes.
//!
//! Routes wrap chan_llm::LlmSession. Streaming events flow over the
//! shared /ws so the frontend has one socket to read from. The route
//! surface stays valid even when chan-llm's backends are stubs:
//! complete() emits an immediate llm.error + llm.done frame for the
//! configured backend.

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_llm::{
    apply_resume, is_pending_placeholder, BackendKind, ImageInput as LlmImageInput, LlmSession,
    Message as LlmMessage, ResumeOutcome, Role as LlmRole, SessionListener, PENDING_STATUS,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::bus::LlmBroadcastListener;
use crate::error::err;
use crate::state::AppState;

use super::preferences::cli_detection_reason;

/// `/api/llm/status` view shape. Frontend's `LlmStatus` type is a
/// flat one-active-backend snapshot. One source of truth per
/// request: the configured backend, its effective model, and whether
/// its CLI binary is launchable.
#[derive(Serialize)]
struct LlmStatus {
    /// Frontend's display tag for the active backend.
    /// "claude_cli" | "gemini_cli" | "codex_cli".
    backend: &'static str,
    /// Effective model for the active backend (config override or
    /// the chan-llm default).
    model: Option<String>,
    /// Whether a request would succeed today (active backend
    /// configured and CLI resolves).
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

fn backend_tag(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::ClaudeCli => "claude_cli",
        BackendKind::GeminiCli => "gemini_cli",
        BackendKind::CodexCli => "codex_cli",
    }
}

pub async fn api_llm_status(State(state): State<Arc<AppState>>) -> Response {
    // Public-tunnel runs return a sealed-off status: no backend, no
    // model or readiness signal. The companion
    // `tunnel_guard::tunnel_public_guard` refuses POST
    // /api/llm/complete anyway; this redaction stops a visitor from
    // (a) discovering which provider the owner uses, and (b)
    // probing for a misconfiguration that might let the gate
    // through. The shape stays compatible: the SPA's existing
    // master-switch logic greys the assistant pill when `enabled`
    // is false, which is the same greying we want here.
    if state.tunnel_public {
        return Json(LlmStatus {
            backend: backend_tag(BackendKind::ClaudeCli),
            model: None,
            ready: false,
            reason: None,
            enabled: false,
            supports_tools: true,
        })
        .into_response();
    }
    let cfg = state.llm_config.lock().unwrap().clone();
    let active = cfg.backend.unwrap_or(BackendKind::ClaudeCli);
    let model = cfg
        .models
        .for_backend(active)
        .map(str::to_owned)
        .or_else(|| Some(active.default_model().to_string()));
    let enabled = cfg.active_backend().is_some();
    let detection = chan_llm::detect_backend_cli(active, &cfg);
    let cli_ready = detection.present();
    let cmd0 = detection.command.first().cloned().unwrap_or_default();
    let ready = enabled && cli_ready;
    let reason = if !enabled {
        Some("no backend selected; pick one in Settings".to_string())
    } else if !cli_ready {
        Some(format!(
            "`{cmd0}` not found or rejected. Install the {} CLI, or set its cmd in llm.toml.",
            backend_tag(active),
        ))
    } else {
        None
    };
    Json(LlmStatus {
        backend: backend_tag(active),
        model,
        ready,
        reason,
        enabled,
        supports_tools: true,
    })
    .into_response()
}

#[derive(Serialize)]
struct CliDetectionResponse {
    detections: Vec<CliDetectionView>,
}

#[derive(Serialize)]
struct CliDetectionView {
    backend: &'static str,
    ready: bool,
    command: Vec<String>,
    reason: Option<String>,
}

fn cli_detection_view(detection: chan_llm::CliDetection) -> CliDetectionView {
    let backend = detection.backend;
    let ready = detection.present();
    let reason = (!ready).then(|| cli_detection_reason(backend, &detection));
    CliDetectionView {
        backend: backend_tag(backend),
        ready,
        command: detection.command,
        reason,
    }
}

fn sealed_cli_detection(kind: BackendKind) -> CliDetectionView {
    CliDetectionView {
        backend: backend_tag(kind),
        ready: false,
        command: vec![default_cli_command(kind).to_string()],
        reason: None,
    }
}

fn default_cli_command(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::ClaudeCli => "claude",
        BackendKind::GeminiCli => "gemini",
        BackendKind::CodexCli => "codex",
    }
}

pub async fn api_llm_cli_detection(State(state): State<Arc<AppState>>) -> Response {
    if state.tunnel_public {
        return Json(CliDetectionResponse {
            detections: [
                BackendKind::ClaudeCli,
                BackendKind::GeminiCli,
                BackendKind::CodexCli,
            ]
            .into_iter()
            .map(sealed_cli_detection)
            .collect(),
        })
        .into_response();
    }
    let cfg = state.llm_config.lock().unwrap().clone();
    Json(CliDetectionResponse {
        detections: chan_llm::detect_all(&cfg)
            .into_iter()
            .map(cli_detection_view)
            .collect(),
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
    /// Per-turn auto-apply override. The composer toggle (next to
    /// Send) supplies this on every request. When Some, both the
    /// per-request config clone (used by in-process backends) and
    /// the live AppState config (read by the MCP bridge at connect
    /// time) are updated so claude_cli / gemini_cli subprocesses
    /// see the same value via their MCP child. None preserves the
    /// existing config (legacy callers without the toggle).
    #[serde(default)]
    auto_apply_writes: Option<bool>,
}

#[derive(Deserialize)]
struct ApiMessage {
    role: ApiRole,
    content: String,
    #[serde(default)]
    tool_call_id: Option<String>,
    /// Tool calls on `assistant` messages. The frontend's
    /// `LlmMessage` carries these on every assistant turn that
    /// invoked a tool; the resume route needs them to find the
    /// turn that proposed a paused `write_file`. Dropping them
    /// silently broke `apply_resume`'s placeholder lookup.
    #[serde(default)]
    tool_calls: Vec<ApiToolCall>,
    /// Optional multimodal payload from the frontend. Each entry
    /// is a base64-encoded image with its MIME type; chan-llm
    /// forwards these to the active backend (Anthropic image
    /// content block, Gemini inline_data, Ollama images array).
    /// We don't validate here — the model rejects oversized /
    /// unsupported MIMEs with a 400 the host bubbles back.
    #[serde(default)]
    images: Vec<ApiImageInput>,
}

#[derive(Deserialize)]
struct ApiToolCall {
    id: String,
    name: String,
    /// Renamed at the wire seam: the frontend uses Anthropic-style
    /// `input` while chan-llm's `ToolCall` uses `args`. Same JSON
    /// payload, different field name.
    #[serde(default)]
    input: serde_json::Value,
}

#[derive(Deserialize)]
struct ApiImageInput {
    mime_type: String,
    data: String,
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
            tool_calls: m
                .tool_calls
                .into_iter()
                .map(|c| chan_llm::ToolCall {
                    id: c.id,
                    name: c.name,
                    args: c.input,
                })
                .collect(),
            images: m
                .images
                .into_iter()
                .map(|img| LlmImageInput {
                    mime_type: img.mime_type,
                    data: img.data,
                })
                .collect(),
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
    /// Canonical post-turn transcript captured via
    /// `SessionListener::on_messages_snapshot`. Present only for
    /// successful terminations (EndOfTurn / ToolUse); None on
    /// cancel / error / max-iter. /api/llm/resume uses this for the
    /// outbound `messages` field so direct backends' continuation
    /// turns (new assistant text, tool_use, tool_result) land in
    /// the client's history. Without it, a follow-up Apply on a
    /// continuation-emitted edit fails with "tool call not found
    /// on Assistant turn".
    messages: Option<Vec<LlmMessage>>,
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
                messages: None,
            }),
            done: tokio::sync::Notify::new(),
        }
    }
}

impl SessionListener for CollectListener {
    fn on_status(&self, status: chan_llm::AgentStatus) {
        self.forward.on_status(status);
    }
    fn on_activity(&self, activity: chan_llm::AgentActivity) {
        self.forward.on_activity(activity);
    }
    fn on_user_request(&self, request: chan_llm::UserRequest) {
        self.forward.on_user_request(request);
    }
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
    fn on_messages_snapshot(&self, history: &[LlmMessage]) {
        // chan-llm fires this before on_done for successful
        // terminations. Stash the canonical transcript so the resume
        // handler can echo it back to the client instead of the
        // pre-continuation snapshot.
        self.state.lock().expect("collect state poisoned").messages = Some(history.to_vec());
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
    // Apply the per-turn auto-apply override (composer toggle) to
    // the live config BEFORE cloning, so the MCP bridge closure
    // (which reads from this same Mutex on each new connection) and
    // the per-request config clone agree. Single-user, single-machine
    // invariant: concurrent requests with different overrides aren't
    // a concern.
    if let Some(override_) = body.auto_apply_writes {
        state.llm_config.lock().unwrap().auto_apply_writes = override_;
    }
    let mut config = state.llm_config.lock().unwrap().clone();
    // Active backend determines the model echoed back in the
    // response. Falls through the same way /api/llm/status does
    // (config override > backend default).
    let active = config.backend.unwrap_or(BackendKind::ClaudeCli);
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
        BackendKind::CodexCli => {
            if let Some(cmd) = mcp_subcommand_for(socket) {
                config.codex_cli.mcp_command = Some(cmd);
            }
        }
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

/// Body for `POST /api/llm/resume`. Replaces the chan-llm
/// placeholder for a paused `write_file` tool call with the user's
/// decision and then resumes the conversation by re-sending the
/// updated transcript to the active backend. Returns the model's
/// next turn AND the updated message list so the frontend can
/// replace its local `conv.messages` with the server-canonical
/// version (placeholder swapped, continuation appended).
#[derive(Deserialize)]
pub struct ResumeBody {
    /// Optional WS correlation id, same shape as `CompleteBody`.
    #[serde(default)]
    session_id: Option<String>,
    /// The paused tool call's id. Must match a `tool_use` block in
    /// the most recent Assistant turn of `messages`.
    call_id: String,
    /// Full conversation transcript up to the paused turn.
    /// May or may not already include the chan-llm `PENDING_STATUS`
    /// placeholder for `call_id`; we insert one when absent.
    messages: Vec<ApiMessage>,
    /// What the user did at the confirmation UI.
    outcome: ResumeOutcomeBody,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ResumeOutcomeBody {
    /// User accepted unchanged: chan-llm runs the tool with
    /// `auto_apply_writes` forced on for this single execution
    /// (via `LlmSession::approve_pending`), so the standard
    /// sandbox writes the file and the canonical result JSON
    /// lands in history.
    Apply,
    /// User accepted but supplied edited content via the diff
    /// review surface. We bypass `approve_pending` (which uses
    /// the model's original args) and call `apply_resume` with
    /// the user's `path` / `content` instead.
    ApplyAs { path: String, content: String },
    /// User rejected. Optional reason surfaces to the model so it
    /// can adapt its plan.
    Discard {
        #[serde(default)]
        reason: Option<String>,
    },
}

/// Response shape for `/api/llm/resume`. Same `content` / `tool_calls`
/// / `stop_reason` / `model` as `/complete`, plus `messages`: the
/// chan-llm-canonical history after the placeholder swap. The
/// frontend writes `conv.messages = response.messages` and appends
/// the response's `content` as a new assistant turn.
#[derive(Serialize)]
struct LlmResumeResponse {
    content: String,
    tool_calls: Vec<LlmToolCallView>,
    stop_reason: &'static str,
    model: String,
    messages: Vec<ApiMessageOut>,
}

/// Mirror of chan-llm's `Message` for the outbound side. We can't
/// re-serialize `LlmMessage` directly because its Rust field shape
/// (`tool_calls: Vec<ToolCall>` etc.) doesn't match the simpler
/// frontend `LlmMessage` exactly. Keeping a wire DTO sidesteps
/// future field-by-field drift.
#[derive(Serialize)]
struct ApiMessageOut {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<LlmToolCallView>,
}

fn role_tag(r: LlmRole) -> &'static str {
    match r {
        LlmRole::System => "system",
        LlmRole::User => "user",
        LlmRole::Assistant => "assistant",
        LlmRole::Tool => "tool",
    }
}

fn message_to_out(m: &LlmMessage) -> ApiMessageOut {
    ApiMessageOut {
        role: role_tag(m.role).to_string(),
        content: m.content.clone(),
        tool_call_id: m.tool_call_id.clone(),
        tool_calls: m
            .tool_calls
            .iter()
            .map(|c| LlmToolCallView {
                id: c.id.clone(),
                name: c.name.clone(),
                input: c.args.clone(),
            })
            .collect(),
    }
}

/// Find the position right after the Assistant turn that carries
/// `call_id`. Used to anchor a fresh PENDING_STATUS placeholder
/// when the frontend's transcript doesn't already have one.
fn assistant_with_call_index(history: &[LlmMessage], call_id: &str) -> Option<usize> {
    history
        .iter()
        .rposition(|m| m.role == LlmRole::Assistant && m.tool_calls.iter().any(|c| c.id == call_id))
}

/// True when `history` already has the chan-llm placeholder Tool
/// message for `call_id`. The frontend may have inserted one to
/// pre-validate the transcript shape; we don't duplicate it.
fn has_pending_placeholder_for(history: &[LlmMessage], call_id: &str) -> bool {
    history
        .iter()
        .any(|m| m.tool_call_id.as_deref() == Some(call_id) && is_pending_placeholder(m))
}

/// Ensure the placeholder Tool message exists for `call_id` so
/// `apply_resume` (which expects one) can find and replace it.
/// Inserts immediately after the Assistant turn that proposed the
/// call so Anthropic / Gemini's strict pairing stays valid.
fn ensure_pending_placeholder(history: &mut Vec<LlmMessage>, call_id: &str) -> Result<(), String> {
    if has_pending_placeholder_for(history, call_id) {
        return Ok(());
    }
    let idx = assistant_with_call_index(history, call_id)
        .ok_or_else(|| format!("no Assistant turn carries tool call {call_id}"))?;
    let call = history[idx]
        .tool_calls
        .iter()
        .find(|c| c.id == call_id)
        .cloned()
        .ok_or_else(|| format!("tool call {call_id} not found on Assistant turn"))?;
    let placeholder = LlmMessage::tool(
        call_id.to_string(),
        serde_json::json!({
            "status": PENDING_STATUS,
            "tool": call.name,
        })
        .to_string(),
    );
    history.insert(idx + 1, placeholder);
    Ok(())
}

pub async fn api_llm_resume(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResumeBody>,
) -> Response {
    let mut config = state.llm_config.lock().unwrap().clone();
    let active = config.backend.unwrap_or(BackendKind::ClaudeCli);
    let model = config
        .models
        .for_backend(active)
        .map(str::to_owned)
        .unwrap_or_else(|| active.default_model().to_string());
    let session_id = body.session_id.clone().unwrap_or_else(random_session_id);

    // Mirror the MCP wiring /complete uses so the agentic CLIs stay
    // sandboxed. The pause/resume contract differs by backend:
    //
    //   - Direct backends (Anthropic / Gemini / Ollama): chan-llm
    //     paused with a Pending placeholder; apply_resume swaps it,
    //     then session.send below continues the loop so the model
    //     reacts to the resolved tool_result.
    //   - CLI backends (ClaudeCli / GeminiCli, v2 MCP): the CLI has
    //     already exited (it got the deferred error back from the
    //     MCP server and emitted a closing assistant turn). There's
    //     no in-process session to continue; Apply / Discard is a
    //     pure host-side operation that mutates the placeholder.
    //     We skip the continuation explicitly below.
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
        BackendKind::CodexCli => {
            if let Some(cmd) = mcp_subcommand_for(socket) {
                config.codex_cli.mcp_command = Some(cmd);
            }
        }
    }

    let drive = state.drive().clone();
    let session = LlmSession::new(drive.clone(), config);

    let mut messages: Vec<LlmMessage> = body.messages.into_iter().map(LlmMessage::from).collect();
    if let Err(e) = ensure_pending_placeholder(&mut messages, &body.call_id) {
        return err(StatusCode::BAD_REQUEST, e);
    }

    let resolved = match body.outcome {
        ResumeOutcomeBody::Apply => session
            .approve_pending(messages, &body.call_id)
            .map_err(|e| e.to_string()),
        ResumeOutcomeBody::ApplyAs { path, content } => {
            // User-edited diff: write through chan-drive directly,
            // then synthesize the canonical Applied result so the
            // model sees the same shape as the unmodified-Apply
            // path. Failures route through ResumeOutcome::Failed
            // so the model can adapt.
            let outcome = match drive.write_text(&path, &content) {
                Ok(()) => {
                    // chan-drive's write_text returns (); fetch the
                    // post-write mtime via stat so the model gets a
                    // canonical mtime_ns it can pair against a future
                    // read_file. Stat failure isn't fatal: we still
                    // surface Applied without mtime_ns.
                    let mtime_ns = drive.stat(&path).ok().map(|s| s.mtime_ns);
                    ResumeOutcome::Applied(serde_json::json!({
                        "path": path,
                        "bytes_written": content.len() as u64,
                        "mtime_ns": mtime_ns,
                    }))
                }
                Err(e) => ResumeOutcome::Failed {
                    error: e.to_string(),
                },
            };
            apply_resume(messages, &body.call_id, outcome).map_err(|e| e.to_string())
        }
        ResumeOutcomeBody::Discard { reason } => {
            apply_resume(messages, &body.call_id, ResumeOutcome::Rejected { reason })
                .map_err(|e| e.to_string())
        }
    };
    let resumed = match resolved {
        Ok(v) => v,
        Err(e) => return err(StatusCode::BAD_REQUEST, e),
    };

    let messages_for_response: Vec<ApiMessageOut> = resumed.iter().map(message_to_out).collect();

    // CLI backends don't pause-and-resume: the agent already ran to
    // completion in /complete (claude-cli / gemini-cli got the
    // deferred tool error back from the MCP server, generated a
    // closing assistant turn, and exited). The host-side Apply /
    // Discard mutates the placeholder in-place; there's no
    // chan-llm session to continue. Re-spawning the CLI here
    // re-feeds it the original user prompt as labelled prose,
    // which it dutifully re-answers — usually by proposing the
    // exact same write again. Skip the continuation for these
    // backends and return the swapped history as-is. Direct
    // backends (Anthropic / Gemini / Ollama) still need the
    // continuation: chan-llm orchestrates their tool loop and the
    // model expects to react to the tool_result it just received.
    let skip_continuation = matches!(
        active,
        BackendKind::ClaudeCli | BackendKind::GeminiCli | BackendKind::CodexCli
    );
    if skip_continuation {
        return Json(LlmResumeResponse {
            content: String::new(),
            tool_calls: Vec::new(),
            stop_reason: stop_reason_tag(chan_llm::StopReason::EndOfTurn),
            model,
            messages: messages_for_response,
        })
        .into_response();
    }

    let collect = Arc::new(CollectListener::new(LlmBroadcastListener {
        tx: state.events_tx.clone(),
        session_id,
    }));
    let listener: Arc<dyn SessionListener> = collect.clone();
    session.send(resumed, listener);
    collect.done.notified().await;

    let snapshot = collect.state.lock().expect("collect state poisoned");
    if let Some(err_msg) = snapshot.error.clone() {
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
    // Prefer the post-continuation snapshot from chan-llm. It
    // includes the continuation's assistant turn (text +
    // tool_use) and any tool_result messages chan-llm appended,
    // so a follow-up Apply on a continuation-emitted edit can
    // find its assistant turn via ensure_pending_placeholder.
    // Fall back to the pre-continuation history when the snapshot
    // hook didn't fire (cancel / error / max-iter): the response
    // still surfaces the placeholder swap that already happened.
    //
    // chan-llm prepends a System message on entry to `send`; the
    // client never had one and round-tripping it would break the
    // resume's strict-equality reuse, so strip it before
    // serializing.
    let messages_out = snapshot
        .messages
        .as_ref()
        .map(|h| {
            h.iter()
                .filter(|m| m.role != LlmRole::System)
                .map(message_to_out)
                .collect::<Vec<_>>()
        })
        .unwrap_or(messages_for_response);
    Json(LlmResumeResponse {
        content: snapshot.text.clone(),
        tool_calls,
        stop_reason: stop_reason_tag(stop),
        model,
        messages: messages_out,
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

#[cfg(test)]
mod collect_listener_tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;
    use tokio::sync::broadcast;

    fn listener() -> (CollectListener, broadcast::Receiver<String>) {
        let (tx, rx) = broadcast::channel(8);
        (
            CollectListener::new(LlmBroadcastListener {
                tx,
                session_id: "collect-session".to_string(),
            }),
            rx,
        )
    }

    fn recv_json(rx: &mut broadcast::Receiver<String>) -> Value {
        let raw = rx.try_recv().expect("broadcast frame");
        serde_json::from_str(&raw).expect("json frame")
    }

    #[test]
    fn forwards_status_activity_and_user_request() {
        let (listener, mut rx) = listener();

        let status = chan_llm::AgentStatus::Heartbeat {
            backend: "claude_cli".into(),
            idle_ms: 1500,
        };
        listener.on_status(status.clone());
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.status");
        assert_eq!(frame["session_id"], "collect-session");
        assert_eq!(
            frame["status"],
            serde_json::to_value(status).expect("status value")
        );

        let activity = chan_llm::AgentActivity::AgentNote {
            backend: "claude_cli".into(),
            text: "working".into(),
            parent_id: None,
        };
        listener.on_activity(activity.clone());
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.activity");
        assert_eq!(
            frame["activity"],
            serde_json::to_value(activity).expect("activity value")
        );

        let request = chan_llm::UserRequest::Survey {
            backend: "claude_cli".into(),
            id: "survey_1".into(),
            questions: vec![chan_llm::UserQuestion {
                question: "Choose one".into(),
                header: None,
                multi_select: false,
                options: vec![chan_llm::UserOption {
                    label: "Continue".into(),
                    description: None,
                }],
            }],
            parent_id: None,
        };
        listener.on_user_request(request.clone());
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.user_request");
        assert_eq!(
            frame["request"],
            serde_json::to_value(request).expect("request value")
        );
    }

    #[test]
    fn delta_updates_text_and_forwards() {
        let (listener, mut rx) = listener();

        listener.on_delta(chan_llm::Delta {
            text: "hello".into(),
        });

        assert_eq!(
            listener.state.lock().expect("collect state poisoned").text,
            "hello"
        );
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.delta");
        assert_eq!(frame["text"], "hello");
    }

    #[test]
    fn tool_call_collects_and_forwards() {
        let (listener, mut rx) = listener();
        let call = chan_llm::ToolCall {
            id: "call_1".into(),
            name: "read_file".into(),
            args: serde_json::json!({"path": "a.md"}),
        };

        listener.on_tool_call(call.clone());

        assert_eq!(
            listener
                .state
                .lock()
                .expect("collect state poisoned")
                .tool_calls
                .len(),
            1
        );
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.tool_call");
        assert_eq!(
            frame["call"],
            serde_json::to_value(call).expect("call value")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn done_updates_state_notifies_and_forwards() {
        let (listener, mut rx) = listener();
        let notified = listener.done.notified();
        tokio::pin!(notified);

        listener.on_done(chan_llm::StopReason::ToolUse);

        {
            let state = listener.state.lock().expect("collect state poisoned");
            assert!(state.finished);
            assert_eq!(state.stop_reason, Some(chan_llm::StopReason::ToolUse));
        }
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), notified)
                .await
                .is_ok()
        );
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.done");
        assert_eq!(frame["reason"], "tool_use");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cli_detection_public_tunnel_shape_has_three_backends() {
        let state = crate::state::test_support::make_test_state(true, true);

        let response = api_llm_cli_detection(State(state)).await;
        let (parts, body) = response.into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        let bytes = to_bytes(body, 8192).await.expect("read body");
        let json: Value = serde_json::from_slice(&bytes).expect("json body");

        let detections = json["detections"].as_array().expect("detections array");
        assert_eq!(detections.len(), 3);
        assert_eq!(detections[0]["backend"], "claude_cli");
        assert_eq!(detections[1]["backend"], "gemini_cli");
        assert_eq!(detections[2]["backend"], "codex_cli");
        assert!(detections.iter().all(|d| d["ready"] == false));
        assert!(detections.iter().all(|d| d["reason"].is_null()));
    }
}
