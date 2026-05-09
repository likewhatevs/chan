//! chan-llm: LLM backends, embedded prompts, and the tool sandbox the
//! assistant uses to read and edit chan drives.
//!
//! This crate is intentionally cross-platform reusable. Two consumer
//! shapes are designed for from day one:
//!
//!   - **`chan-server`** (the HTTP server in `chan-writer/chan`) wraps
//!     `LlmSession` in axum routes and forwards streaming events to
//!     the web frontend over WebSocket.
//!
//!   - **Native shells** (iOS / Android, future) link this crate via
//!     uniffi alongside `chan-drive`. They construct `LlmSession`
//!     directly, implement `SessionListener` in Swift / Kotlin, and
//!     receive streaming deltas + tool calls + tool results without
//!     a network hop.
//!
//! Both consumers see the same prompts, the same tool schemas, and
//! the same edit-control rules. That's the point of the crate: one
//! place where "what the assistant does" lives.
//!
//! ## API shape
//!
//! ```text
//!   LlmConfig          load/save TOML; backend, model, auto_apply,
//!                      key resolution policy.
//!
//!   LlmSession         Arc-able handle. Owns the HTTP client and
//!                      an internal tokio runtime. Constructor:
//!                      `LlmSession::new(drive, config)`.
//!
//!   SessionListener    Send + Sync trait the consumer implements.
//!                      Receives streaming events:
//!                        on_delta(text)
//!                        on_tool_call(call)
//!                        on_tool_result(result)
//!                        on_done(stop_reason)
//!                        on_error(error)
//!
//!   Tool               trait for the assistant's read/write/list/
//!                      search tools. Default impls call into
//!                      chan-drive::Drive.
//! ```
//!
//! Async stays internal: `LlmSession::send` spawns onto the runtime
//! and dispatches into the listener. Callers never see a `Future`.
//! Same callback pattern as `chan-drive::Drive::watch`, for the same
//! reason: uniffi doesn't cross async boundaries cleanly.

#![forbid(unsafe_code)]

pub mod backends;
pub mod config;
pub mod error;
pub mod keys;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod prompts;
pub mod session;
pub mod tools;

pub use backends::BackendKind;
pub use config::LlmConfig;
pub use error::LlmError;
pub use keys::KeyStatus;
pub use session::{
    CancelHandle, Delta, LlmSession, Message, Role, SessionListener, StopReason, ToolCall,
    ToolResult,
};
pub use tools::{StandardTool, ToolContext};
