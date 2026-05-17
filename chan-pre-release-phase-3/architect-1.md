# architect-1: @@Backend+Rustacean idle, ready for more

Owner: @@Architect.

Status: TODO.

Related:

- [journal.md](./journal.md)
- [backend-1.md](./backend-1.md)
- [backend-2.md](./backend-2.md)

## From @@Backend+Rustacean

Both backend tasks moved to REVIEW:

- [backend-1](./backend-1.md) — Agent naming / CLI resume / status routing.
  Rust change landed: rename "assistant config" -> "agent config" error
  context in `crates/chan/src/main.rs` (3 occurrences). All other items are
  either non-issues on the backend side or coordination notes for
  [frontend-1](./frontend-1.md).
- [backend-2](./backend-2.md) — Graph data / URL-state audit. No code
  changes; documented that existing endpoints already cover every phase-3
  requirement.

Build state: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test -p chan -p chan-server -p chan-llm` all green.

## Asks

1. Schedule @@Rustacean review on [backend-1](./backend-1.md) (Rust naming /
   test quality on the small rename).
2. Confirm with @@Frontend that the CODEx-on-CLAUDE handoff in backend-1
   reaches them; the fix sits at
   `web/src/components/InlineAssist.svelte:624` and/or in store.svelte.ts
   near `currentAssistantConversation` (line 1926).
3. Assign next work. @@Backend+Rustacean is idle.

## Possible next tasks (suggestions)

- Coordinate the SERVE_LONG_ABOUT regen with [frontend-1](./frontend-1.md):
  once shortcuts.ts label flips to "Agent", run
  `node web/scripts/shortcuts-table.mjs --serve-long-about` and paste between
  the BEGIN/END markers in `crates/chan/src/main.rs`. This is a 1-line
  paste; could be folded into frontend-1's commit.
- If a phase 3 follow-up needs server-side help — e.g., a new
  `/api/graph/folders?path=...` helper if folder-overlay benchmarks reveal
  /api/graph + /api/fs-graph double-fetch is too expensive — @@Backend+Rustacean is
  ready to pick that up.

## 2026-05-16 role update

Alex reassigned the backend slot to Backend+Rustacean for the rest of phase 3.
Use this slot for backend/Rust implementation plus Rust quality review on its
own changes, unless @@Architect explicitly requests a separate @@Rustacean pass.
