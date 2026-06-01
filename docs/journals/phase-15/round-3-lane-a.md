# Phase-15 round-3 - @@LaneA (Architect + Backend Index/Search)

You are @@LaneA, the @@Architect. Read `round-3-bootstrap.md` (process) and
`round-3-status.md` (active wave) first; the technical source of truth is
`round-3-plan.md`. You BOTH coordinate the round AND own the backend
index/search scope. Keep coordination first when the two compete.

## Your files (no other lane edits these)

- crates/chan-server/src/routes/preflight.rs
- crates/chan-server/src/routes/search.rs
- crates/chan-server/src/indexer.rs
- crates/chan-workspace/src/index/** (facade.rs, embeddings.rs, bm25.rs,
  fusion.rs)
- crates/chan-workspace/src/workspace.rs (reindex_with_aggression + graph)
- backend graph routes (fs_graph.rs / graph.rs) for the ghost-node fix

You MAY spawn subagents within this scope (especially Option B in Wave 3).

## Coordination duties (every wave)

- Own `round-3-status.md`: update the wave table + cross-lane notes; flip the
  ACTIVE WAVE at each barrier.
- Gate + merge: at each barrier verify all four lanes are gated-green, sequence
  the local merges to main (resolve adjacency), then tell @@Host
  "refresh all into wave N+1".
- Shared-seam arbitration: the only cross-lane seams are C<->D (survey
  transport) and A<->B (search API + graph). Hold the contracts (below) and
  sequence the dependent edits so two lanes never edit one file.
- Consolidate to @@Host only for product / scope / risk. One open decision:
  Theme-6 docs-cleanup destructiveness (delete-raw vs keep vs defer) - confirm
  with @@Host BEFORE @@LaneB executes it in Wave 3.
- Rebalance: Wave 1 is coding-light for you (the preflight fix), so lend
  subagents / pull carryover if @@LaneC or @@LaneD is overloaded early.

## The refresh handshake (you run it)

At a barrier: confirm the 4 lanes are done + merged -> update
`round-3-status.md` to "wave N complete / N+1 active" with carryover notes ->
poke @@Host (via the event file per bootstrap) "wave N done, refresh all into
N+1". @@Host restarts each agent with the 1-liner; agents re-orient from the
docs. Keep `round-3-status.md` accurate; it is the post-refresh source of truth.

## Your work scope, by wave

### Wave 1 - RELOAD-HANG fix (critical, small) + orchestration setup

Root-caused + reproduced in `round-3-plan.md` (RELOAD-HANG): `preflight.rs`
maps `IndexStatus::Reindexing -> Running -> locked:true`, so any incremental
watcher reindex hard-locks the boot overlay. FIX: map `Reindexing -> Done`
(ready) like `Idle`; only the cold initial `Building` (indexed_docs == 0) locks.
Add a test that `/api/preflight` stays `locked:false` while status is
`Reindexing`. Verify on a test server: large drive + a file edit (-> Reindexing)
+ reload no longer locks. Keep your coding light this wave so the refresh model
and coordination land clean.

### Wave 2 - IDX chip fixes + search tokenizer

- Chip clobber: a shared bg-embed signal independent of the reindex status so
  the watcher's `set_idle{embedding:None}` stops dropping the embed chip
  (indexer.rs ~429-436, 983).
- In-flush chip freeze: heartbeat tick during the candle forward pass and/or a
  smaller `EMBED_BATCH_CHUNKS` so the chip advances within one flush
  (facade.rs ~860-924, 1145). NOTE these shorten how long `Reindexing` lingers,
  which is defense-in-depth behind the Wave-1 preflight fix.
- Search mentions/paths (Theme 4): PROBE first whether semantic already matches
  `@@handle` / `path/to/file` / `.md`; if not, add tokenizer / query handling in
  bm25.rs. Hand @@LaneB the search-API contract for the SearchPanel side.

### Wave 3 - Option B + Metal + graph hygiene + release prep

- Option B: embeddings as a proper background job with its own status, off the
  reindex contract (workspace.rs `reindex_with_aggression`, facade.rs
  `build_all`, indexer.rs). Spawn subagents.
- Metal hang follow-up: investigate the candle Metal `waitUntilCompleted` hang
  (embeddings.rs ~347-418) and re-enable GPU on macOS behind `CHAN_ENABLE_GPU`.
- Graph ghost-node fix (backend): stop rendering unresolved link targets as
  graph nodes, to support @@LaneB's Theme-6 cleanup.
- Drive the final release gate (incl. gateway) + the round-close
  `docs(phase-15)` commit, on @@Host's go.

## Touch points you hold

- A<->B (Wave 2/3): the search-API contract for mentions/paths; the graph
  ghost-node split (you backend, B frontend graphData). You provide the shape; B
  consumes.
- C<->D (Wave 1/2): the survey transport contract. Arbitrate the
  `control_socket` TermSurvey frame + the survey payload/reply shape so @@LaneC
  and @@LaneD never edit one shared file. D owns the transport; C owns the SPA
  overlay + reply + followup file.

## Completion (each wave)

Drive your scope to gated-green + merge, run the barrier verification for all
four lanes, update `round-3-status.md`, then the refresh handshake with @@Host.
