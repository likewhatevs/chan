# Phase 5 - MCP-only refactor and persistent terminals

Status: closed
Span: 2026-05-17 (single working day; every dated journal header and phase-close commits carry this date; a 2026-05-18 git date reflects the later relocation of the working tree)
Versions: none cut this phase
Tags: #refactor #mcp #terminal #indexing #bugfixes #release

## Roadmap (the asks)

@@Alex framed the phase as "a multi-step move, starting from big clean up and then drilling into smaller bugs and feature requests". He introduced @@Systacean as a new combined Syseng + Rustacean profile for this work.

**Cleanup**
- Merge the sibling `chan-term` terminal work onto main.
- Remove the in-app Agent overlay and Agent-history overlay across the frontend and backend entirely.
- Strip the agent and history backends from chan-llm and global config, preserving only the MCP server and external-agent access to the drive.

**Enhancements**
- Export environment variables from embedded terminal sessions so common agent CLIs (claude, codex, gemini) can reach chan's MCP server without manual config.
- Tune boot and in-flight resource use: graph and reports scheduled ahead of the search index, with a configurable aggression knob.
- VCS-aware indexing: detect sudden filesystem changes such as git checkouts, with correctness tests and concrete benchmarks.
- Back terminal tabs with persistent sessions so they survive page reloads (the initial framing said "tmux -CC"; the final design diverged, see below).

**Bug fixes**
- Confirm before closing tabs with unsaved files or live terminals.
- Per-window desktop pane and tab state.
- Editor over-scroll fix when the cursor lands at the top of a screen-sized page.

## Rounds and waves

Single-round phase. The architect dispatched work in a flat sequence of housekeeping rounds (numbered 1 through 14 in the journal), reconciling in-flight status into the dispatch table as agents reported.

Wave 1 was the cleanup block (chan-term merge, Agent overlay removal, chan-llm prune) and was already in flight before the architect finished orientation, so the first several housekeeping rounds were spent reconciling mid-stream rather than driving forward from a clean state.

Wave 2 covered the enhancements (terminal MCP env, indexer tuning, VCS indexing, PTY registry) running roughly in parallel with the tail of wave
1.

Wave 3 was the bug-fix and smoke-test pass (confirm-on-close, per-window keys, over-scroll, PTY reattach matrix, alt-screen validation).

## Team and coordination

Six agent slots. See `../agents/README.md` for role cards.

```
handle        role this phase
------------  ---------------------------------------------------
@@Architect   plan, dispatch, tmux design memo, docs sweep,
              commit grouping, phase summary
@@Backend     agent-surface removal, terminal MCP env export,
              per-window desktop session keys
@@Frontend    overlay removal, close-confirm + scroll bundle,
              persistent-terminal client side
@@Systacean   new combined profile (Syseng + Rustacean);
              highest output volume: chan-llm prune, indexer
              scheduling, PTY registry, VCS-aware indexing,
              alt-screen fixes
@@Webtest A   primary live smoke, PTY reattach acceptance matrix
@@Webtest B   parallel scenarios on the shared test server
```

Coordination scheme: flat task files at the phase root, named `{agent}-{n}.md`, dispatched by the architect through a single central `journal.md`. No per-author directories and no separate event-channel files. The journal advanced through numbered housekeeping rounds where the architect reconciled status drift and re-dispatched blocked work.

## What shipped, tried, and undone

**Shipped**

