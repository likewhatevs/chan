// LlmSession: the public handle the assistant operates through.
//
// Designed callback-first so uniffi can wrap it cleanly later. The
// caller (chan-server, a future native shell, the CLI) implements
// `SessionListener` and hands an `Arc` to `LlmSession::send`. The
// session drives the HTTP stream on an internal tokio runtime and
// dispatches into the listener as deltas, tool calls, and the
// final stop reason arrive.
//
// Async stays inside. Public methods don't return `Future`; they
// kick off background work and return immediately. This is the
// same pattern `chan_drive::Drive::watch` uses, for the same reason:
// a foreign-language consumer shouldn't have to negotiate an async
// runtime across the FFI boundary.
//
// Tool-call orchestration: this commit is text-only. When the
// follow-up commit lands tool round-trips, `send` will spawn a
// loop that drives backend.run() -> on_tool_call -> wait for
// host-supplied result -> next backend.run() until a non-tool
// stop reason. The loop lives in this module; backends just
// translate one HTTP exchange.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use chan_drive::Drive;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use tokio::runtime::Runtime;

use crate::backends::{self, BackendKind};
use crate::config::LlmConfig;
use crate::error::LlmError;
use crate::tools::ToolContext;

/// Conversation roles. The taxonomy mirrors common LLM chat
/// conventions so backends don't have to invent their own; each
/// translates these to its wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// One turn in the conversation. The host (chan-server, native
/// shell) owns the history and passes the full transcript on each
/// `send` call. Stateless on the chan-llm side keeps the FFI
/// surface simple: no hidden state to synchronize across the
/// boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    /// Set on `Role::Tool` messages to identify which tool call
    /// this is the result of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Set on `Role::Assistant` messages that include tool calls.
    /// Mirrors the previous turn's tool calls so the assistant
    /// can reference them across the conversation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Optional multimodal payload: base64-encoded images that
    /// accompany this message. Today only `Role::User` messages
    /// carry them. Backends without image support drop them
    /// silently; the text content is still authoritative.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<ImageInput>,
}

/// One base64-encoded image attached to a `Message`. `mime_type`
/// is the standard MIME (`image/png`, `image/jpeg`, etc.); `data`
/// is the raw base64 payload WITHOUT a `data:` URI prefix. Hosts
/// (chan-server, native shells) are responsible for capping size
/// before they construct one; chan-llm trusts the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInput {
    pub mime_type: String,
    pub data: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            images: Vec::new(),
        }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            images: Vec::new(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            images: Vec::new(),
        }
    }
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: Vec::new(),
            images: Vec::new(),
        }
    }
}

/// Streaming text delta. Backends emit these as they receive
/// SSE / streaming JSON chunks from the upstream model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub text: String,
}

/// One tool call the assistant proposes during generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Backend-assigned id. Surfaced so multiple parallel tool
    /// calls can be matched to their results.
    pub id: String,
    pub name: String,
    pub args: Json,
}

/// Result of executing a tool the assistant requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub id: String,
    pub output: Json,
}

/// Status string the orchestrator writes into the placeholder Tool
/// message when a `write_file` call pauses for user approval. Hosts
/// match against this (or use `is_pending_placeholder`) to drive
/// the confirmation UI without coupling to the full JSON shape.
pub const PENDING_STATUS: &str = "awaiting_user_approval";

/// Status string `apply_resume` writes into the Tool message when
/// the host reports that the user rejected a paused write.
pub const REJECTED_STATUS: &str = "rejected_by_user";

/// Status string `apply_resume` writes when the user approved but
/// applying the call failed for an external reason (disk full,
/// write conflict).
pub const FAILED_STATUS: &str = "applied_but_failed";

/// Outcome of a paused tool call after the host's confirmation UI
/// resolves it. Pass to `apply_resume` to swap the orchestrator's
/// placeholder for the typed result before re-sending the
/// transcript via `LlmSession::send`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeOutcome {
    /// User approved and the host (or `LlmSession::approve_pending`)
    /// applied the call. `output` is the JSON the tool would have
    /// returned; for `write_file` via the standard sandbox this is
    /// the `{"path":..., "bytes_written":..., "mtime_ns":...}` shape.
    Applied(Json),
    /// User rejected at the confirmation UI. Optional `reason` is
    /// surfaced to the assistant so it can adapt its plan.
    Rejected { reason: Option<String> },
    /// User approved but applying the call failed (e.g. disk full,
    /// write conflict). The assistant sees this as a structured
    /// error and can retry or hand back to the user.
    Failed { error: String },
}

/// True if `msg` is the orchestrator's placeholder Tool message
/// for a paused write. Hosts use this to find which call needs
/// confirmation without parsing JSON internals themselves.
pub fn is_pending_placeholder(msg: &Message) -> bool {
    if msg.role != Role::Tool {
        return false;
    }
    let parsed: serde_json::Value = match serde_json::from_str(&msg.content) {
        Ok(v) => v,
        Err(_) => return false,
    };
    parsed.get("status").and_then(|s| s.as_str()) == Some(PENDING_STATUS)
}

/// Replace the orchestrator's placeholder Tool message for
/// `call_id` with `outcome`'s typed serialization, then return the
/// transcript ready to pass back to `LlmSession::send`.
///
/// Errors when the matching message isn't actually a placeholder,
/// so a host that double-resumes the same call (or targets the
/// wrong id) catches the mistake instead of silently corrupting
/// the transcript fed back to the model.
pub fn apply_resume(
    mut history: Vec<Message>,
    call_id: &str,
    outcome: ResumeOutcome,
) -> Result<Vec<Message>, LlmError> {
    let idx = history
        .iter()
        .rposition(|m| m.role == Role::Tool && m.tool_call_id.as_deref() == Some(call_id))
        .ok_or_else(|| LlmError::Resume(format!("no Tool message with id {call_id}")))?;
    if !is_pending_placeholder(&history[idx]) {
        return Err(LlmError::Resume(format!(
            "Tool message {call_id} is not a pending placeholder; refusing to overwrite",
        )));
    }
    let body = match outcome {
        ResumeOutcome::Applied(json) => serde_json::to_string(&json),
        ResumeOutcome::Rejected { reason } => serde_json::to_string(&serde_json::json!({
            "status": REJECTED_STATUS,
            "reason": reason,
        })),
        ResumeOutcome::Failed { error } => serde_json::to_string(&serde_json::json!({
            "status": FAILED_STATUS,
            "error": error,
        })),
    }
    .map_err(|e| LlmError::Resume(format!("encode resume body: {e}")))?;
    history[idx].content = body;
    Ok(history)
}

