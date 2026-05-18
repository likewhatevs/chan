# backsystacean-9: fold filesystem + language layers into /api/graph

Owner: @@Backsystacean
Status: REVIEW

## Goal

Make the filesystem the primary layer of the graph by merging the
fs-graph + language-graph data into the main `/api/graph` response.
The overlay renders one endpoint and gets the layered view that
[request.md](./request.md) asked for.

Headline ask:

> We are going to fix the way we index and plot the graph. From now
> on, the primary layer of the graph is the filesystem, starting
> from the drive. ... Language binds to directory in the graph.

## Background

Live test service shows the gap: `/api/graph` with `scope=drive`
returns 14/17 markdown-centric nodes (link / tag / contact) and
zero directory, file, language, or media nodes. The chip filters
already expose `language`, `media`, `folder` (-> `directory` after
the codemod), so the frontend supports rendering them; the
producer is missing.

Existing pieces to fold in:

* `/api/fs-graph` (`crates/chan-server/src/routes/fs_graph.rs`):
  directory + file nodes, classifier metadata (`path_class`,
  permission, link count, symlink-escape flag), dead-end semantics
  for read-only / special files. Already REVIEW from
  [backsystacean-2](./backsystacean-2.md) and tightened by
  [backsystacean-8](./backsystacean-8.md).
* `/api/language-graph` (`build_language_graph` in
  `crates/chan-server/src/routes/graph.rs`): language nodes +
  language-to-directory edges drawn from chan-report's
  `ReportFileStats`.
* `/api/graph` (the semantic graph today): emits link / tag /
  mention / contact nodes from markdown files.

## Relevant links

* Request: [request.md](./request.md)
* Design memo: [architect-2.md](./architect-2.md)
* Journal: [journal.md](./journal.md)
* Source: `crates/chan-server/src/routes/graph.rs`,
  `crates/chan-server/src/routes/fs_graph.rs`,
  `crates/chan-drive/` (graph + index).

## Scope

### Merged response shape

`/api/graph` (default `scope=drive`) returns:

* Nodes: existing kinds (`link`, `tag`, `mention`/`contact`) +
  new kinds (`directory`, `file`, `language`, `media`). The
  `media` kind comes from `path_class` + file-extension sniff
  consistent with `/api/inspector`'s `media` bucket.
* Edges:
  * Existing: cross-link, tag, mention edges.
  * New: directory contains file, directory contains directory
    (filesystem parent edges), language to directory (from
    chan-report rollups).
* Each node carries the minimum metadata the inspector + graph
  rendering need: id, kind, label, path, plus `path_class`
  (file/directory nodes) and language name (language nodes).

The response shape stays additive: existing consumers keep working
because the existing node kinds and edge fields are unchanged.

### Layering and filters

* Filesystem is the **primary** layer. When the overlay opens
  with no filter, the user sees the fs spine (drive root +
  directories + files) plus the markdown + language layers on
  top.
* Filter chips can hide layers selectively:
  * Off `directory` + `file` -> hide filesystem nodes, leaving
    the markdown layer.
  * Off `link` + `tag` + `contact` -> hide markdown layer,
    leaving filesystem + language.
  * Off `language` -> hide language nodes only.
* Read-only directory dead-end semantics from
  [backsystacean-2](./backsystacean-2.md) carry through: no
  subtree edges out of read-only dirs.
* Symlinks pointing outside the drive: render the node, do not
  traverse.

### Scope and depth

* `scope=drive` returns the full drive; respect the existing
  fs-graph + language-graph budgets so the response stays under
  the latency floor measured in
  [webtest-1](./webtest-1.md) round 2's 298-file probe (~10 ms
  warm).
* Per-directory and per-file scopes pivot around that node and
  return its subtree + neighbors. Same `depth` param shape as
  fs-graph today.

### Backward compat

* Existing /api/fs-graph and /api/language-graph routes stay
  available for callers that already use them.
* No new top-level routes needed.
* Response field additions are tolerated by serde-default
  consumers; no client breakage expected.

## Out of scope

* Removing /api/fs-graph or /api/language-graph (deferred; if
  /api/graph subsumes them in practice, a later phase can
  collapse).
* New visual styling for the new node kinds beyond what
  [frontend-4](./frontend-4.md) already wired through `path_class`
  and the royal-pink language token.
* Realtime push of graph changes; the overlay still pulls on open
  and on debounced refresh.

## Acceptance criteria

* `GET /api/graph?scope=drive` returns nodes of every kind
  declared above with non-zero counts on a fixture that has at
  least one rust file + a few markdown files.
* Chip counts in the overlay reflect the new producers (the
  `language 0` / `folder 0` / `media 0` chips show non-zero on
  the seeded test drive).
* Read-only directories appear as dead-ends.
* Latency under the phase-5 / phase-6 webtest probe budget
  (drive-scope response under ~20 ms warm on the 298-file
  fixture).
* Tests: focused unit + integration coverage for the merge.

## Tests

* `cargo test -p chan-server graph` covering the new shape on a
  fixture with mixed languages, directories, and markdown.
* `cargo test -p chan-server` clean.
* Pre-push gate green.

## Review and hardening

* @@Architect contract review before commit. PASS; see journal row.
* @@Backsystacean self-review for redundancy with fs-graph /
  language-graph (the merge should not duplicate computation).
* @@Webtest live verification on the seeded test drive: open the
  graph overlay, confirm chip counts and dead-end rendering.

## Dependencies

* [backsystacean-2](./backsystacean-2.md) PathClass payload.
* [backsystacean-3](./backsystacean-3.md) inspector kind taxonomy
  (the media / special split).
* [backsystacean-8](./backsystacean-8.md) fs-graph special-file
  `path_class` propagation.

## Progress notes

* `/api/graph` now accepts optional `scope=drive|directory|file`,
  `path`, and `depth` params. Existing callers still get the drive
  graph by default.
* The route keeps the existing semantic graph layer and merges in:
  filesystem directory/file/media nodes plus `contains` edges from
  the fs-graph builder, and language nodes plus language-to-directory
  edges from the language-graph builder.
* Directory node ids use the existing `directory:<path>` form, so
  language edges and filesystem directory nodes land on the same
  rendered node. File/media ids remain drive-relative paths.
* File and directory graph nodes carry `path_class` when available;
  media nodes are split by the same extension/classifier bucket the
  inspector uses.
* The graph overlay's combined drive/global load path now calls the
  merged `/api/graph` response instead of making separate fs-graph and
  language-graph requests. Standalone `/api/fs-graph` and
  `/api/graph/languages` remain available.
* Read-only directory dead-end behavior is inherited from the fs-graph
  builder and covered by a merged-graph test.

Self-review: the merge intentionally reuses the existing fs-graph and
language-graph builders, so the code does not fork traversal or report
aggregation logic. This still means drive-scope `/api/graph` computes
semantic + fs + language layers in one request; if Webtest finds the
298-file warm latency over budget, the next optimization should be a
shared precomputed graph response rather than another client-side
stitching path.

## Completion notes

Checks:

* `cargo test -p chan-server graph -- --test-threads=1`
* `cargo test -p chan-server -- --test-threads=1`
* `npm run check` in `web/`
* `cargo fmt --check`
* `scripts/pre-push`

Pre-push gate green.
