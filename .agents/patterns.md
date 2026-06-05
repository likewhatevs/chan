# Contributor Patterns

- **Atomic writes via chan-workspace**: every user-content write
  goes through `Workspace::write_text` or `Workspace::write_bytes`.
  These enforce the editable-text gate, the path sandbox, and
  the special-file refusal. Don't bypass.
- **Subcommand parity**: every chan subcommand has a clap
  definition + a `cmd_*` function in `crates/chan/src/main.rs`.
  Help text must reflect actual behavior; don't claim env vars
  or flags that don't exist.
- **Server routes go in chan-server**: never inline an axum
  handler inside the binary crate. The `chan` crate parses args
  and calls `chan_server::serve`. New routes belong in the
  matching `crates/chan-server/src/routes/<area>.rs`; cross-area
  shared types live in the module that owns them. `lib.rs::router()`
  is the only place the route table is assembled.
- **App-level config files**: anything new under `<config>/chan/`
  goes through `crate::store::{load_toml, save_toml}` so atomic
  writes + parent-dir fsync match the rest of the app. Don't roll
  a fresh `tempfile + rename` by hand.
- **chan-llm is MCP-only**: after Phase 5 the crate exposes the
  chan MCP `Server`, its tool schemas, embedded prompts, and key
  resolution. There is no in-app agent session and no CLI
  backend wrappers; external agents connect through the in-process
  MCP server in `crates/chan-server/src/mcp_bridge.rs`.
- **Pinned toolchain**: do not introduce code that requires a
  newer Rust than `rust-toolchain.toml` declares without bumping
  the pin in the same commit.