/// Walk `history` newest-first for an Assistant turn carrying a
/// tool call with the given id. Used by `LlmSession::approve_pending`
/// to recover the original args after the host's confirmation UI
/// resolves a paused write.
fn find_tool_call_in_history(history: &[Message], call_id: &str) -> Result<ToolCall, LlmError> {
    for msg in history.iter().rev() {
        if msg.role != Role::Assistant {
            continue;
        }
        if let Some(call) = msg.tool_calls.iter().find(|c| c.id == call_id) {
            return Ok(call.clone());
        }
    }
    Err(LlmError::Resume(format!(
        "no Assistant turn carries tool call {call_id}",
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndOfTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    Error,
    /// The host called `CancelHandle::cancel`. The orchestrator and
    /// the in-flight backend stop at the next checkpoint and emit
    /// `on_done(Cancelled)`. The transcript may carry a partial
    /// assistant turn; the host decides whether to keep it.
    Cancelled,
}

/// Handle returned by `LlmSession::send`. Call `cancel()` to stop
/// the in-flight session at the next checkpoint (between SSE/NDJSON
/// chunks, between tool iterations, between subprocess reads).
/// Cheap to clone: a single `Arc<AtomicBool>` under the hood.
#[derive(Clone, Debug)]
pub struct CancelHandle(Arc<AtomicBool>);

impl CancelHandle {
    fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub(crate) fn flag(&self) -> Arc<AtomicBool> {
        self.0.clone()
    }
}

impl Default for CancelHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed companion to the free-form `on_error(String)` callback.
/// Hosts that want to drive UX off the error category (retry button,
/// settings link, auth prompt) override `on_error_kind` and branch
/// on the variant; hosts that only want a human-readable string can
/// keep using `on_error` and inherit the default bridge that calls
/// `Display` on the variant.
///
/// Variants carry only primitive payloads (owned `String`, `u16`,
/// `u64`) so the enum survives a uniffi boundary. `#[non_exhaustive]`
/// keeps adding a new variant from being a breaking change.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LlmEventError {
    /// Upstream or CLI rejected the credential. The user needs to
    /// fix the CLI/provider auth.
    Auth { backend: String, message: String },
    /// Upstream rate-limited and the retry budget was exhausted.
    /// `retry_after_secs` carries the upstream hint when one was
    /// supplied, `None` otherwise; hosts can use it to schedule
    /// the next attempt.
    RateLimited {
        backend: String,
        retry_after_secs: Option<u64>,
        message: String,
    },
    /// Could not reach the upstream service (DNS / connect / TLS).
    /// Distinct from `Backend` because the host should show a
    /// "check your network" affordance rather than blaming the
    /// provider.
    BackendUnreachable { backend: String, message: String },
    /// Upstream returned a non-retryable 4xx other than 401/403/429
    /// (typically 400: bad model name, malformed body).
    BadRequest { backend: String, message: String },
    /// Upstream returned a 5xx after the retry budget was exhausted,
    /// or any other backend-level failure tied to a status code.
    Backend {
        backend: String,
        status: u16,
        message: String,
    },
    /// Subprocess failed to spawn (ENOENT / EPERM). Hosts can
    /// surface a "install the CLI" affordance distinct from a
    /// generic backend error.
    SpawnFailed { backend: String, message: String },
    /// Connection or pipe dropped mid-stream after some bytes were
    /// already emitted. The transcript may be incomplete; retrying
    /// is unsafe because the upstream may have already accounted for
    /// the request.
    StreamTruncated { backend: String, message: String },
    /// Inactivity / read timeout. The upstream went silent for
    /// longer than the configured budget; child or connection has
    /// been torn down.
    Timeout { backend: String, message: String },
    /// Could not parse a streaming event payload. Counted against
    /// `PARSE_ERROR_EMIT_LIMIT` per turn; the host shouldn't be
    /// alarmed by a single occurrence.
    ParseError { backend: String, message: String },
    /// Cancel handle flipped. Distinct from a Cancelled stop reason
    /// because `on_done(Cancelled)` still fires after this event;
    /// `on_error_kind` may also fire when the cancel triggered a
    /// cleanup error worth surfacing.
    Cancelled { backend: String },
    /// Catch-all for failures that don't fit a typed variant. Hosts
    /// fall back to substring-matching the message string only when
    /// this variant fires.
    Other { backend: String, message: String },
}

impl LlmEventError {
    /// Backend name the error is attributed to (`claude_cli`,
    /// `gemini_cli`, `codex_cli`, or any other identifier a backend
    /// supplies). Useful for logs and per-backend UX.
    pub fn backend(&self) -> &str {
        match self {
            LlmEventError::Auth { backend, .. }
            | LlmEventError::RateLimited { backend, .. }
            | LlmEventError::BackendUnreachable { backend, .. }
            | LlmEventError::BadRequest { backend, .. }
            | LlmEventError::Backend { backend, .. }
            | LlmEventError::SpawnFailed { backend, .. }
            | LlmEventError::StreamTruncated { backend, .. }
            | LlmEventError::Timeout { backend, .. }
            | LlmEventError::ParseError { backend, .. }
            | LlmEventError::Cancelled { backend }
            | LlmEventError::Other { backend, .. } => backend,
        }
    }

    /// Short snake_case kind used for telemetry / WebSocket frame
    /// `code` fields. Stable across versions; chan-server's frontend
    /// branches on these strings to pick the retry / auth-prompt /
    /// settings-link affordance.
    pub fn code(&self) -> &'static str {
        match self {
            LlmEventError::Auth { .. } => "auth",
            LlmEventError::RateLimited { .. } => "rate_limited",
            LlmEventError::BackendUnreachable { .. } => "backend_unreachable",
            LlmEventError::BadRequest { .. } => "bad_request",
            LlmEventError::Backend { .. } => "backend",
            LlmEventError::SpawnFailed { .. } => "spawn_failed",
            LlmEventError::StreamTruncated { .. } => "stream_truncated",
            LlmEventError::Timeout { .. } => "timeout",
            LlmEventError::ParseError { .. } => "parse_error",
            LlmEventError::Cancelled { .. } => "cancelled",
            LlmEventError::Other { .. } => "other",
        }
    }
}

impl std::fmt::Display for LlmEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmEventError::Auth { backend, message } => {
                write!(f, "{backend} auth: {message}")
            }
            LlmEventError::RateLimited {
                backend,
                retry_after_secs: Some(s),
                message,
            } => write!(f, "{backend} rate limited (retry after {s}s): {message}"),
            LlmEventError::RateLimited {
                backend,
                retry_after_secs: None,
                message,
            } => write!(f, "{backend} rate limited: {message}"),
            LlmEventError::BackendUnreachable { backend, message } => {
                write!(f, "{backend} unreachable: {message}")
            }
            LlmEventError::BadRequest { backend, message } => {
                write!(f, "{backend} bad request: {message}")
            }
            LlmEventError::Backend {
                backend,
                status,
                message,
            } => write!(f, "{backend} {status}: {message}"),
            LlmEventError::SpawnFailed { backend, message } => {
                write!(f, "{backend} spawn failed: {message}")
            }
            LlmEventError::StreamTruncated { backend, message } => {
                write!(f, "{backend} stream truncated: {message}")
            }
            LlmEventError::Timeout { backend, message } => {
                write!(f, "{backend} timeout: {message}")
            }
            LlmEventError::ParseError { backend, message } => {
                write!(f, "{backend} parse error: {message}")
            }
            LlmEventError::Cancelled { backend } => {
                write!(f, "{backend} cancelled")
            }
            LlmEventError::Other { backend, message } => {
                write!(f, "{backend}: {message}")
            }
        }
    }
}

/// What the consumer implements. `Send + Sync` because events
/// arrive on the runtime's worker threads.
pub trait SessionListener: Send + Sync {
    fn on_delta(&self, delta: Delta);
    fn on_tool_call(&self, call: ToolCall);
    fn on_tool_result(&self, result: ToolResult);
    fn on_done(&self, reason: StopReason);
    fn on_error(&self, error: String);
    /// Typed companion to `on_error`. Backends that classify failures
    /// (HTTP status mapping, spawn errors, timeouts, parse errors)
    /// call this; hosts that override it can branch on the variant
    /// for richer UX. Default impl bridges to `on_error(String)` via
    /// the variant's `Display` impl so existing listeners keep
    /// compiling and still see every failure.
    fn on_error_kind(&self, error: LlmEventError) {
        self.on_error(error.to_string());
    }
    /// Canonical post-turn transcript. Fires immediately before
    /// `on_done` for successful terminations only (EndOfTurn and
    /// ToolUse). Carries the full history chan-llm built during the
    /// turn, including the final assistant message and any tool
    /// pairings appended by the loop.
    ///
    /// Skipped on Cancelled / Error / max-iter so hosts don't replace
    /// their pre-call transcript with a partial one. Default no-op
    /// so existing listeners keep compiling.
    fn on_messages_snapshot(&self, _history: &[Message]) {}
}

/// Internal wrapper that catches panics from a host-supplied
/// `SessionListener` so a buggy implementation can't tear down the
/// orchestrator task before `on_done` fires. Every callback runs
/// inside `catch_unwind` with `AssertUnwindSafe`; on unwind we log
/// via `tracing::error` and continue. The exact-once contract for
/// `on_done` survives a panicking host.
struct SafeListener {
    inner: Arc<dyn SessionListener>,
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic>".to_string()
    }
}

impl SessionListener for SafeListener {
    fn on_delta(&self, delta: Delta) {
        let inner = self.inner.clone();
        if let Err(p) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || inner.on_delta(delta)))
        {
            tracing::error!(panic = %panic_message(&p), "listener.on_delta panicked");
        }
    }
    fn on_tool_call(&self, call: ToolCall) {
        let inner = self.inner.clone();
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            inner.on_tool_call(call)
        })) {
            tracing::error!(panic = %panic_message(&p), "listener.on_tool_call panicked");
        }
    }
    fn on_tool_result(&self, result: ToolResult) {
        let inner = self.inner.clone();
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            inner.on_tool_result(result)
        })) {
            tracing::error!(panic = %panic_message(&p), "listener.on_tool_result panicked");
        }
    }
    fn on_done(&self, reason: StopReason) {
        let inner = self.inner.clone();
        if let Err(p) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || inner.on_done(reason)))
        {
            tracing::error!(panic = %panic_message(&p), "listener.on_done panicked");
        }
    }
    fn on_error(&self, error: String) {
        let inner = self.inner.clone();
        if let Err(p) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || inner.on_error(error)))
        {
            tracing::error!(panic = %panic_message(&p), "listener.on_error panicked");
        }
    }
    fn on_error_kind(&self, error: LlmEventError) {
        let inner = self.inner.clone();
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            inner.on_error_kind(error)
        })) {
            tracing::error!(panic = %panic_message(&p), "listener.on_error_kind panicked");
        }
    }
    fn on_messages_snapshot(&self, history: &[Message]) {
        let inner = self.inner.clone();
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            inner.on_messages_snapshot(history)
        })) {
            tracing::error!(panic = %panic_message(&p), "listener.on_messages_snapshot panicked");
        }
    }
}

pub struct LlmSession {
    drive: Arc<Drive>,
    config: LlmConfig,
}

impl LlmSession {
    pub fn new(drive: Arc<Drive>, config: LlmConfig) -> Self {
        Self { drive, config }
    }

