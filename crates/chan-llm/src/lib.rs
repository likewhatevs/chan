//! chan-llm: MCP-facing prompts and tool sandbox for chan workspaces.
//!
//! The supported surface is the MCP server plus the shared tool
//! implementations it exposes to external agents.

#![forbid(unsafe_code)]

pub mod error;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod prompts;
pub mod tools;

pub use error::LlmError;
pub use tools::{StandardTool, ToolContext};
