# @@Backend task 1

Owner: @@Backend
Status: REVIEW

## Goal

Start backend work for [request](./request.md), beginning with a merge plan for
`../chan-term` onto `main`, then backend cleanup around Agent/history surfaces
while preserving MCP drive access.

## Merge plan for `../chan-term`

Source: `../chan-term`

Current state:

* `../chan-term` is clean.
* It is detached at `963bade web: add terminal tab controls`.
* It contains three commits on top of `main`:
  * `0f4614e web: add terminal overlay`
  * `980fc3e web: move terminal into workspace tabs`
  * `963bade web: add terminal tab controls`
* Changed surface:
  * Backend: `crates/chan-server/src/routes/terminal.rs`, route wiring, CLI serve wiring.
  * Desktop: `desktop/src-tauri/src/serve.rs`.
  * Frontend: terminal tab component, pane/tab state, shortcuts, Vite/package deps.

Recommended merge path:

1. Keep the current `main` worktree clean except for phase coordination docs.
2. Bring the three terminal commits onto `main` as a normal merge or cherry-pick
   series before Agent/history cleanup lands.
3. Build-check backend first: `cargo check -p chan-server`.
4. Build-check frontend dependency state: `npm --prefix web install` if lockfile
   needs reconciliation, then `npm --prefix web test` or the smallest available
   frontend check.
5. Backend review points after merge:
   * `terminal.rs` process lifetime and cancellation behavior.
   * WebSocket close semantics for live terminal sessions.
   * Shell environment setup, especially Chan MCP variables requested later in
     [request](./request.md).
6. Frontend review points after merge:
   * Tab persistence/reload behavior.
   * Close-confirm behavior for terminal tabs and dirty editors.
   * Shortcut conflicts with removed Agent overlay bindings.

Risk notes:

* The terminal work touches tab state, which overlaps the Agent/history cleanup
  in `web/src/state/tabs.svelte.ts` and `web/src/App.svelte`; merge it first to
  avoid deleting or rewriting stale frontend code.
* The terminal route is a new backend surface and should get a targeted backend
  test or at least a manual WebSocket lifecycle check before release.

## Progress

* Read [request](./request.md) and [process](./process.md).
* Read `../chan-term` status and commit/file diff summary.
* Confirmed this worktree is already at `../chan-term` HEAD
  (`963bade`), so no merge command was needed.
* Removed chan-server's assistant HTTP surface:
  * deleted `/api/llm/*` route module and router wiring
  * deleted `/api/assistant/*` and `/api/answers` handlers/wiring
  * removed assistant preferences from `/api/config`
  * removed server-side `llm.toml` load/state
  * removed LLM event broadcasting from `/ws`
* Preserved the in-process MCP bridge.
* Added terminal MCP discovery environment variables:
  `CHAN_MCP_SERVER_NAME`, `CHAN_MCP_SOCKET`, `CHAN_MCP_COMMAND`,
  `CHAN_MCP_COMMAND_JSON`, `CHAN_MCP_SERVER_JSON`,
  `CLAUDE_MCP_SERVER_JSON`, `CODEX_MCP_SERVER_JSON`,
  `GEMINI_MCP_SERVER_JSON`.
* Updated README-level docs for the removed in-app assistant surface
  and terminal MCP env.
* Frontend type/settings cleanup:
  * removed assistant preferences from the `Preferences` wire type
  * removed the stale assistant pane-width config field
  * removed SettingsPanel's assistant preference normalization
  * removed unused `/api/llm/*` client methods/types
* Picked up [frontend-2](./frontend-2.md) residue after update check:
  removed the remaining assistant/scope-history state from the web store,
  removed stale LLM/assistant client/type exports, and rewrote the store
  tests around graph-only behavior.

## Verification

* `cargo check -p chan-server`
* `cargo test -p chan-server`
* `npm --prefix web run check`
* `npm --prefix web test`
* `npm --prefix web run build`

## Notes / follow-up

* chan-drive still has legacy assistant blob helpers and state-dir cleanup
  references. They are now disconnected from chan-server, but should be
  removed in a dedicated chan-drive cleanup pass with migration/delete
  semantics decided explicitly.
* chan-llm still contains the historical assistant backend code. The server
  only keeps the crate for MCP, but a deeper chan-llm pruning pass remains.
* The working tree already contained broad frontend Agent/assistant overlay
  deletions/edits outside the files I changed directly; I worked with that
  state and did not revert it.