    pub fn backend(&self) -> Option<BackendKind> {
        self.config.backend
    }

    pub fn tool_context(&self) -> ToolContext {
        ToolContext::new(self.drive.clone(), self.config.auto_apply_writes)
    }

    /// Convenience for the "user clicked Apply unchanged" path:
    /// recover the deferred call from `history`, run it through
    /// the standard tool sandbox with auto_apply forced on for
    /// this single execution, and return the transcript with the
    /// placeholder replaced by the tool's real output.
    ///
    /// The host's `auto_apply_writes` config is unchanged. The
    /// override applies only to this one call.
    ///
    /// When the host needs to mutate args before applying (e.g.
    /// the user edited the diff in the confirmation UI), it
    /// should run the write itself via
    /// `chan_drive::Drive::write_text` and call `apply_resume`
    /// directly with the resulting JSON.
    pub fn approve_pending(
        &self,
        history: Vec<Message>,
        call_id: &str,
    ) -> Result<Vec<Message>, LlmError> {
        let call = find_tool_call_in_history(&history, call_id)?;
        let ctx = ToolContext::new(self.drive.clone(), true);
        let outcome = crate::tools::execute(&call.name, &call.args, &ctx)?;
        let json = match outcome {
            crate::tools::ToolOutcome::Ok(v) => v,
            crate::tools::ToolOutcome::Pending { tool, .. } => {
                return Err(LlmError::Resume(format!(
                    "tool {tool} returned Pending despite auto_apply override; this is a bug",
                )));
            }
        };
        apply_resume(history, call_id, ResumeOutcome::Applied(json))
    }

    /// Kick off a turn. The host passes the full conversation
    /// transcript; chan-llm prepends the system prompt + tool
    /// descriptions automatically. Returns immediately after
    /// spawning the background task; events flow into the
    /// listener.
    ///
    /// Tool-call orchestration: the loop runs the backend, runs
    /// any tool calls the assistant proposed (auto-executable
    /// reads / search; pauses on un-confirmed writes when
    /// auto_apply_writes is off), appends results to the
    /// transcript, and runs the backend again. Loops until the
    /// assistant returns text only or hits the configured
    /// `max_tool_iterations` cap (defaults to
    /// `DEFAULT_MAX_TOOL_ITERATIONS`, defense against runaway
    /// loops). Override the cap via `LlmConfig::max_tool_iterations`.
    ///
    /// Resume contract for paused writes: when `auto_apply_writes`
    /// is off and the assistant proposes `write_file`, the loop
    /// pauses with `on_done(ToolUse)` AFTER pushing both the
    /// assistant's tool_use turn and a placeholder Tool message
    /// for every dangling call. The placeholder shape is
    /// `{"status":"awaiting_user_approval","tool":"<name>"}`
    /// (use the `PENDING_STATUS` constant or
    /// `is_pending_placeholder` helper to detect it without coupling
    /// to the JSON shape).
    ///
    /// To resume, the host typically:
    ///   1. Detects the placeholder via `is_pending_placeholder`.
    ///   2. Shows its confirmation UI.
    ///   3. Builds a `ResumeOutcome` (`Applied` / `Rejected` /
    ///      `Failed`) and calls `apply_resume(history, call_id,
    ///      outcome)` to swap the placeholder. Convenience
    ///      `LlmSession::approve_pending(history, call_id)` does
    ///      step 3 for the unmodified-Apply case.
    ///   4. Calls `send` again with the updated transcript.
    ///
    /// `apply_resume` validates that the target message is actually
    /// a placeholder, so a host that double-resumes catches the
    /// mistake instead of silently corrupting the transcript.
    pub fn send(&self, messages: Vec<Message>, listener: Arc<dyn SessionListener>) -> CancelHandle {
        // Wrap once at the entry point so every downstream call site
        // (run_loop dispatches, backend on_delta emissions) inherits
        // the panic guard for free. A host listener that panics in
        // any callback can't tear down the orchestrator task before
        // `on_done` fires.
        let listener: Arc<dyn SessionListener> = Arc::new(SafeListener { inner: listener });
        let cancel = CancelHandle::new();
        let Some(kind) = self.config.active_backend() else {
            // This is a "the user hasn't picked a backend" or "the
            // user has disabled the picked backend" state. The check
            // uses `active_backend` so a disabled provider can't
            // sneak a request through even if `config.backend` is
            // still set as the sticky default.
            listener.on_error(LlmError::BackendNotConfigured.to_string());
            listener.on_done(StopReason::Error);
            return cancel;
        };

        let backend = match backends::build(kind, &self.config, self.drive.root()) {
            Ok(b) => b,
            Err(e) => {
                match e {
                    LlmError::CliNotFound {
                        backend,
                        command,
                        reason,
                    } => {
                        listener.on_error_kind(LlmEventError::SpawnFailed {
                            backend,
                            message: format!("CLI unavailable: {command}: {reason}"),
                        });
                    }
                    other => listener.on_error(other.to_string()),
                }
                listener.on_done(StopReason::Error);
                return cancel;
            }
        };

        // Prepend the chan-wide system prompt unless the host
        // already provided one (a few editor surfaces want a
        // chat-only variant; passing the system message
        // explicitly overrides the default).
        let mut history: Vec<Message> = Vec::with_capacity(messages.len() + 1);
        if !messages.first().is_some_and(|m| m.role == Role::System) {
            history.push(Message::system(crate::prompts::SYSTEM_PROMPT));
        }
        history.extend(messages);

        // The agentic CLIs (ClaudeCli, GeminiCli, CodexCli) run as full agents
        // in both modes. The chan-llm orchestration loop never
        // executes tool calls for them (the CLI does its own), so
        // we always pass empty schemas. The auto-apply story
        // differs by mode:
        //
        //   - v1 (mcp_command = None): the CLI writes through its
        //     own native tools, bypassing chan-llm's gate entirely.
        //     We force-enable auto_apply so the (unused) ToolContext
        //     reflects the contract gap honestly.
        //   - v2 (mcp_command = Some): writes flow through the
        //     chan-llm MCP subprocess, which applies the user's
        //     auto_apply_writes flag itself. The orchestrator's
        //     ToolContext is irrelevant here too.
        let agentic_cli_v1 = match kind {
            BackendKind::ClaudeCli => self.config.claude_cli.mcp_command.is_none(),
            BackendKind::GeminiCli => self.config.gemini_cli.mcp_command.is_none(),
            BackendKind::CodexCli => self.config.codex_cli.mcp_command.is_none(),
        };
        let tool_ctx = if agentic_cli_v1 {
            crate::tools::ToolContext::new(self.drive.clone(), true)
        } else {
            self.tool_context()
        };
        let tool_schemas = Vec::new();

        // Resolve the per-call tool-iteration cap. Clamps zero to one
        // so a misconfigured value can't deadlock the orchestrator.
        let max_iter = self
            .config
            .max_tool_iterations
            .map(|n| n.max(1) as usize)
            .unwrap_or(DEFAULT_MAX_TOOL_ITERATIONS);

        let cancel_inner = cancel.flag();
        spawn(async move {
            run_loop(
                backend,
                history,
                tool_schemas,
                tool_ctx,
                listener,
                cancel_inner,
                max_iter,
            )
            .await;
        });
        cancel
    }
}

/// Default tool-call rounds in a single `send`. Defense against
/// the assistant looping on a buggy tool call (e.g. read_file on
/// a non-existent path, then read_file on a similar non-existent
/// path, etc.). When the cap fires we emit `on_done(Error)` with
/// a clear message; the host can offer the user a "try again"
/// affordance. Overridable via `LlmConfig::max_tool_iterations`.
pub const DEFAULT_MAX_TOOL_ITERATIONS: usize = 12;

/// Drive the assistant loop with a caller-provided backend and the
/// other orchestrator inputs. Public entry point gated on the
/// `bench` feature so the `end_to_end` bench (and any future
/// integration test in a separate crate) can exercise the loop
/// against a `MockBackend` without going through `backends::build`
/// or paying real-API tokens. Hosts use `LlmSession::send`
/// instead; this is harness-only.
#[cfg(feature = "bench")]
pub async fn run_session_for_bench(
    backend: Arc<dyn backends::Backend>,
    history: Vec<Message>,
    tool_schemas: Vec<crate::tools::ToolSchema>,
    tool_ctx: crate::tools::ToolContext,
    listener: Arc<dyn SessionListener>,
    cancel: Arc<AtomicBool>,
    max_iterations: usize,
) {
    let listener: Arc<dyn SessionListener> = Arc::new(SafeListener { inner: listener });
    run_loop(
        backend,
        history,
        tool_schemas,
        tool_ctx,
        listener,
        cancel,
        max_iterations,
    )
    .await;
}

