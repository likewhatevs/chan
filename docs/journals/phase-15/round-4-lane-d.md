# Phase-15 round-4 - @@LaneD (semantic wiring + phase-8 docs)

You are @@LaneD. Read `round-4-bootstrap.md` -> `round-4-status.md` -> this
file -> `round-4-plan.md` (grounded anchors). You own two small, disjoint
workstreams: the semantic-search wiring (backend) and the phase-8 docs cleanup
(docs). Do the semantic wiring FIRST (it lands fast), then the docs.

## Your files (no other lane edits these)

- `crates/chan-server/src/routes/search.rs` (the HTTP search route)
- `crates/chan/src/main.rs` (`cmd_search`, the CLI search)
- `docs/journals/phase-8/` (the essence README + the raw deletion)
- `docs/agents/` (`desktect.md`, `bootstrap.md` - the citation handling)

Disjoint from C (chan-shell + control_socket/team_config) and B (build infra).

## Workstream 1 - semantic search behind `semantic_enabled` (SMALL, Wave 1)

Grounded state (see round-4-plan.md): `semantic_enabled` already persists
(`index/config.rs` ~155, default false) + toggles
(`Workspace::set_semantic_enabled`, `/api/index/semantic/*`); dense vectors +
the Hybrid/RRF path (`facade.rs` ~1088) already work. The route
(`routes/search.rs` ~180) hardcodes `..Default::default()` = `Mode::Bm25` and
NEVER reads the flag; the "defaults to Hybrid" comment (~183) is STALE; the
empty-query mode (~171) hardcodes "hybrid". `cmd_search` (`main.rs` ~1779)
also hardcodes default.

Task:
- In the route, read `workspace.semantic_enabled()` (+ a model-present check,
  mirroring `routes/index.rs` ~144 / `embeddings::resolve_model`). Request
  `Mode::Hybrid` when the flag is on AND the model is present, else `Mode::
  Bm25`. Fix the stale comment + the empty-query mode string.
- Mirror the same logic in `cmd_search` (CLI parity).
- Add a test: the route requests Hybrid when `semantic_enabled` is on (and
  falls back to Bm25 / `ready:false` when the model is absent).
- DO NOT touch facade.rs / config.rs / the indexer - the infra is already
  correct; you only change the mode-selection decision point.

Verify (live probe): serve a drive; with `semantic_enabled` OFF a query
reports `mode=bm25`; toggle it ON (model present) and the same query reports
`mode=hybrid`. ~20-30 lines.

## Workstream 2 - phase-8 docs cleanup (@@Host-sequenced; destructive LAST)

phases 1-7, 9-14 already cleaned (raw dropped, essence READMEs, a930a96f).
phase-8 was deferred because `docs/agents/` cites it.

Wave 1 (non-destructive):
- Synthesize the phase-8 essence README in the phases-1-13 shape (read the
  phase-8 raw to summarize: the desktop-native vision, the round-2 work; tag
  outcomes with #hashtags).
- Repoint `docs/agents/desktect.md`'s 3 links (`phase-9-desktop-native-
  vision.md`, `event-architect-desktect.md`, `process.md` - already broken,
  pre-`raw/` paths) to the new essence README (or a git-history note).
- Decide `docs/agents/bootstrap.md`'s template-phase handling: it uses
  `phase-8` as an EXAMPLE path throughout ("phase-8 for now; update as we roll
  forward"), NOT live cites. Leave it as an illustrative example (recommended)
  or bump the example phase; do NOT blanket-repoint it.

Wave 2 (destructive, after the repoint lands):
- Delete phase-8 `raw/`. Verify the chan-source graph then shows no phase-8
  ghost nodes and desktect.md's links resolve.

## Your work scope, by wave

- Wave 1: land the semantic wiring (gated + live-probed) + the phase-8 essence
  README + the citation repoint. Poke @@Architect "wave 1 done".
- Wave 2: delete phase-8 raw (after the repoint is merged). Poke @@Architect
  "wave 2 done".

## Completion (each wave)

Drive your files to gated-green + merge (pathspec commits), write your journal
(`round-4-lane-d-journal.md`), poke @@Architect. The semantic change is
browser/CLI-probe-verifiable in this environment; do it.
