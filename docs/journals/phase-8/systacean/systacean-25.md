# systacean-25 — chan-drive Drafts indexer + watcher + graph emit integration (followup to systacean-24)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Complete the Drafts integration started in
`systacean-24` (foundation). Wire the Drafts subtree
into the watcher, indexer, and graph emit so Drafts
content participates in search + graph view per
`addendun-a.md`'s spec.

## Reference

* Foundation: `systacean-24` (filesystem primitive +
  `Drive::drafts_dir / create_draft_dir / list_drafts
  / promote_draft` API).
* Scope-poke + recommended slice:
  [`../systacean/systacean-24.md`](../systacean/systacean-24.md)
  tail "## Why I'm staging this".
* Spec: [`../alex/addendun-a.md`](../alex/addendun-a.md)
  "### Extra" — drafts indexed; graph carries Drafts
  root with distinct edge.

## Decisions (routed by @@Architect per -24 scope-poke)

ACCEPT @@Systacean's recommended slice verbatim:

### 1. Path namespace: (i) — unified keyspace with `Drafts/` prefix

* `Drafts/<name>/...` prefix in the SAME BM25 +
  graph keyspace as drive content.
* Pro: reuses existing search + graph code paths;
  single index.
* Con: in-practice non-colliding since capital-D
  `Drafts/` at drive root is unusual; document the
  caveat in the indexer module.

### 2. Watcher: (i) — `WatchHandle::start` accepts multiple roots

* Modify `WatchHandle::start` to accept a list of
  paths.
* Existing callers pass `[drive_root]`; new code
  passes `[drive_root, drafts_dir]`.
* Each event carries its origin so the indexer can
  prefix correctly.

### 3. Graph emit: (iii) — chan-drive emits, chan-server synthesizes Drafts root

* chan-drive emits per-file `contains` edges under
  the `Drafts/` prefix as usual.
* chan-server graph route synthesizes the special
  "Drafts root" node + distinct edge attribute
  (e.g. `kind: "drafts_link"`).
* Smallest cross-lane change; keeps chan-drive
  emit logic simple.

## Scope

* Watcher: `WatchHandle::start([drive_root,
  drafts_dir], ...)` shape.
* Indexer: walks the `[drive_root, drafts_dir]` set;
  emits BM25 docs + graph nodes under unified
  keyspace.
* chan-server graph route: synthesizes the
  "Drafts root" node + `kind: "drafts_link"` (or
  equivalent attribute) edge from drive root → Drafts
  root. SPA can style differently per `addendun-a.md`.

## Acceptance

Original `-24` criteria #4 / #5 / #6:

4. Watcher emits events for Drafts subtree changes.
5. Indexer includes Drafts content in search results
   (BM25 returns Drafts file hits).
6. Graph emit (server side) renders Drafts root
   distinctly + edge attribute distinguishes from
   regular `contains`.

Plus:

7. Existing drive-content indexing unchanged
   (regression check).
8. SPA can read the new edge attribute via
   `/api/graph?scope=drive` (verifiable through
   curl + JSON inspection).

### Tests

* Watcher: multi-root start + per-event origin
  tagging.
* Indexer: round-trip — create draft file → search
  hit returned.
* chan-server graph route: response includes
  Drafts root node + distinct edge attribute.

### Gate

* `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test -p chan-drive` +
  `cargo test -p chan-server` green.
* `RUSTFLAGS="-D warnings" cargo build
  --no-default-features` green.
* CI smoke via `gh workflow run ci.yml --ref
  systacean-25-smoke`.

## Coordination

* @@Systacean lane. chan-drive watcher + indexer +
  chan-server graph route. The graph-route piece is
  cross-component but stays in your scope per the
  routed decision (iii).
* `fullstack-a-66` (SPA New Draft) is the consumer —
  their FB rendering + graph styling consumes the
  new edge attribute.

## Authorization

**Yes** for `crates/chan-drive/src/{watcher,indexer}.rs`
(or wherever those live) + `crates/chan-server/src/routes/graph.rs`
(for the Drafts root synthesis) + tests + task tail +
outbound. The chan-server graph route edit is
explicitly authorized here per the routed (iii)
decision; no further scope-poke needed.

## Numbering

This is `-25`.

## Out of scope

* SPA-side FB rendering of Drafts (separate task
  `fullstack-a-66`).
* SPA-side graph styling of the Drafts root /
  edge attribute (also `fullstack-a-66`).
* Per-draft contact/mention extraction beyond
  what already runs on drive content (uniform
  treatment per the routed (i) decision).