- The chan-term terminal work merged and finalized on main.
- In-app Agent and Agent-history overlays removed end to end: frontend components, store, types, settings, and the backend `/api/llm/*`, `/api/assistant/*`, `/api/answers`, `llm.toml`, and LLM websocket events.
- chan-llm pared to MCP-only: dropped the session and CLI/mock/ndjson backends and their config; the MCP server and external-agent drive access preserved. The workspace `*_assistant` blob API removed.
- The embedded terminal exports MCP discovery env per session, using a `CHAN_`-only namespace (e.g. `CHAN_MCP_SERVER_JSON`, `CHAN_MCP_*` discovery variables) plus an `mcp_env=on|off` query parameter.
- Indexer scheduling tightened: graph and reports run before search; deletions processed before upserts; a `[search].aggression` knob added; git/hg-aware indexing with checkout detection and staged-row resume. Benchmark numbers for the VCS path: 80 files, 20 touched; initial index 11078 ms, post-checkout settle 3138 ms, staged-resume re-index 235 ms.
- Persistent terminal sessions: a chan-native PTY registry with a byte-offset replay ring, idle prune, a soft cap, and client reattach by session id and sequence.
- Terminal Alt-key word motion support.
- Bug fixes: confirm-on-close for dirty or live tabs, per-window desktop session keys, the editor over-scroll fix.

**Tried then abandoned or superseded**

- tmux `-CC` native integration (the literal request) was not built. The architect wrote a design memo sketching three external-tmux options; all three were superseded by the chan-native PTY registry. External tmux-client compatibility was dropped as a non-goal.
- `CLAUDE_/CODEX_/GEMINI_` env aliases shipped first, then dropped in favor of `CHAN_`-only to avoid colliding with user-managed wrappers.
- The first hydration-race fix fired too late; superseded by inverting the bootstrap order (fetch the session blob, then restore layout). The original patch survived as a building block.
- The first alt-screen reattach fix was insufficient. A premature "PASS" was caught visually and the fix was superseded by cross-chunk-safe alt-screen sniffing and a real winsize wobble trigger.

**Deferred**

- A UI surface for the `[search].aggression` knob.
- A true PATCH verb for the config route.
- Real CLI end-to-end MCP validation on a host with claude/codex/gemini installed.
- Re-rendering plain bash scrollback on reload (alt-screen TUIs are handled; plain scrollback is not).

## Retrospective

**Highlights**

- The terminal-persistence design decision pivoted cleanly at the right moment. Alex's chan-native PTY session registry (described as Option 4 in the design memo) replaced all three tmux variants and held the single-binary, no-runtime-deps line at a fraction of the integration cost.
- VCS-aware indexing shipped with concrete benchmark numbers, making future regressions measurable rather than impressionistic.
- A screenshot-diff bar for TUI reattach became the canonical acceptance shape for this class of fix and caught a partial-redraw regression that a cell-level check would have missed.

**Lowlights and contention**

- Architect orientation lagged the first wave. Three housekeeping rounds went into reconciling status drift before dispatch ran clean. The architect's own retrospective note: skim the process doc and list the phase directory before reading the request so the dispatch loop starts at minute one.
- Stale "sibling chan-core" framing in CLAUDE.md and design.md was load-bearing inaccuracy. Agents reasoning from it would have produced wrong crate boundaries; caught and rewritten mid-phase.
- The rebuild cycle (npm build, cargo build, server restart) consumed real wall time on every re-smoke because rust-embed has no hot-reload path.

**Constructive feedback and lessons**

- File a test-server smoke as "preliminary" until it is compared against a fresh-launch baseline. A premature "PASS" on the alt-screen reattach cost a full housekeeping round.
- When designing a redraw fix, ask "what triggers the structural repaint, not just the cell refresh?" The no-op-resize choice was reasonable, but the redraw failure was knowable from documented PTY signal behavior before the test.
- Read the bootstrap and mount lifecycle before patching a hydration race. The late hydration call was caught by a network trace; reading the lifecycle first would have flagged it at design time.
- Ship the `CHAN_`-only env shape first; add CLI-flavoured aliases only if a user files a concrete need. Shipping them speculatively created churn.

## Notes

Terminology drift: this phase used "drive" to mean the workspace directory (later renamed to "chan-workspace"); "chan-core" referred to a sibling repository that had already been collapsed into this repo before the phase ran (the stale framing was corrected mid-phase). "Agent overlay" referred to the in-app LLM chat surface that was removed.

Raw working material (per-author task files, request, roadmap, and the architect's journal) lives in git history under `docs/journals/phase-5/`; that tree was removed from the working tree during the phase-15 docs cleanup.
