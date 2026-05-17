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

/// Subprocess/agent lifecycle status. Additive companion to the
/// existing text/tool callbacks: hosts can drive activity indicators,
/// health checks, and recycle decisions without parsing backend-
/// specific strings.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentStatus {
    Spawned {
        backend: String,
        pid: Option<u32>,
    },
    Ready {
        backend: String,
        session_id: Option<String>,
        model: Option<String>,
        version: Option<String>,
    },
    Thinking {
        backend: String,
        status: Option<String>,
    },
    Heartbeat {
        backend: String,
        idle_ms: u64,
    },
    TurnStopping {
        backend: String,
        reason: Option<String>,
    },
    RateLimit {
        backend: String,
        status: String,
        resets_at: Option<String>,
        rate_limit_type: Option<String>,
        in_overage: bool,
    },
    Exited {
        backend: String,
        code: Option<i32>,
        success: bool,
    },
    Unhealthy {
        backend: String,
        reason: String,
        detail: Option<String>,
    },
    Cancelled {
        backend: String,
    },
}

/// UI-facing activity emitted by agentic CLI backends. These events
/// are observational; they do not change the transcript contract.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentActivity {
    SessionStarted {
        backend: String,
        session_id: Option<String>,
    },
    MessageStarted {
        backend: String,
        message_id: Option<String>,
        parent_id: Option<String>,
    },
    ThinkingStarted {
        backend: String,
        parent_id: Option<String>,
    },
    ThinkingDelta {
        backend: String,
        text: String,
        parent_id: Option<String>,
    },
    ToolStarted {
        backend: String,
        id: String,
        name: String,
        parent_id: Option<String>,
    },
    ToolArgsDelta {
        backend: String,
        id: Option<String>,
        partial_json: String,
        parent_id: Option<String>,
    },
    ToolFinished {
        backend: String,
        id: String,
        name: Option<String>,
        output: Json,
        is_error: bool,
        parent_id: Option<String>,
    },
    ToolDenied {
        backend: String,
        id: Option<String>,
        name: Option<String>,
        reason: Option<String>,
        input: Json,
        parent_id: Option<String>,
    },
    AgentNote {
        backend: String,
        text: String,
        parent_id: Option<String>,
    },
    TurnUsage {
        backend: String,
        usage: Json,
    },
}

