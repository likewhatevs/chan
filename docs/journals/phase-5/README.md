# Phase 5 - MCP-only refactor and persistent terminals

Status: closed
Span: 2026-05-17, one working day (estimate; see Duration)

Tags: #refactor #mcp #terminal #indexing #bugfixes #release

## Initial asks

Source: `raw/request.md`. Alex framed the phase as "a
multi-step move, starting from big clean up and then drilling into
smaller bugs and feature requests", and introduced @@Systacean as a
combined Syseng + Rustacean profile. The checklist:

- Cleanup: merge the `../chan-term` work onto main first; remove the
  in-app Agent overlay and Agent-history overlay across frontend and
  backend; strip the agent and history backends from chan-llm and global
  config, while preserving the MCP server and external-agent access to
  the drive.
- Enhancements: the embedded terminal should export environment variables
  so common agents (claude, codex, gemini) can reach chan's MCP server;
  tune boot and in-flight resource use (graph and reports ahead of the
  search index, with a configurable aggression knob); detect sudden
  filesystem changes such as git checkouts (VCS-aware indexing) with
  correctness tests and benchmarks; back terminal tabs with tmux `-CC` so
  they survive reloads.
- Bug fixes: confirmation when closing tabs with unsaved files or live
  terminals; per-window desktop pane/tab state; an editor over-scroll fix
  when the cursor is at the top of a screen-sized page.

## Team, profiles, and coordination

Six slots, confirmed with Alex. Cards under `../../agents/`, mapped via
[../../agents/README.md](../../agents/README.md).

```
handle        role this phase                          card
------------  --------------------------------------   -----------------
@@Architect   plan, dispatch, the tmux design memo,    architect.md
              docs sweep, commit grouping, summary
@@Backend     agent surface removal, terminal MCP      backend.md
              env, per-window session keys             (-> FullStack A/B)
@@Frontend    overlay removal, close-confirm + scroll  frontend.md
              bundle, persistent-terminal client       (-> FullStack A/B)
@@Systacean   new combined profile; highest output:    systacean.md
              chan-llm prune, indexer, PTY registry,
              VCS-aware indexing, alt-screen fixes
@@Webtest A   primary live smoke, PTY reattach matrix  webtest-a.md
@@Webtest B   parallel scenarios on the shared server  webtest-b.md
```

Coordination scheme: flat task files at the phase root named
`{agent}-{n}.md`, dispatched by the architect through a single central
`journal.md`. The log advanced in numbered housekeeping rounds (1 through
14) where the architect reconciled in-flight work into the dispatch
table. There were no per-author directories and no separate event-channel
files. As in the neighboring phases, wave-1 cleanup was already in flight
before the architect finished orientation, so the journal was reconciled
mid-stream.

## Duration

Estimate: a single working day, 2026-05-17. Basis: every dated header in
the journals reads 2026-05-17, and the phase-close commits carry the same
date; the 2026-05-18 git date is the later relocation.

## Highlights and lowlights

Highlights:
- The terminal-persistence decision pivoted cleanly: Alex's chan-native
  PTY session registry (Option 4) replaced all three tmux options,
  holding the single-binary, no-runtime-deps line at a fraction of the
  cost.
- The VCS-aware indexing change produced concrete benchmark numbers (80
  files, 20 touched: initial 11078 ms, checkout settle 3138 ms, staged
  resume 235 ms), making future regressions measurable.
- A screenshot-diff bar for TUI reattach became the canonical acceptance
  shape and caught a partial-redraw regression.

Lowlights:
- Architect orientation lagged the first wave; three housekeeping rounds
  went into reconciling status drift before dispatch ran clean.
- Stale "sibling chan-core" framing in CLAUDE.md and design.md was
  load-bearing and would have made agents reason wrongly about the
  layout; caught and rewritten.
- The rebuild cycle (npm build, cargo build, restart; rust-embed has no
  hot reload) consumed real wall time on every re-smoke.

## Constructive feedback

- A test-server smoke should be filed as "preliminary" until it is
  compared against a fresh-launch baseline; a premature "PASS" cost a
  round here.
- During design, ask "how does this trigger the structural repaint, not
  just the cell refresh?"; the no-op-resize choice was reasonable but the
  redraw failure was knowable from documented signal behavior.
- Read the bootstrap/mount lifecycle before patching a hydration race;
  the late hydration call was caught by a network trace.
- Ship the `CHAN_`-only env shape first and add CLI-flavoured aliases
  only if a user needs them.
- Architect self-note: skim the process doc and list the phase directory
  before reading the request so the dispatch loop starts at minute one.

## What shipped, tried, and undone

Shipped:
- The `../chan-term` terminal work finalized on main.
- The in-app Agent and Agent-history overlays removed end to end
  (frontend components, store, types, settings, and the backend
  `/api/llm/*`, `/api/assistant/*`, `/api/answers`, `llm.toml`, and LLM
  websocket events).
- chan-llm pared to MCP-only: dropped the session and CLI/mock/ndjson
  backends and their config; the MCP server and external-agent drive
  access preserved. The workspace `*_assistant` blob API removed.
- The embedded terminal exports MCP discovery env per session, using a
  `CHAN_`-only namespace plus an `mcp_env=on|off` query parameter.
- Indexer scheduling tightened (graph and reports ahead of search;
  deletions before upserts), a `[search].aggression` knob, and
  git/hg-aware indexing with checkout detection and staged-row resume.
- Persistent terminal sessions: a chan-native PTY registry with a
  byte-offset replay ring, idle prune, a soft cap, and client reattach by
  session id and sequence. Terminal Alt-key word motions.
- Bug fixes: confirm-on-close for dirty or live tabs, per-window desktop
  session keys, and the over-scroll fix.

Tried then abandoned or superseded:
- tmux `-CC` native integration (the literal request) was not built. The
  design memo sketched three external-tmux options; all were superseded
  by the chan-native registry, and external tmux-client compatibility was
  dropped as a non-goal.
- The `CLAUDE_/CODEX_/GEMINI_` env aliases shipped first were dropped in
  favor of `CHAN_`-only, to avoid colliding with user wrappers.
- The first hydration-race fix fired too late and was superseded by
  inverting the bootstrap order (fetch the session blob, then restore
  layout); the original patch stayed as the building block.
- The first alt-screen reattach fix was insufficient (a wrong early
  "PASS" was caught visually) and was superseded by cross-chunk-safe
  alt-screen sniffing and a real winsize wobble.

Deferred follow-ups: a UI for the aggression knob, a true PATCH for the
config route, real CLI end-to-end MCP validation on a host with those
CLIs, and re-rendering plain bash scrollback on reload (alt-screen TUIs
are fine).

## Raw material

Raw working material (per-author journals, task/request/roadmap files,
coordination logs) is preserved in git history under this phase's `raw/`
tree; it was removed from the working tree in the phase-15 docs cleanup.
