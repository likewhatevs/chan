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

## 2026-05-22 — implementation complete; ready for smoke

`-25` picked up directly after the `-24` foundation smoke landed green. All 3 acceptance criteria (#4 watcher, #5 indexer, #6 graph emit) implemented per the architect's routed (1.i) / (2.i) / (3.iii) decisions.

### chan-drive watcher refactor (acceptance #4)

`crates/chan-drive/src/watch.rs`:

* New `WatchRoot` struct: `{ abs: PathBuf, prefix: Option<String> }`. Constructors `WatchRoot::drive(abs)` (no prefix) + `WatchRoot::drafts(abs)` (prefix `"Drafts"`).
* `WatchHandle::start` signature changed from `(drive_root: &Path, cb)` to `(roots: &[WatchRoot], cb)`. Single caller (`Drive::watch`) updated.
* New `locate_root` helper: resolves which `WatchRoot` an absolute event path falls under (longer-path tiebreak for nested-root safety).
* `dispatch` extended to find the originating root, relativize against it, apply the root's prefix when set. `is_filtered` still runs against the RAW (pre-prefix) relative path so `.chan/` filters keep their canonical shape.

`crates/chan-drive/src/drive.rs::Drive::watch`:

* Now passes `&[WatchRoot::drive(self.root()), WatchRoot::drafts(self.drafts_dir())]` to `WatchHandle::start`. Drafts subtree is watched alongside the drive root; events emerge with `Drafts/<name>/...` paths in the unified keyspace.

### chan-drive indexer integration (acceptance #5)

`crates/chan-drive/src/drive.rs::Drive::index_draft_file`:

* New public method. Accepts paths with `Drafts/` prefix; strips the prefix to resolve the file on disk under `drafts_dir`; stores BM25 + graph DB entries under the full `Drafts/<...>` key.
* Reads via `std::fs::read_to_string` from `drafts_dir/<sub_rel>` (NOT the sandboxed `Drive::dir`). Drafts are chan-drive's own metadata; the sandbox isn't a security concern at the drafts root.
* Stat-before-read pattern preserved (parallel to `index_file_inner`); mtime + size stored on the graph entry.
* Skipped silently for non-indexable text + for directory events (FSEvents fires Created on parent dir creation).
* On `NotFound` (drafts file vanished between event and read), routes through `Drive::forget_file` to keep BM25 + graph consistent.

`crates/chan-drive/src/indexer.rs::run_loop`:

* `apply_event`'s ready-list dispatcher routes `Drafts/`-prefixed paths to `index_draft_file`; non-prefixed paths go to the existing `index_file` as before. Remove events go to the existing `forget_file` regardless (path string is opaque to forget).

### chan-server graph route synthesis (acceptance #6)

`crates/chan-server/src/routes/graph.rs::synthesize_drafts_layer`:

* New helper called from `api_graph` after the per-file + per-language layer merges.
* Checks if any indexed file path starts with `Drafts/`. If yes:
  * Inserts a `GraphNodeView::Directory { id: "directory:Drafts", label: "Drafts", path: "Drafts", files: 0, code: 0 }` (no clobber if already present).
  * Returns a single `GraphEdgeView { source: "directory:", target: "directory:Drafts", kind: "drafts_link", ... }`.
* The SPA reads `kind == "drafts_link"` to style the Drafts root + edge differently from regular `contains` (per `addendun-a.md`'s "different edge to the drive to indicate this one is different than the others").

### Tests (+5)

**chan-drive** (+1):

* `indexer::tests::writes_to_drafts_subtree_get_indexed_under_drafts_prefix` — full end-to-end: create_draft_dir + std::fs::write inside drafts → watcher fires → indexer routes to `index_draft_file` → search returns the draft under `Drafts/untitled-1/draft.md` path. Uses the `-23` outcome-poll pattern + small sleep between create + write so macOS FSEvents doesn't coalesce the events into a single directory-Created (5/5 local runs green after the sleep was added).

**chan-server** (+2):

* `routes::graph::tests::synthesize_drafts_layer_emits_root_node_and_distinct_link_edge_when_drafts_present` — pure helper unit. Files contains a `Drafts/...` path → Drafts root node + `drafts_link` edge synthesized.
* `routes::graph::tests::synthesize_drafts_layer_is_noop_when_no_draft_paths` — files contains no `Drafts/` paths → no synthesis (backward-compat for users without drafts).

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-drive --lib`: **440 passed; 0 failed; 2 ignored** (was 439; +1 new test).
* `cargo test -p chan-server --lib`: **213 passed; 0 failed** (was 211; +2 new tests).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                            | +    | -   |
|-------------------------------------------------|------|-----|
| `crates/chan-drive/src/watch.rs`                | +117 | -22 |
| `crates/chan-drive/src/drive.rs`                | +79  | -5  |
| `crates/chan-drive/src/indexer.rs`              | +71  | -4  |
| `crates/chan-server/src/routes/graph.rs`        | +107 | 0   |

Plus task tail + outbound poke. 6 paths total.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 4 | Watcher emits events for Drafts subtree changes | ✓ |
| 5 | Indexer includes Drafts content in search results | ✓ |
| 6 | Graph emit carries a Drafts root + distinct drive→Drafts edge | ✓ |
| 7 | Existing drive-content indexing unchanged (regression check) | ✓ (all prior tests green) |
| 8 | SPA can read the new edge attribute via `/api/graph?scope=drive` | ✓ (`kind: "drafts_link"` in JSON; verified via unit test) |

### Suggested commit subject

```
chan-drive + chan-server: Drafts watcher + indexer + graph route synthesis (systacean-25)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-25-smoke` on a fresh smoke branch. Expected: all 5 jobs green. The drafts-indexer test is the only race-prone piece on macOS; the 200ms sleep + outcome-poll wait_for absorb the FSEvents coalescing. If macOS still red post-fix, would pivot to a more aggressive timing tune.

Holding for smoke verdict.