/// Drive the assistant loop. Backend produces text + tool calls;
/// we run the auto-executable tools and append their results,
/// then call the backend again. Pauses (emitting `on_done(ToolUse)`)
/// when a tool returns `Pending` (write_file with auto_apply
/// off): the host shows a confirmation UI and resumes by sending
/// the next turn with the tool result message appended.
async fn run_loop(
    backend: Arc<dyn backends::Backend>,
    mut history: Vec<Message>,
    tool_schemas: Vec<crate::tools::ToolSchema>,
    tool_ctx: crate::tools::ToolContext,
    listener: Arc<dyn SessionListener>,
    cancel: Arc<AtomicBool>,
    max_iterations: usize,
) {
    for _ in 0..max_iterations {
        if cancel.load(Ordering::Relaxed) {
            listener.on_done(StopReason::Cancelled);
            return;
        }
        let mut outcome = backend
            .run(&history, &tool_schemas, listener.clone(), cancel.clone())
            .await;

        if outcome.stop_reason == StopReason::Error {
            // Backend already emitted on_error; we just close out.
            listener.on_done(StopReason::Error);
            return;
        }
        if outcome.stop_reason == StopReason::Cancelled {
            listener.on_done(StopReason::Cancelled);
            return;
        }

        if outcome.tool_calls.is_empty() {
            // Push the final assistant turn into `history` before
            // snapshotting so hosts that rely on the snapshot for
            // canonical post-turn state see the closing message.
            // The previous code never appended this because run_loop
            // returns here without another iteration; the message
            // was only surfaced via on_delta. Empty-text turns can
            // happen (tool-only model that just ran out of tools);
            // skip the push in that case to avoid a stub message.
            if !outcome.assistant_text.is_empty() {
                history.push(Message {
                    role: Role::Assistant,
                    content: std::mem::take(&mut outcome.assistant_text),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    images: Vec::new(),
                });
            }
            listener.on_messages_snapshot(&history);
            listener.on_done(outcome.stop_reason);
            return;
        }

        // Assistant proposed tool calls. Append the assistant
        // turn (with tool_calls) to history so the next backend
        // call sees the conversation including the proposed
        // calls; then run each tool, appending its result as a
        // Tool message. `mem::take` moves the outcome's strings
        // and vec into the new Message instead of cloning them;
        // outcome is dropped immediately after this loop iteration
        // so the empty placeholders left behind don't matter.
        let tool_calls = std::mem::take(&mut outcome.tool_calls);
        history.push(Message {
            role: Role::Assistant,
            content: std::mem::take(&mut outcome.assistant_text),
            tool_call_id: None,
            tool_calls: tool_calls.clone(),
            images: Vec::new(),
        });

        // Did any tool call return `Pending` (write_file with
        // auto_apply off)? Track via a plain bool because the call
        // detail was unused. We push a placeholder result for any
        // tool that returned Pending so the transcript stays
        // well-formed, then pause so the host can run its
        // confirmation UI.
        let mut paused = false;
        for call in tool_calls.into_iter() {
            if cancel.load(Ordering::Relaxed) {
                listener.on_done(StopReason::Cancelled);
                return;
            }
            listener.on_tool_call(call.clone());
            // Tool execution is sync: chan-drive's read/write/list/
            // search are blocking I/O. Without spawn_blocking, a
            // slow read or an indexer-busy search ties up the tokio
            // worker that's running this loop, starving every other
            // session sharing the runtime. Move the call to the
            // blocking pool so the worker stays free to drive
            // streams and other sessions.
            let exec_ctx = tool_ctx.clone();
            let exec_name = call.name.clone();
            let exec_args = call.args.clone();
            let panic_tool_name = call.name.clone();
            let exec_handle = tokio::task::spawn_blocking(move || {
                // catch_unwind keeps the panic payload (which can
                // contain user paths or other PII) out of the
                // model-visible tool result. The full payload and
                // backtrace land in logs via the panic hook; the
                // assistant sees a generic, scrubbed message.
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::tools::execute(&exec_name, &exec_args, &exec_ctx)
                }))
            });
            // Race the blocking task against the cancel flag. A long-
            // running tool (search over a large drive) shouldn't pin
            // the orchestrator past the user's cancel; the blocking
            // task continues to completion in the pool (tokio
            // doesn't kill spawn_blocking work on JoinHandle drop),
            // but the orchestrator stops observing it and emits
            // Cancelled within the cancel-poll interval.
            let exec_result = tokio::select! {
                biased;
                _ = wait_for_cancel(&cancel) => {
                    listener.on_done(StopReason::Cancelled);
                    return;
                }
                r = exec_handle => r,
            };
            let exec_result = match exec_result {
                Ok(Ok(r)) => r,
                Ok(Err(_panic_payload)) => {
                    tracing::error!(
                        tool = %panic_tool_name,
                        "tool panic captured; returning generic error to model",
                    );
                    Err(LlmError::Tool(format!(
                        "tool {panic_tool_name} panicked; see host logs"
                    )))
                }
                Err(join_err) => {
                    // JoinError without a payload: cancellation or
                    // runtime shutdown. Distinct from a panic; no
                    // need to scrub.
                    Err(LlmError::Tool(format!("tool join error: {join_err}")))
                }
            };
            match exec_result {
                Ok(crate::tools::ToolOutcome::Ok(result)) => {
                    listener.on_tool_result(ToolResult {
                        id: call.id.clone(),
                        output: result.clone(),
                    });
                    history.push(Message::tool(
                        call.id.clone(),
                        serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()),
                    ));
                }
                Ok(crate::tools::ToolOutcome::Pending { .. }) => {
                    // auto_apply_writes is off and the tool was
                    // write_file. Pause; the host's UI confirms
                    // and resumes by re-sending with the tool
                    // result appended. Multiple Pending in one
                    // turn is supported: each lands here, pushes
                    // its placeholder, and the pause fires after
                    // the loop completes so every tool_use block
                    // has a matching tool_result.
                    paused = true;
                    let placeholder = serde_json::json!({
                        "status": PENDING_STATUS,
                        "tool": call.name,
                    });
                    listener.on_tool_result(ToolResult {
                        id: call.id.clone(),
                        output: placeholder.clone(),
                    });
                    history.push(Message::tool(
                        call.id.clone(),
                        serde_json::to_string(&placeholder).unwrap_or_default(),
                    ));
                }
                Err(e) => {
                    let err_text = e.to_string();
                    let err_json = serde_json::json!({"error": err_text});
                    listener.on_tool_result(ToolResult {
                        id: call.id.clone(),
                        output: err_json.clone(),
                    });
                    history.push(Message::tool(
                        call.id.clone(),
                        serde_json::to_string(&err_json).unwrap_or_default(),
                    ));
                }
            }
        }

        if paused {
            listener.on_messages_snapshot(&history);
            listener.on_done(StopReason::ToolUse);
            return;
        }
        // else: loop continues; backend gets the assistant +
        // tool messages we just appended on the next iteration.
    }

    listener.on_error(format!(
        "max tool iterations ({max_iterations}) reached without a final answer"
    ));
    listener.on_done(StopReason::Error);
}

/// Async helper that resolves the moment the cancel flag flips.
/// Polled at 50ms granularity so a user hitting "stop" while a slow
/// tool runs returns control to the orchestrator within that budget.
/// We don't pay for a tokio::sync::Notify here because the cancel
/// path is rare; the periodic load is cheap.
async fn wait_for_cancel(cancel: &Arc<AtomicBool>) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// Spawn a future onto whichever runtime is appropriate. When
/// chan-llm is called from a tokio context (chan-server's runtime),
/// we use that. Otherwise (a CLI / native shell with no ambient
/// runtime) we fall back to a process-shared runtime created on
/// first use.
fn spawn<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(future);
    } else {
        shared_runtime().spawn(future);
    }
}

