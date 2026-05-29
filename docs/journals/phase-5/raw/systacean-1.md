# @@Systacean task 1: chan-llm + chan-drive deep prune, build gate

Owner: @@Systacean (combined Syseng + Rustacean profile, introduced this
phase, addressed directly by @@Architect)
Status: REVIEW
Depends on: [backend-1](./backend-1.md) and [frontend-1](./frontend-1.md)
landing the user-facing removals first.
Coordinates with: [frontend-2](./frontend-2.md) on shared dependency
removal order; @@Backend for chan-server `Cargo.toml` feature pruning.

## Goal

With Alex's authorisation to delete chan-drive's `*_assistant` blob API
(see [journal](./journal.md), decision 2) and to delete chan-llm's
in-app agent session + CLI backends (decision 3), remove the dead code
end to end and make the pre-push gate green on a single coherent diff.

## Acceptance criteria

### chan-llm prune

Delete (after confirming no chan-server / chan / desktop call sites
remain via `cargo check --all-targets`):

* `crates/chan-llm/src/session.rs` (the `LlmSession` type and listener
  trait that backed the in-app agent).
* `crates/chan-llm/src/backends/claude_cli.rs`,
  `crates/chan-llm/src/backends/codex_cli.rs`,
  `crates/chan-llm/src/backends/gemini_cli.rs`,
  `crates/chan-llm/src/backends/subprocess_env.rs`.
* `crates/chan-llm/src/backends/mod.rs` if nothing in `mcp.rs` /
  `tools.rs` / `prompts.rs` consumes it. If it does, pare to the
  minimum.
* `cli.rs` and `config.rs` entries that only existed to detect /
  configure those CLI backends. Keep MCP key resolution.
* `error.rs` variants that no longer have producers.
* Update `crates/chan-llm/src/lib.rs` re-exports accordingly.
* `crates/chan-llm/README.md` reflects "MCP server only" surface.

### chan-drive prune

Delete (after confirming no callers in this repo and grep shows only
tests using these helpers):

* `crates/chan-drive/src/drive.rs`: `put_assistant`, `get_assistant`,
  `list_assistant`, `delete_assistant`, `clear_assistant`.
* Tests in `drive.rs` and `library.rs` that exercise those helpers.
* Any on-disk `.chan/assistant/` directory documentation or
  reservation in `paths.rs` / state-dir constants. Migration: nothing
  active to migrate (frontend overlay is gone); record this in the
  task progress notes.

### chan-server tidy after the strip

* Make sure `crates/chan-server/Cargo.toml` does not pull
  unused chan-llm features / dependencies.
* Confirm `crates/chan-server/src/mcp_bridge.rs` still compiles and
  passes its tests (the MCP server stays).

### Build green gate

Pre-push checklist run from repo root:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --no-default-features
cargo test
cd web && npm run check && npm test -- --run && npm run build
```

All must pass on the merged wave-1 + this task's diff.

### Rust quality review

Read [backend-1](./backend-1.md) and [frontend-2](./frontend-2.md) diffs
before commit. Look for:

* Unused imports / variables Rust didn't warn on because the file got
  fully deleted.
* `pub` exports that became internal-only and should drop visibility.
* Dependency rows in `Cargo.toml` that no longer have consumers.
* Error variants that no longer have producers.
* Tests that became no-ops.

## Test expectations

* `crates/chan-drive` keeps its existing tests for the surviving APIs;
  only the `*_assistant` tests are deleted.
* If `chan-llm` had unit tests for `LlmSession` they go with `session.rs`.
* MCP server tests (`mcp.rs` / `tools.rs` / `prompts.rs` coverage) stay
  and must keep passing.

## Hardening expectations

* @@Systacean re-reads `crates/chan-server/src/mcp_bridge.rs` after the
  prune to verify the in-process MCP server still binds, accepts,
  shuts down on drop, and unlinks the socket.
* Flag any place where the chan-drive prune touches a public API that
  may have downstream consumers (mobile uniffi shells, external
  scripts). Surface to @@Architect before committing.

## Progress

* 2026-05-17 @@Systacean: picked up after [backend-1](./backend-1.md)
  and [frontend-1](./frontend-1.md) reached REVIEW. Starting with a
  grep/build-guided prune; MCP bridge and `chan __mcp` are explicitly
  preserved.
* 2026-05-17 @@Systacean: removed chan-llm's in-app session/CLI
  backend surface (`session.rs`, `cli.rs`, `config.rs`, backend
  modules, bench, reexports, config/error variants), leaving prompts,
  tools, and MCP server support.
* 2026-05-17 @@Systacean: removed chan-drive assistant blob helpers and
  state-dir reservation. `reset_drive` progress now emits four state
  subsystem events because the assistant bucket is gone. No migration is
  needed; the frontend overlay/history surface no longer reads those
  blobs.
* 2026-05-17 @@Systacean: reviewed the MCP bridge after the prune. It
  still hosts `chan_llm::mcp::Server` in-process over a Unix-domain
  socket and unlinks the socket on handle drop.
* 2026-05-17 @@Systacean: completed [frontend-2](./frontend-2.md)
  residue required for the full gate: removed stale store/API assistant
  references and rewrote store tests around graph hash/watch behavior.

## Completion notes

All requested gate commands passed from the repo root:

* `cargo fmt --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo build --no-default-features`
* `cargo test`
* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build`

Final greps for removed Rust surfaces and web assistant/LLM API/store
symbols returned no matches in the checked live code paths.
