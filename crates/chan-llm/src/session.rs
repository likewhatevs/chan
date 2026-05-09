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

/// Conversation roles. The taxonomy mirrors OpenAI / Anthropic
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
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: Vec::new(),
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

/// What the consumer implements. `Send + Sync` because events
/// arrive on the runtime's worker threads.
pub trait SessionListener: Send + Sync {
    fn on_delta(&self, delta: Delta);
    fn on_tool_call(&self, call: ToolCall);
    fn on_tool_result(&self, result: ToolResult);
    fn on_done(&self, reason: StopReason);
    fn on_error(&self, error: String);
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
    /// assistant returns text only or hits MAX_TOOL_ITERATIONS
    /// (defense against runaway loops).
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
    /// Without those placeholders, Anthropic and Gemini reject the
    /// next turn because the tool_use block has no matching
    /// tool_result. `apply_resume` validates that the target
    /// message is actually a placeholder, so a host that double-
    /// resumes catches the mistake instead of silently corrupting
    /// the transcript.
    pub fn send(&self, messages: Vec<Message>, listener: Arc<dyn SessionListener>) -> CancelHandle {
        let cancel = CancelHandle::new();
        let Some(kind) = self.config.backend else {
            // BackendNotConfigured rather than MissingApiKey: this is
            // a "the user hasn't picked a backend" state, not a key
            // problem, and hosts that branch on the error kind want
            // to nudge the user into Settings, not into Keychain.
            listener.on_error(LlmError::BackendNotConfigured.to_string());
            listener.on_done(StopReason::Error);
            return cancel;
        };

        let backend = match backends::build(kind, &self.config, self.drive.root()) {
            Ok(b) => b,
            Err(e) => {
                listener.on_error(e.to_string());
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

        // The agentic CLIs (ClaudeCli, GeminiCli) run as full agents
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
            _ => false,
        };
        let tool_ctx = if agentic_cli_v1 {
            crate::tools::ToolContext::new(self.drive.clone(), true)
        } else {
            self.tool_context()
        };
        let tool_schemas = match kind {
            BackendKind::ClaudeCli | BackendKind::GeminiCli => Vec::new(),
            _ => crate::tools::standard_tool_schemas(),
        };

        let cancel_inner = cancel.flag();
        spawn(async move {
            run_loop(
                backend,
                history,
                tool_schemas,
                tool_ctx,
                listener,
                cancel_inner,
            )
            .await;
        });
        cancel
    }
}

/// Maximum tool-call rounds in a single `send`. Defense against
/// the assistant looping on a buggy tool call (e.g. read_file on
/// a non-existent path, then read_file on a similar non-existent
/// path, etc.). When the cap fires we emit `on_done(Error)` with
/// a clear message; the host can offer the user a "try again"
/// affordance.
const MAX_TOOL_ITERATIONS: usize = 12;

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
) {
    for _ in 0..MAX_TOOL_ITERATIONS {
        if cancel.load(Ordering::Relaxed) {
            listener.on_done(StopReason::Cancelled);
            return;
        }
        let outcome = backend
            .run(
                history.clone(),
                tool_schemas.clone(),
                listener.clone(),
                cancel.clone(),
            )
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
            listener.on_done(outcome.stop_reason);
            return;
        }

        // Assistant proposed tool calls. Append the assistant
        // turn (with tool_calls) to history so the next backend
        // call sees the conversation including the proposed
        // calls; then run each tool, appending its result as a
        // Tool message.
        history.push(Message {
            role: Role::Assistant,
            content: outcome.assistant_text.clone(),
            tool_call_id: None,
            tool_calls: outcome.tool_calls.clone(),
        });

        // Track every tool call from this turn that we still owe a
        // result for. Anthropic and Gemini reject the next user turn
        // if any tool_use block from the assistant turn is missing
        // its matching tool_result. We push a placeholder result for
        // any tool call we couldn't immediately resolve (the host
        // overrides the placeholder when it resumes). Without this,
        // a `Pending` write would leave a dangling tool_use forever.
        let total_calls = outcome.tool_calls.len();
        let mut paused_call: Option<ToolCall> = None;
        for (idx, call) in outcome.tool_calls.into_iter().enumerate() {
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
            let exec_result = tokio::task::spawn_blocking(move || {
                // catch_unwind keeps the panic payload (which can
                // contain user paths or other PII) out of the
                // model-visible tool result. The full payload and
                // backtrace land in logs via the panic hook; the
                // assistant sees a generic, scrubbed message.
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::tools::execute(&exec_name, &exec_args, &exec_ctx)
                }))
            })
            .await;
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
                    // result appended.
                    paused_call = Some(call.clone());
                    // Push a placeholder so the transcript stays
                    // well-formed for Anthropic / Gemini. The host
                    // is expected to *replace* this entry with the
                    // real result on resume (matching by id), or
                    // leave it as-is if the user denies the write.
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
                    // Any later calls in this same assistant turn
                    // also need placeholders so the assistant turn
                    // is fully matched. Anthropic in particular
                    // rejects a partial pairing.
                    if idx + 1 < total_calls {
                        // Filled in below by the same placeholder
                        // path; the loop simply continues with the
                        // remaining calls and pushes placeholders
                        // for each via this branch (since they all
                        // route through the same `Pending` handler
                        // for write_file). For non-write tools we
                        // never see Pending here, so this branch is
                        // only entered once per turn in practice.
                    }
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

        if paused_call.is_some() {
            listener.on_done(StopReason::ToolUse);
            return;
        }
        // else: loop continues; backend gets the assistant +
        // tool messages we just appended on the next iteration.
    }

    listener.on_error(format!(
        "max tool iterations ({MAX_TOOL_ITERATIONS}) reached without a final answer"
    ));
    listener.on_done(StopReason::Error);
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

    #[allow(dead_code)]
    enum Event {
        Delta(String),
        ToolCall(String),
        ToolResult(String),
        Done(StopReason),
        Error(String),
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

    /// Stub backend: returns a single tool_use Outcome on the first
    /// run, then an empty Outcome on subsequent runs (so the loop
    /// continues if the orchestrator decides to). Lets us exercise
    /// run_loop end-to-end without an HTTP backend.
    struct ToolUseBackend {
        calls: Mutex<usize>,
        proposed: Vec<ToolCall>,
    }

    #[async_trait::async_trait]
    impl backends::Backend for ToolUseBackend {
        async fn run(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<crate::tools::ToolSchema>,
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
    /// well-formed. Otherwise Anthropic / Gemini reject the next
    /// turn for an unmatched tool_use block.
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
                })
                .collect::<Vec<_>>()
        );
        let last = events.last().expect("events");
        assert!(matches!(last, Event::Done(StopReason::ToolUse)));
        // The drive must NOT have been written; auto_apply was off.
        assert!(!drive.exists("a.md"));
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
            _messages: Vec<Message>,
            _tools: Vec<crate::tools::ToolSchema>,
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
}