fn shared_runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        // Default to 4 worker threads (was 2). Hosts that fan out
        // many concurrent sessions on the same chan-llm instance
        // (chan-server with multiple clients) benefit from more
        // headroom; a single-session CLI doesn't notice. Override
        // with `CHAN_LLM_RUNTIME_THREADS` for tuning without a
        // rebuild.
        let worker_threads = std::env::var("CHAN_LLM_RUNTIME_THREADS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(4);
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .enable_all()
            .build()
            .expect("build chan-llm shared tokio runtime")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_drive::Library;
    use std::sync::Mutex;
    use tempfile::TempDir;

    struct Collector(Mutex<Vec<Event>>);

    #[derive(Clone)]
    #[allow(dead_code)]
    enum Event {
        Delta(String),
        ToolCall(String),
        ToolResult(String),
        Done(StopReason),
        Error(String),
        ErrorKind(LlmEventError),
        Snapshot(Vec<Message>),
    }

    impl SessionListener for Collector {
        fn on_delta(&self, d: Delta) {
            self.0.lock().unwrap().push(Event::Delta(d.text));
        }
        fn on_tool_call(&self, c: ToolCall) {
            self.0.lock().unwrap().push(Event::ToolCall(c.name));
        }
        fn on_tool_result(&self, r: ToolResult) {
            self.0.lock().unwrap().push(Event::ToolResult(r.id));
        }
        fn on_done(&self, r: StopReason) {
            self.0.lock().unwrap().push(Event::Done(r));
        }
        fn on_error(&self, e: String) {
            self.0.lock().unwrap().push(Event::Error(e));
        }
        // Override the default on_error_kind so the typed variant is
        // observable in tests rather than being flattened to a string
        // by the trait's default impl. Production hosts (chan-server's
        // LlmBroadcastListener) override on_error_kind for the same
        // reason: they want the variant's `code()`, not just its
        // `Display` text.
        fn on_error_kind(&self, e: LlmEventError) {
            self.0.lock().unwrap().push(Event::ErrorKind(e));
        }
        fn on_messages_snapshot(&self, history: &[Message]) {
            self.0
                .lock()
                .unwrap()
                .push(Event::Snapshot(history.to_vec()));
        }
    }

    fn fixture() -> (TempDir, TempDir, Arc<Drive>) {
        let cfg = TempDir::new().unwrap();
        let drive_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(drive_dir.path(), Some("Test".into()))
            .unwrap();
        let drive = lib.open_drive(drive_dir.path()).unwrap();
        (cfg, drive_dir, drive)
    }

    fn fake_agent_cli(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(if cfg!(windows) {
            format!("{name}.cmd")
        } else {
            name.to_string()
        });
        let script = if cfg!(windows) {
            format!("@echo off\r\n{body}\r\n")
        } else {
            let lines = body
                .lines()
                .map(|line| format!("printf '%s\\n' '{line}'\n"))
                .collect::<String>();
            format!("#!/bin/sh\n{lines}")
        };
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn cli_config(kind: backends::BackendKind, cli_path: std::path::PathBuf) -> LlmConfig {
        let mut config = LlmConfig {
            backend: Some(kind),
            cli_path: Some(vec![cli_path]),
            ..Default::default()
        };
        config.enabled.set_for_backend(kind, true);
        config
    }

    fn wait_for_done(collector: &Collector) -> Vec<Event> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            {
                let events = collector.0.lock().unwrap();
                if events.iter().any(|e| matches!(e, Event::Done(_))) {
                    return events.clone();
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for on_done"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn send_with_no_backend_emits_error_and_done() {
        let (_cfg, _root, drive) = fixture();
        let session = LlmSession::new(drive, LlmConfig::default());
        let collector = Arc::new(Collector(Mutex::new(Vec::new())));
        session.send(vec![Message::user("hi")], collector.clone());
        let events = collector.0.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Event::Error(_)));
        assert!(matches!(events[1], Event::Done(StopReason::Error)));
    }

    #[cfg(unix)]
    #[test]
    fn e2e_cli_path_spawn_succeeds_for_all_cli_backends() {
        for (kind, bin, body, expected_delta) in [
            (
                backends::BackendKind::ClaudeCli,
                "claude",
                r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"text","text":"claude ok"}]}}
{"type":"result","subtype":"success","result":"claude ok","is_error":false}"#,
                "claude ok",
            ),
            (
                backends::BackendKind::GeminiCli,
                "gemini",
                r#"{"type":"init","timestamp":"t","session_id":"s","model":"m"}
{"type":"message","timestamp":"t","role":"assistant","content":"gemini ok","delta":true}
{"type":"result","timestamp":"t","status":"success"}"#,
                "gemini ok",
            ),
            (
                backends::BackendKind::CodexCli,
                "codex",
                r#"{"method":"item/agentMessage/delta","params":{"itemId":"msg_1","delta":"codex ok"}}
{"type":"turn.completed","turn":{"status":"completed"}}"#,
                "codex ok",
            ),
        ] {
            let (_cfg, _root, drive) = fixture();
            let bin_dir = TempDir::new().unwrap();
            fake_agent_cli(bin_dir.path(), bin, body);
            let session = LlmSession::new(drive, cli_config(kind, bin_dir.path().to_path_buf()));
            let collector = Arc::new(Collector(Mutex::new(Vec::new())));
            session.send(vec![Message::user("hi")], collector.clone());
            let events = wait_for_done(&collector);
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, Event::Delta(text) if text == expected_delta)),
                "{kind:?} did not emit expected delta; events={events:?}"
            );
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, Event::Done(StopReason::EndOfTurn))),
                "{kind:?} did not finish successfully; events={events:?}"
            );
        }
    }

    #[test]
    fn send_with_missing_cli_emits_typed_spawn_failed() {
        let (_cfg, _root, drive) = fixture();
        let config = LlmConfig {
            backend: Some(backends::BackendKind::ClaudeCli),
            enabled: crate::config::EnabledProviders {
                claude_cli: true,
                ..Default::default()
            },
            claude_cli: crate::config::ClaudeCli {
                cmd: Some(vec!["definitely-not-a-chan-agent-cli".into()]),
                ..Default::default()
            },
            ..Default::default()
        };
        let session = LlmSession::new(drive, config);
        let collector = Arc::new(Collector(Mutex::new(Vec::new())));
        session.send(vec![Message::user("hi")], collector.clone());
        let events = collector.0.lock().unwrap();
        assert!(matches!(
            &events[0],
            Event::ErrorKind(LlmEventError::SpawnFailed { backend, message })
                if backend == "claude_cli" && message.contains("definitely-not-a-chan-agent-cli")
        ));
        assert!(matches!(events[1], Event::Done(StopReason::Error)));
    }

    /// Stub backend: returns a single tool_use Outcome on the first
    /// run, then an empty Outcome on subsequent runs (so the loop
    /// continues if the orchestrator decides to). Lets us exercise
    /// run_loop end-to-end without a real CLI backend.
    struct ToolUseBackend {
        calls: Mutex<usize>,
        proposed: Vec<ToolCall>,
    }

    #[async_trait::async_trait]
    impl backends::Backend for ToolUseBackend {
        async fn run(
            &self,
            _messages: &[Message],
            _tools: &[crate::tools::ToolSchema],
            _listener: Arc<dyn SessionListener>,
            _cancel: Arc<AtomicBool>,
        ) -> backends::Outcome {
            let mut n = self.calls.lock().unwrap();
            *n += 1;
            if *n == 1 {
                backends::Outcome {
                    assistant_text: String::new(),
                    tool_calls: self.proposed.clone(),
                    stop_reason: StopReason::ToolUse,
                }
            } else {
                backends::Outcome {
                    assistant_text: "done".into(),
                    tool_calls: Vec::new(),
                    stop_reason: StopReason::EndOfTurn,
                }
            }
        }
    }

    /// When auto_apply_writes is off and the assistant proposes
    /// write_file, the orchestrator must push a placeholder Tool
    /// message with the same call id so the transcript is
    /// well-formed for resume.
    #[test]
    fn pending_write_pushes_placeholder_tool_result() {
        let (_cfg, _root, drive) = fixture();
        let backend = Arc::new(ToolUseBackend {
            calls: Mutex::new(0),
            proposed: vec![ToolCall {
                id: "call-1".into(),
                name: "write_file".into(),
                args: serde_json::json!({"path": "a.md", "content": "hi"}),
            }],
        });
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let tool_ctx = crate::tools::ToolContext::new(drive.clone(), false);
        // Drive the loop on a one-shot tokio runtime.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(super::run_loop(
            backend,
            vec![Message::user("write a file")],
            Vec::new(),
            tool_ctx,
            listener.clone(),
            Arc::new(AtomicBool::new(false)),
            DEFAULT_MAX_TOOL_ITERATIONS,
        ));
        // Expect: on_tool_call(write_file), on_tool_result(call-1)
        // with the placeholder, on_done(ToolUse).
        let events = listener.0.lock().unwrap();
        let call_count = events
            .iter()
            .filter(|e| matches!(e, Event::ToolCall(_)))
            .count();
        let result_count = events
            .iter()
            .filter(|e| matches!(e, Event::ToolResult(_)))
            .count();
        assert_eq!(call_count, 1, "one tool_call event");
        assert_eq!(
            result_count,
            1,
            "one tool_result placeholder before pause; got: {events:?}",
            events = events
                .iter()
                .map(|e| match e {
                    Event::ToolCall(s) => format!("call({s})"),
                    Event::ToolResult(s) => format!("result({s})"),
                    Event::Done(r) => format!("done({r:?})"),
                    Event::Delta(_) => "delta".into(),
                    Event::Error(s) => format!("err({s})"),
                    Event::ErrorKind(k) => format!("err_kind({})", k.code()),
                    Event::Snapshot(h) => format!("snap(len={})", h.len()),
                })
                .collect::<Vec<_>>()
        );
        let last = events.last().expect("events");
        assert!(matches!(last, Event::Done(StopReason::ToolUse)));
        // The drive must NOT have been written; auto_apply was off.
        assert!(!drive.exists("a.md"));
        // The ToolUse pause must surface a messages snapshot
        // immediately before on_done so the host can echo the
        // canonical (assistant + placeholder tool_result)
        // transcript back to the client without rebuilding it
        // from streamed events.
        let snap = events
            .iter()
            .rev()
            .find_map(|e| match e {
                Event::Snapshot(h) => Some(h.clone()),
                _ => None,
            })
            .expect("snapshot fired before on_done(ToolUse)");
        assert!(
            snap.iter().any(|m| m.role == Role::Assistant
                && m.tool_calls.iter().any(|c| c.id == "call-1")),
            "snapshot carries the assistant turn that proposed call-1"
        );
        assert!(
            snap.iter()
                .any(|m| m.role == Role::Tool && m.tool_call_id.as_deref() == Some("call-1")),
            "snapshot carries the placeholder tool_result for call-1"
        );
    }

    /// Backend that ends the turn on the first call with text only.
    /// Used to verify the EndOfTurn snapshot includes the final
    /// assistant message; previously run_loop returned without
    /// pushing it into `history`, so any host trying to echo back
    /// canonical post-turn state lost the closing assistant turn.
    struct TextOnlyBackend {
        text: String,
    }

    #[async_trait::async_trait]
    impl backends::Backend for TextOnlyBackend {
        async fn run(
            &self,
            _messages: &[Message],
            _tools: &[crate::tools::ToolSchema],
            _listener: Arc<dyn SessionListener>,
            _cancel: Arc<AtomicBool>,
        ) -> backends::Outcome {
            backends::Outcome {
                assistant_text: self.text.clone(),
                tool_calls: Vec::new(),
                stop_reason: StopReason::EndOfTurn,
            }
        }
    }

    #[test]
    fn end_of_turn_snapshot_includes_final_assistant_message() {
        let (_cfg, _root, drive) = fixture();
        let backend = Arc::new(TextOnlyBackend {
            text: "all done".into(),
        });
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let tool_ctx = crate::tools::ToolContext::new(drive, false);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(super::run_loop(
            backend,
            vec![Message::user("anything")],
            Vec::new(),
            tool_ctx,
            listener.clone(),
            Arc::new(AtomicBool::new(false)),
            DEFAULT_MAX_TOOL_ITERATIONS,
        ));
        let events = listener.0.lock().unwrap();
        // Snapshot must precede on_done so the host can capture
        // it before the request handler unblocks.
        let snap_idx = events
            .iter()
            .position(|e| matches!(e, Event::Snapshot(_)))
            .expect("snapshot fired");
        let done_idx = events
            .iter()
            .position(|e| matches!(e, Event::Done(_)))
            .expect("done fired");
        assert!(snap_idx < done_idx, "snapshot before done");
        let Event::Snapshot(history) = &events[snap_idx] else {
            unreachable!()
        };
        let last = history.last().expect("non-empty snapshot");
        assert_eq!(last.role, Role::Assistant);
        assert_eq!(last.content, "all done");
    }

    /// EndOfTurn with empty assistant_text must not push a stub
    /// message into the snapshot. Some backends emit an empty
    /// closing turn after tools have run; a stub Assistant{""}
    /// round-trips badly.
    #[test]
    fn end_of_turn_snapshot_skips_empty_assistant_text() {
        let (_cfg, _root, drive) = fixture();
        let backend = Arc::new(TextOnlyBackend {
            text: String::new(),
        });
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let tool_ctx = crate::tools::ToolContext::new(drive, false);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(super::run_loop(
            backend,
            vec![Message::user("anything")],
            Vec::new(),
            tool_ctx,
            listener.clone(),
            Arc::new(AtomicBool::new(false)),
            DEFAULT_MAX_TOOL_ITERATIONS,
        ));
        let events = listener.0.lock().unwrap();
        let Event::Snapshot(history) = events
            .iter()
            .find(|e| matches!(e, Event::Snapshot(_)))
            .expect("snapshot fired")
        else {
            unreachable!()
        };
        assert!(
            !history.iter().any(|m| m.role == Role::Assistant),
            "no synthetic empty assistant message"
        );
    }

    /// Backend that simulates a long-running stream by sleeping
    /// before returning. Used to test cancellation: if the cancel
    /// flag flips while the backend is "running", run_loop should
    /// emit on_done(Cancelled) and stop iterating.
    struct SlowBackend;

    #[async_trait::async_trait]
    impl backends::Backend for SlowBackend {
        async fn run(
            &self,
            _messages: &[Message],
            _tools: &[crate::tools::ToolSchema],
            _listener: Arc<dyn SessionListener>,
            cancel: Arc<AtomicBool>,
        ) -> backends::Outcome {
            // Pretend to stream; check cancel periodically.
            for _ in 0..50 {
                if cancel.load(Ordering::Relaxed) {
                    return backends::Outcome::cancelled(String::new());
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            backends::Outcome {
                assistant_text: "done".into(),
                tool_calls: Vec::new(),
                stop_reason: StopReason::EndOfTurn,
            }
        }
    }

    #[test]
    fn cancel_during_backend_emits_cancelled() {
        let (_cfg, _root, drive) = fixture();
        let backend = Arc::new(SlowBackend);
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let tool_ctx = crate::tools::ToolContext::new(drive, false);
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = cancel.clone();
        // Flip cancel after a short delay so the backend's loop
        // sees it mid-stream.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener_inner = listener.clone();
            let join = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                cancel_clone.store(true, Ordering::Relaxed);
            });
            super::run_loop(
                backend,
                vec![Message::user("hi")],
                Vec::new(),
                tool_ctx,
                listener_inner,
                cancel,
                DEFAULT_MAX_TOOL_ITERATIONS,
            )
            .await;
            let _ = join.await;
            let events = listener.0.lock().unwrap();
            let last = events.last().expect("at least one event");
            assert!(
                matches!(last, Event::Done(StopReason::Cancelled)),
                "last event should be Done(Cancelled); got {events:?}",
                events = events.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>()
            );
        });
    }

    impl std::fmt::Debug for Event {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Event::Delta(t) => write!(f, "Delta({t})"),
                Event::ToolCall(n) => write!(f, "ToolCall({n})"),
                Event::ToolResult(id) => write!(f, "ToolResult({id})"),
                Event::Done(r) => write!(f, "Done({r:?})"),
                Event::Error(e) => write!(f, "Error({e})"),
                Event::ErrorKind(e) => write!(f, "ErrorKind({}={e})", e.code()),
                Event::Snapshot(h) => write!(f, "Snapshot(len={})", h.len()),
            }
        }
    }

    #[test]
    fn message_constructors() {
        let u = Message::user("hi");
        assert_eq!(u.role, Role::User);
        let s = Message::system("you are chan");
        assert_eq!(s.role, Role::System);
        let a = Message::assistant("ok");
        assert_eq!(a.role, Role::Assistant);
        let t = Message::tool("call-1", "result");
        assert_eq!(t.role, Role::Tool);
        assert_eq!(t.tool_call_id.as_deref(), Some("call-1"));
    }

    /// Build a transcript that mirrors what the orchestrator
    /// leaves in `history` after a paused write_file: a user turn,
    /// the assistant's tool_use turn carrying `call_id`, and a
    /// placeholder Tool message for the same call.
    fn paused_transcript(call_id: &str, args: serde_json::Value) -> Vec<Message> {
        let placeholder = serde_json::json!({
            "status": PENDING_STATUS,
            "tool": "write_file",
        });
        vec![
            Message::user("write a file"),
            Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    id: call_id.into(),
                    name: "write_file".into(),
                    args,
                }],
                images: Vec::new(),
            },
            Message::tool(call_id, serde_json::to_string(&placeholder).unwrap()),
        ]
    }

    #[test]
    fn is_pending_placeholder_detects_orchestrator_emit() {
        let history = paused_transcript("c1", serde_json::json!({}));
        assert!(is_pending_placeholder(history.last().unwrap()));
        // A non-Tool message is never a placeholder.
        assert!(!is_pending_placeholder(&history[0]));
        // A Tool message with a different status is not a placeholder.
        let other = Message::tool("c2", r#"{"status":"applied_but_failed"}"#);
        assert!(!is_pending_placeholder(&other));
        // A Tool message with non-JSON content is not a placeholder.
        let plain = Message::tool("c3", "raw text");
        assert!(!is_pending_placeholder(&plain));
    }

    #[test]
    fn apply_resume_swaps_placeholder_for_applied() {
        let history = paused_transcript("c1", serde_json::json!({}));
        let result = serde_json::json!({"path": "a.md", "bytes_written": 2});
        let updated = apply_resume(history, "c1", ResumeOutcome::Applied(result.clone())).unwrap();
        // Last message is the swapped Tool result; content is the
        // raw applied JSON (no wrapper) so back-compat with hosts
        // that already wrote the value is preserved.
        let last = updated.last().unwrap();
        assert_eq!(last.role, Role::Tool);
        assert_eq!(last.tool_call_id.as_deref(), Some("c1"));
        let parsed: serde_json::Value = serde_json::from_str(&last.content).unwrap();
        assert_eq!(parsed, result);
    }

    #[test]
    fn apply_resume_swaps_placeholder_for_rejected() {
        let history = paused_transcript("c1", serde_json::json!({}));
        let updated = apply_resume(
            history,
            "c1",
            ResumeOutcome::Rejected {
                reason: Some("user said no".into()),
            },
        )
        .unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&updated.last().unwrap().content).unwrap();
        assert_eq!(parsed["status"], REJECTED_STATUS);
        assert_eq!(parsed["reason"], "user said no");
    }

    #[test]
    fn apply_resume_swaps_placeholder_for_failed() {
        let history = paused_transcript("c1", serde_json::json!({}));
        let updated = apply_resume(
            history,
            "c1",
            ResumeOutcome::Failed {
                error: "disk full".into(),
            },
        )
        .unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&updated.last().unwrap().content).unwrap();
        assert_eq!(parsed["status"], FAILED_STATUS);
        assert_eq!(parsed["error"], "disk full");
    }

    #[test]
    fn apply_resume_errors_when_id_unknown() {
        let history = paused_transcript("c1", serde_json::json!({}));
        let err = apply_resume(
            history,
            "wrong-id",
            ResumeOutcome::Applied(serde_json::json!({})),
        )
        .unwrap_err();
        match err {
            LlmError::Resume(msg) => assert!(msg.contains("wrong-id"), "msg: {msg}"),
            other => panic!("expected Resume, got {other:?}"),
        }
    }

    #[test]
    fn apply_resume_refuses_to_overwrite_real_result() {
        // After a successful apply, the message is no longer a
        // placeholder. A second apply_resume against the same id
        // must error rather than silently clobber the real result.
        let history = paused_transcript("c1", serde_json::json!({}));
        let once = apply_resume(
            history,
            "c1",
            ResumeOutcome::Applied(serde_json::json!({"ok":true})),
        )
        .unwrap();
        let err = apply_resume(
            once,
            "c1",
            ResumeOutcome::Applied(serde_json::json!({"ok":false})),
        )
        .unwrap_err();
        match err {
            LlmError::Resume(msg) => {
                assert!(msg.contains("not a pending placeholder"), "msg: {msg}")
            }
            other => panic!("expected Resume, got {other:?}"),
        }
    }

    #[test]
    fn approve_pending_executes_call_and_swaps_placeholder() {
        let (_cfg, _root, drive) = fixture();
        let session = LlmSession::new(drive.clone(), LlmConfig::default());
        // Simulate the orchestrator's paused state: the assistant
        // wanted to write `note.md`, auto_apply was off, and the
        // placeholder is sitting in the transcript.
        let history = paused_transcript(
            "c1",
            serde_json::json!({"path": "note.md", "content": "hello\n"}),
        );
        let updated = session.approve_pending(history, "c1").unwrap();
        // Placeholder is gone; the Tool message now carries the
        // real write result and the file landed on disk.
        let last = updated.last().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&last.content).unwrap();
        assert_eq!(parsed["path"], "note.md");
        assert!(parsed.get("bytes_written").is_some(), "got: {parsed}");
        assert!(drive.exists("note.md"));
    }

    #[test]
    fn approve_pending_errors_when_call_not_in_history() {
        let (_cfg, _root, drive) = fixture();
        let session = LlmSession::new(drive, LlmConfig::default());
        // Transcript without the assistant turn carrying the call.
        let history = vec![Message::user("hi")];
        let err = session.approve_pending(history, "c1").unwrap_err();
        match err {
            LlmError::Resume(msg) => assert!(msg.contains("c1"), "msg: {msg}"),
            other => panic!("expected Resume, got {other:?}"),
        }
    }

    /// A host `SessionListener` that panics in every callback. Used
    /// to verify the orchestrator's panic guard keeps on_done's
    /// exact-once contract even when the host listener is buggy.
    struct PanickingListener;
    impl SessionListener for PanickingListener {
        fn on_delta(&self, _: Delta) {
            panic!("on_delta panicked");
        }
        fn on_tool_call(&self, _: ToolCall) {
            panic!("on_tool_call panicked");
        }
        fn on_tool_result(&self, _: ToolResult) {
            panic!("on_tool_result panicked");
        }
        fn on_done(&self, _: StopReason) {
            panic!("on_done panicked");
        }
        fn on_error(&self, _: String) {
            panic!("on_error panicked");
        }
        fn on_error_kind(&self, _: LlmEventError) {
            panic!("on_error_kind panicked");
        }
        fn on_messages_snapshot(&self, _: &[Message]) {
            panic!("on_messages_snapshot panicked");
        }
    }

    #[test]
    fn safe_listener_catches_panics_in_every_callback() {
        // Without the wrapper, each of these would unwind the
        // caller. With it, the panic is caught and logged.
        let inner: Arc<dyn SessionListener> = Arc::new(PanickingListener);
        let safe = SafeListener { inner };
        safe.on_delta(Delta { text: "x".into() });
        safe.on_tool_call(ToolCall {
            id: "c1".into(),
            name: "f".into(),
            args: serde_json::json!({}),
        });
        safe.on_tool_result(ToolResult {
            id: "c1".into(),
            output: serde_json::json!({}),
        });
        safe.on_error("boom".into());
        safe.on_error_kind(LlmEventError::Auth {
            backend: "claude_cli".into(),
            message: "auth failed".into(),
        });
        safe.on_messages_snapshot(&[Message::user("hi")]);
        safe.on_done(StopReason::EndOfTurn);
        // No panic propagated; reaching this line is the assertion.
    }

    #[test]
    fn llm_event_error_code_and_backend_match_variant() {
        let auth = LlmEventError::Auth {
            backend: "claude_cli".into(),
            message: "auth failed".into(),
        };
        assert_eq!(auth.code(), "auth");
        assert_eq!(auth.backend(), "claude_cli");

        let rl = LlmEventError::RateLimited {
            backend: "gemini_cli".into(),
            retry_after_secs: Some(30),
            message: "quota exceeded".into(),
        };
        assert_eq!(rl.code(), "rate_limited");
        assert!(rl.to_string().contains("retry after 30s"));

        let backend = LlmEventError::Backend {
            backend: "claude_cli".into(),
            status: 503,
            message: "Service Unavailable".into(),
        };
        assert_eq!(backend.code(), "backend");
        assert!(backend.to_string().contains("503"));

        let truncated = LlmEventError::StreamTruncated {
            backend: "claude_cli".into(),
            message: "connection reset".into(),
        };
        assert_eq!(truncated.code(), "stream_truncated");
    }

    #[test]
    fn on_error_kind_default_impl_bridges_to_on_error() {
        // A listener that only implements on_error must still receive
        // typed errors via the default bridge as their Display string.
        struct StringOnly(Mutex<Vec<String>>);
        impl SessionListener for StringOnly {
            fn on_delta(&self, _: Delta) {}
            fn on_tool_call(&self, _: ToolCall) {}
            fn on_tool_result(&self, _: ToolResult) {}
            fn on_done(&self, _: StopReason) {}
            fn on_error(&self, e: String) {
                self.0.lock().unwrap().push(e);
            }
        }
        let l = StringOnly(Mutex::new(Vec::new()));
        l.on_error_kind(LlmEventError::Auth {
            backend: "claude_cli".into(),
            message: "auth failed".into(),
        });
        let errs = l.0.lock().unwrap();
        assert_eq!(errs.len(), 1);
        assert!(
            errs[0].contains("claude_cli") && errs[0].contains("auth failed"),
            "got: {}",
            errs[0]
        );
    }

    #[test]
    fn send_with_panicking_listener_does_not_propagate() {
        // The no-backend path emits on_error + on_done synchronously
        // before returning. With the panic guard in place those
        // callbacks panic and are caught; `send` must return
        // normally instead of unwinding through the caller.
        let (_cfg, _root, drive) = fixture();
        let session = LlmSession::new(drive, LlmConfig::default());
        let listener: Arc<dyn SessionListener> = Arc::new(PanickingListener);
        let _cancel = session.send(vec![Message::user("hi")], listener);
        // Reaching this line means the panicking on_error and on_done
        // were caught by the SafeListener wrapper rather than unwinding.
    }

    /// Backend that emits a typed error through the listener and
    /// returns `Outcome::error()`. Lets the orchestrator-level
    /// typed-error contract be exercised without standing up a real
    /// CLI process. The backend itself decides which variant to
    /// emit so each test can pick the variant that matches the UX
    /// path it's covering (Auth -> "fix your key" affordance,
    /// RateLimited -> "back off" affordance, etc).
    struct TypedErrorBackend {
        kind: LlmEventError,
    }

    #[async_trait::async_trait]
    impl backends::Backend for TypedErrorBackend {
        async fn run(
            &self,
            _messages: &[Message],
            _tools: &[crate::tools::ToolSchema],
            listener: Arc<dyn SessionListener>,
            _cancel: Arc<AtomicBool>,
        ) -> backends::Outcome {
            listener.on_error_kind(self.kind.clone());
            backends::Outcome::error()
        }
    }

    /// The orchestrator must forward typed errors from the backend
    /// to the listener verbatim (variant + payload), then close the
    /// turn with `Done(Error)`. Without the on_error_kind override on
    /// the listener, the typed variant would be flattened to a
    /// string by the trait's default impl and chan-server's frontend
    /// would lose the per-variant UX branch.
    #[test]
    fn forwards_typed_backend_error_to_listener_and_closes_with_error() {
        let (_cfg, _root, drive) = fixture();
        let backend = Arc::new(TypedErrorBackend {
            kind: LlmEventError::Auth {
                backend: "claude_cli".into(),
                message: "401 unauthorized".into(),
            },
        });
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let tool_ctx = crate::tools::ToolContext::new(drive, false);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(super::run_loop(
            backend,
            vec![Message::user("hi")],
            Vec::new(),
            tool_ctx,
            listener.clone() as Arc<dyn SessionListener>,
            Arc::new(AtomicBool::new(false)),
            DEFAULT_MAX_TOOL_ITERATIONS,
        ));
        let events = listener.0.lock().unwrap();
        // Two events: typed error from the backend, then Done(Error)
        // from the orchestrator. on_messages_snapshot must NOT fire
        // (snapshot is success-path only; firing on Error would
        // overwrite the host's pre-call transcript with a partial
        // one).
        assert_eq!(
            events.len(),
            2,
            "exactly typed-error + done; got {events:?}",
            events = events.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>()
        );
        match &events[0] {
            Event::ErrorKind(LlmEventError::Auth { backend, message }) => {
                assert_eq!(backend, "claude_cli");
                assert_eq!(message, "401 unauthorized");
            }
            other => panic!("expected ErrorKind(Auth); got {other:?}"),
        }
        assert!(matches!(events[1], Event::Done(StopReason::Error)));
    }

    /// Each typed error variant must round-trip through the
    /// orchestrator's listener wiring with its payload preserved.
    /// The default `on_error_kind` impl on `SessionListener` would
    /// flatten everything to a string; chan-server's listener (and
    /// the test `Collector`) override that to keep the variant.
    /// This test catches a regression where the orchestrator (or
    /// `SafeListener`) accidentally rebuilds the error from
    /// `Display` instead of forwarding the original.
    #[test]
    fn each_typed_error_variant_round_trips_through_listener() {
        let (_cfg, _root, drive) = fixture();
        let variants = vec![
            LlmEventError::Auth {
                backend: "claude_cli".into(),
                message: "x".into(),
            },
            LlmEventError::RateLimited {
                backend: "gemini_cli".into(),
                retry_after_secs: Some(7),
                message: "x".into(),
            },
            LlmEventError::BackendUnreachable {
                backend: "codex_cli".into(),
                message: "x".into(),
            },
            LlmEventError::BadRequest {
                backend: "claude_cli".into(),
                message: "x".into(),
            },
            LlmEventError::Backend {
                backend: "claude_cli".into(),
                status: 500,
                message: "x".into(),
            },
            LlmEventError::SpawnFailed {
                backend: "gemini_cli".into(),
                message: "ENOENT".into(),
            },
            LlmEventError::StreamTruncated {
                backend: "codex_cli".into(),
                message: "EOF".into(),
            },
            LlmEventError::Timeout {
                backend: "claude_cli".into(),
                message: "300s".into(),
            },
            LlmEventError::ParseError {
                backend: "gemini_cli".into(),
                message: "bad frame".into(),
            },
            LlmEventError::Cancelled {
                backend: "codex_cli".into(),
            },
            LlmEventError::Other {
                backend: "claude_cli".into(),
                message: "x".into(),
            },
        ];
        for kind in variants {
            let backend = Arc::new(TypedErrorBackend { kind: kind.clone() });
            let listener = Arc::new(Collector(Mutex::new(Vec::new())));
            let tool_ctx = crate::tools::ToolContext::new(drive.clone(), false);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(super::run_loop(
                backend,
                vec![Message::user("hi")],
                Vec::new(),
                tool_ctx,
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
                DEFAULT_MAX_TOOL_ITERATIONS,
            ));
            let events = listener.0.lock().unwrap();
            // Find the ErrorKind event and check its `code()` matches
            // the input variant. We don't pattern-match the entire
            // payload (the loop has 11 variants and the assertion
            // would balloon); the `code()` discriminator is the
            // public contract chan-server's frontend branches on.
            let got_code = events.iter().find_map(|e| match e {
                Event::ErrorKind(k) => Some(k.code()),
                _ => None,
            });
            assert_eq!(
                got_code,
                Some(kind.code()),
                "variant {kind:?} did not round-trip; saw {events:?}",
                events = events.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>(),
            );
        }
    }

    /// Backend that proposes two tool calls in one turn. Pairs with
    /// `CancelOnFirstResultListener` so the second call's dispatch
    /// is exercised against an already-flipped cancel flag, which
    /// must short-circuit at the per-call cancel checkpoint at the
    /// top of the loop in `run_loop` rather than running both tools
    /// to completion.
    struct TwoToolBackend;

    #[async_trait::async_trait]
    impl backends::Backend for TwoToolBackend {
        async fn run(
            &self,
            _messages: &[Message],
            _tools: &[crate::tools::ToolSchema],
            _listener: Arc<dyn SessionListener>,
            _cancel: Arc<AtomicBool>,
        ) -> backends::Outcome {
            backends::Outcome {
                assistant_text: String::new(),
                tool_calls: vec![
                    ToolCall {
                        id: "call-a".into(),
                        name: "list_files".into(),
                        args: serde_json::json!({}),
                    },
                    ToolCall {
                        id: "call-b".into(),
                        name: "list_files".into(),
                        args: serde_json::json!({}),
                    },
                ],
                stop_reason: StopReason::ToolUse,
            }
        }
    }

    /// Listener that flips the cancel flag the first time the
    /// orchestrator emits `on_tool_result`. After this fires, the
    /// orchestrator's per-call cancel check at the top of the
    /// dispatch loop (`if cancel.load(...) { ... return; }`) must
    /// observe the flag and emit Cancelled instead of running the
    /// second tool. Inner Collector keeps the event log so tests
    /// can assert on what fired.
    struct CancelOnFirstResultListener {
        inner: Arc<Collector>,
        cancel: Arc<AtomicBool>,
        tripped: Mutex<bool>,
    }

    impl SessionListener for CancelOnFirstResultListener {
        fn on_delta(&self, d: Delta) {
            self.inner.on_delta(d);
        }
        fn on_tool_call(&self, c: ToolCall) {
            self.inner.on_tool_call(c);
        }
        fn on_tool_result(&self, r: ToolResult) {
            self.inner.on_tool_result(r);
            let mut tripped = self.tripped.lock().unwrap();
            if !*tripped {
                self.cancel.store(true, Ordering::Relaxed);
                *tripped = true;
            }
        }
        fn on_done(&self, r: StopReason) {
            self.inner.on_done(r);
        }
        fn on_error(&self, e: String) {
            self.inner.on_error(e);
        }
        fn on_error_kind(&self, e: LlmEventError) {
            self.inner.on_error_kind(e);
        }
        fn on_messages_snapshot(&self, h: &[Message]) {
            self.inner.on_messages_snapshot(h);
        }
    }

    /// Cancel flipped after the first tool result must short-circuit
    /// before the second tool dispatches. The orchestrator must:
    ///   - emit on_tool_call(call-a) + on_tool_result(call-a)
    ///   - observe cancel at the top of the per-call loop
    ///   - emit on_done(Cancelled)
    ///   - NOT emit on_tool_call(call-b)
    ///   - NOT emit a snapshot (reserved for clean terminations)
    #[test]
    fn cancel_after_first_tool_short_circuits_remaining_tools() {
        let (_cfg, _root, drive) = fixture();
        let backend = Arc::new(TwoToolBackend);
        let inner = Arc::new(Collector(Mutex::new(Vec::new())));
        let cancel = Arc::new(AtomicBool::new(false));
        let listener = Arc::new(CancelOnFirstResultListener {
            inner: inner.clone(),
            cancel: cancel.clone(),
            tripped: Mutex::new(false),
        });
        let tool_ctx = crate::tools::ToolContext::new(drive, false);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(super::run_loop(
            backend,
            vec![Message::user("list please")],
            Vec::new(),
            tool_ctx,
            listener as Arc<dyn SessionListener>,
            cancel,
            DEFAULT_MAX_TOOL_ITERATIONS,
        ));
        let events = inner.0.lock().unwrap();
        let tool_calls: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::ToolCall(n) => Some(n.as_str()),
                _ => None,
            })
            .collect();
        let tool_results: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::ToolResult(id) => Some(id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            tool_calls.len(),
            1,
            "only call-a should have dispatched; got {events:?}",
            events = events.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>(),
        );
        assert_eq!(tool_results, vec!["call-a"], "only call-a result fires");
        let last = events.last().expect("at least one event");
        assert!(
            matches!(last, Event::Done(StopReason::Cancelled)),
            "last event must be Done(Cancelled); got {last:?}",
        );
        // Snapshot must NOT have fired: it's success-path only.
        assert!(
            !events.iter().any(|e| matches!(e, Event::Snapshot(_))),
            "snapshot must not fire on Cancelled",
        );
    }
}
