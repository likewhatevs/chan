# task-Conductor-PromptQueue-2 — item 2: Rich Prompt queue visibility

From: @@Conductor. To: @@PromptQueue. Cut: 2026-06-12.

## Scope

Item 2 end-to-end: a Rich Prompt message stays visible until the
agent consumes it, plus a queue-depth badge (queue is shared with
`cs terminal write` pokes).

Design (read fully — wire contract, state machine, sequencing all
inside): new-team-2/designs/item-2-prompt-queue-visibility.md.
Line numbers from main @ 3ebee587 — verify before editing.

## Sequencing (binding)

- Server half FIRST (terminal_sessions.rs + routes/terminal.rs).
  @@CtxPass's wave 4 is gated behind your terminal_sessions.rs
  changes — land the server half promptly and poke me the sha
  (1 line; this is a deliberate milestone poke, it also lets me route
  the cross-review without stalling you).
- chan-server is THREE-lane hot this round (@@you, @@CtxPass,
  @@TeamFlow). Signature change + ALL call sites in one burst;
  `cargo check -p chan-server` green before pausing; announce
  multi-file Rust bursts in your journal.
- Web half second. The Pane.svelte badge edit (design § Pane badge)
  WAITS: do not touch Pane.svelte until I poke you that @@Editor's
  restructure landed. Everything else in the web half is yours to do
  meanwhile.

## Gate

- Rust own-gate with REAL flags: RUSTFLAGS="-D warnings" on clippy
  AND test, scoped to chan-server. Web half: `make web-check`
  (vitest) + svelte-check + build. Re-run after the FINAL edit.
- Browser-smoke the RichPrompt state machine (runtime reactivity —
  static gates miss state_unsafe_mutation-class errors).
- Manual recipe from the design doc (busy-agent loop, cs write ×3,
  gemini 2-write rule, reload mid-pending, idle fast path) on a
  throwaway `chan serve --standalone` workspace; pkill scoped to your
  own path/port; NEVER restart the live serving binary.
- Regression check: `cs terminal write` stdout (queued/full/position)
  and cap 100 byte-for-byte unchanged.
- Subagents allowed (e.g. vitest coverage) — review their diffs fully
  before committing (round-1 lesson).
- Commits pathspec-atomic: `git commit -F <msg-file> -- <paths>`;
  staged-stat before, show-stat after.

## Review pairing

- @@CtxPass reviews your chan-server half (I route it on your
  milestone poke).
- You review @@CtxPass's B1 waves field-by-field (round-1 standard)
  when I route them.

## Completion

Milestone poke after the server half (sha). Then ONE completion poke
after the web half, with new-team-2/tasks/task-PromptQueue-Conductor-<n>.md:
shas, gate results, manual-recipe evidence, regression checks,
WKWebView-pending items. Journal: journals/journal-PromptQueue.md,
append-only.
