# task Lead -> LaneB (1): Graph

You are @@LaneB - Graph lane. Round-1, Wave 1. START NOW.

## Read first (context lives here, not in this poke)
- Process: docs/journals/phase-18/team/bootstrap.md
- Plan + your lane section + gate/quality bar + shared-file table:
  docs/journals/phase-18/team/round-1-plan.md  (section "@@LaneB - Graph")
- Verbatim spec: docs/journals/phase-18/round-1/draft.md  (section "### Graph")
- Images: round-1/image-5.png, image-10.png, image-12.png.
- Re-verify all line anchors against HEAD; they drift.

## Wave 1 scope (5 items)
1. "Graph from here" must SELECT the originating node (select is dropped today).
2. Directory nodes plotted with NO visible edge to workspace root (image-5,
   image-12). Parent-spine invariant not landing.
3. Binary/symlink rendered as a CONTACT node (image-10). Cross-crate: the Rust
   chan-workspace indexer node_kind stamp and the TS classifyFile mirror MUST
   stay in LOCKSTEP.
4. STOP the auto-reload (highest-signal bug): graph reloads on ANY workspace
   file edit, even files not in the current graph scope. Gate invalidation on
   whether the changed path is in the current graph scope.
5. Enhancement "Copy link to graph": Graph TAB right-click menu (replace
   "Reload") -> a link serializing GraphTab (scopeId/depth/mode/filters/selected),
   openable from a markdown file.

## Owned files (edit ONLY these)
web/src/components/{GraphPanel.svelte,GraphCanvas.svelte},
web/src/state/graphData.svelte.ts, web/src/state/store.svelte.ts (graph region
only, ~625-672 / 2054-2107 / 1915), web/src/state/tabs.svelte.ts (GraphTab),
web/src/state/tabMenu.svelte.ts (graph region), crates/chan-workspace indexer
(contact-stamp), crates/chan-server graph route (wire kinds).

## Shared-file rules (plan "Shared-file contention")
- store.svelte.ts: you = graph region; @@LaneC = persist region (far apart,
  .ts interleave-safe). @@Lead commits the merged file.
- tabs.svelte.ts / tabMenu.svelte.ts: coordinate if both add fields.
- crates/chan-server: graph route is yours, terminal_sessions.rs is @@LaneE's;
  same crate -> land any shared-signature change in one burst, re-`cargo check
  -p chan-server` green before pausing.

## Gate before any "done" report
Frontend: make web-check + svelte-check + npm run build (browser-smoke any
$state/$derived change). Rust: cargo fmt --check + cargo clippy --all-targets
-D warnings + cargo test (scoped -p chan-workspace and -p chan-server).

## On completion
Cut task-LaneB-Lead-1.md (own-gate-green + pathspec sha + per-item status),
poke me (--tab-name=@@Lead --submit=claude). Journal:
journal-LaneB.md. Flag ANY shared-file touch to @@Lead BEFORE landing.