/// Structured request for user input from an agentic backend. A
/// single-choice prompt is represented as a survey with one question.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UserRequest {
    Survey {
        backend: String,
        id: String,
        questions: Vec<UserQuestion>,
        parent_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserQuestion {
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(default)]
    pub multi_select: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<UserOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserOption {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
    /// Agent lifecycle and health status. Default no-op so hosts can
    /// adopt the richer event stream incrementally.
    fn on_status(&self, _status: AgentStatus) {}
    /// UI-facing tool/thinking/background activity. Existing
    /// `on_tool_call` / `on_tool_result` remain the compatibility
    /// contract; this method carries richer status for activity panes.
    fn on_activity(&self, _activity: AgentActivity) {}
    /// Structured prompt from the agent to the user, such as Claude's
    /// `AskUserQuestion` survey. Plain numbered text remains `on_delta`.
    fn on_user_request(&self, _request: UserRequest) {}
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
    fn on_status(&self, status: AgentStatus) {
        let inner = self.inner.clone();
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            inner.on_status(status)
        })) {
            tracing::error!(panic = %panic_message(&p), "listener.on_status panicked");
        }
    }
    fn on_activity(&self, activity: AgentActivity) {
        let inner = self.inner.clone();
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            inner.on_activity(activity)
        })) {
            tracing::error!(panic = %panic_message(&p), "listener.on_activity panicked");
        }
    }
    fn on_user_request(&self, request: UserRequest) {
        let inner = self.inner.clone();
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            inner.on_user_request(request)
        })) {
            tracing::error!(panic = %panic_message(&p), "listener.on_user_request panicked");
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
        self.config.active_backend()
    }

    pub fn tool_context(&self) -> ToolContext {
        ToolContext::new(self.drive.clone())
    }

    /// Kick off a turn. The host passes the full conversation
    /// transcript; chan-llm prepends the system prompt + tool
    /// descriptions automatically. Returns immediately after
    /// spawning the background task; events flow into the
    /// listener.
    ///
    /// Tool-call orchestration: the loop runs the backend, runs
    /// any tool calls the assistant proposed (writes apply
    /// immediately through chan-drive's sandbox), appends results
    /// to the transcript, and runs the backend again. Loops until
    /// the assistant returns text only or hits the configured
    /// `max_tool_iterations` cap (defaults to
    /// `DEFAULT_MAX_TOOL_ITERATIONS`, defense against runaway
    /// loops). Override the cap via `LlmConfig::max_tool_iterations`.
    ///
    /// Permission gating for destructive batch work is the model's
    /// responsibility: a well-behaved agent emits
    /// `AskUserQuestion` (claude-cli) before a batch of writes,
    /// chan-llm forwards that as `UserRequest::Survey`, and the
    /// host's UI collects an answer. Because the agentic CLIs run
    /// as stateless one-shot processes, the answer rides back in
    /// the next call's transcript as a tool_result message paired
    /// to the `AskUserQuestion` tool_use id.
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

        // The agentic CLIs (ClaudeCli, GeminiCli, CodexCli) run as
        // full agents — the chan-llm orchestration loop never
        // executes tool calls for them (the CLI does its own).
        // v1 (mcp_command = None) writes through the CLI's native
        // tools bypassing the sandbox entirely; v2 routes writes
        // through chan-llm's MCP subprocess which uses the same
        // ToolContext. Either way the orchestrator's tool_ctx is
        // just the shared drive handle.
        let _ = kind;
        let tool_ctx = self.tool_context();
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
/// we run the tools through chan-drive's sandbox and append their
/// results, then call the backend again. Loops until the backend
/// emits no tool calls or hits the configured max-iterations cap.
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
                Ok(result) => {
                    listener.on_tool_result(ToolResult {
                        id: call.id.clone(),
                        output: result.clone(),
                    });
                    history.push(Message::tool(
                        call.id.clone(),
                        serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()),
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
        // Loop continues; backend gets the assistant + tool messages
        // we just appended on the next iteration.
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

    #[test]
    fn backend_reports_none_when_selected_backend_is_disabled() {
        let (_cfg, _root, drive) = fixture();
        let config = LlmConfig {
            backend: Some(backends::BackendKind::CodexCli),
            enabled: crate::config::EnabledProviders {
                claude_cli: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let session = LlmSession::new(drive, config);
        assert_eq!(session.backend(), None);
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

    /// write_file used to gate behind auto_apply_writes. Now it
    /// applies immediately; verify the tool result lands in the
    /// transcript and the drive saw the write.
    #[test]
    fn write_file_applies_immediately_through_orchestrator() {
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
        let tool_ctx = crate::tools::ToolContext::new(drive.clone());
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
        let events = listener.0.lock().unwrap();
        // Expect at least one tool_call + matching tool_result.
        assert!(events.iter().any(|e| matches!(e, Event::ToolCall(_))));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ToolResult(id) if id == "call-1")));
        // The write actually landed.
        assert!(drive.exists("a.md"));
        assert_eq!(drive.read_text("a.md").unwrap(), "hi");
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
        let tool_ctx = crate::tools::ToolContext::new(drive);
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
        let tool_ctx = crate::tools::ToolContext::new(drive);
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
        let tool_ctx = crate::tools::ToolContext::new(drive);
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
        fn on_status(&self, _: AgentStatus) {
            panic!("on_status panicked");
        }
        fn on_activity(&self, _: AgentActivity) {
            panic!("on_activity panicked");
        }
        fn on_user_request(&self, _: UserRequest) {
            panic!("on_user_request panicked");
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
        safe.on_status(AgentStatus::Heartbeat {
            backend: "claude_cli".into(),
            idle_ms: 5,
        });
        safe.on_activity(AgentActivity::ToolStarted {
            backend: "claude_cli".into(),
            id: "c1".into(),
            name: "read_file".into(),
            parent_id: None,
        });
        safe.on_user_request(UserRequest::Survey {
            backend: "claude_cli".into(),
            id: "q1".into(),
            questions: vec![UserQuestion {
                question: "Pick one".into(),
                header: None,
                multi_select: false,
                options: vec![UserOption {
                    label: "One".into(),
                    description: None,
                }],
            }],
            parent_id: None,
        });
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
        let tool_ctx = crate::tools::ToolContext::new(drive);
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
            let tool_ctx = crate::tools::ToolContext::new(drive.clone());
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
        let tool_ctx = crate::tools::ToolContext::new(drive);
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
