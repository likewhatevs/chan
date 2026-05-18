# backsystacean-8: inspector / frontmatter follow-ups

Owner: @@Backsystacean
Status: REVIEW

## Goal

Close the three real defects surfaced by @@WebtestB's parallel
scenarios in [webtest-2](./webtest-2.md) round 2. Each item is a
small correctness fix layered on top of the REVIEW backend lanes.

## Items

### OBS-WT6-I: hardlink double-count in inspector aggregation

* Symptom: a hardlink pair (two paths sharing one inode) is counted
  twice in `report_summary` totals and `subtree.bytes` from
  `/api/inspector`.
* Expected: inode-deduped totals so a hardlinked file contributes
  once to bytes / file count.
* Likely fix site: `crates/chan-report/src/summary.rs` (rollup) or
  `crates/chan-server/src/routes/inspector.rs` (aggregation).
* Test: add a unit test that creates a hardlink pair in a fixture
  drive and asserts the rollup counts one file + one inode worth
  of bytes.
* Source: @@WebtestB round 2 observations in
  [webtest-2](./webtest-2.md).

### OBS-WT6-J: inspector payload missing `frontmatter_kind`

* Symptom: `/api/inspector?path=<contact.md>` returns markdown kind
  but does not surface the resolved `chan.kind` value, even though
  [backsystacean-4](./backsystacean-4.md) wired the registry.
* Expected: payload carries a `frontmatter_kind` (or equivalent)
  field for markdown entries, sourced via `chan_kind` lookup.
  `null` for non-frontmatter markdown.
* Likely fix site:
  `crates/chan-server/src/routes/inspector.rs` `build_inspector_payload`
  + the `InspectorPayload` struct.
* Test: extend `inspector_payload_covers_drive_directory_text_and_binary`
  with a contact markdown fixture; assert the field is set.
* Source: @@WebtestB round 2 observations in
  [webtest-2](./webtest-2.md).

### OBS-WT6-K: canonical frontmatter shape is nested

* Correction after @@WebtestA's read of
  `crates/chan-drive/src/markdown/frontmatter.rs`: the canonical
  shape is the **nested** `chan:` map with `kind:` inside, not the
  flat `kind: chan.contact` that the architect-2 memo and earlier
  fixtures implied. The flat form was never the registry contract;
  Webtest's first fixture (`alex.md` flat) had to be rewritten to
  nested for `/api/contacts` to resolve it.
* Fix: update `crates/chan-drive/design.md` paragraph from
  [backsystacean-4](./backsystacean-4.md) and the
  [architect-2](./architect-2.md) memo to state the nested shape
  is canonical. Convert any fixture / example that still uses the
  flat shape.
* Source: @@WebtestA observations in
  [webtest-1](./webtest-1.md) and @@WebtestB round 2 in
  [webtest-2](./webtest-2.md).

### OBS-WT6-WTA-1: `/api/files` listing omits symlinks

* Symptom: `Drive::list` (`crates/chan-drive/src/drive.rs`)
  filters symlinks out of the file-browser listing. `/api/fs-graph`
  surfaces them with `kind: "symlink"` and `/api/inspector`
  classifies them per path_class, but the file tree never shows
  them.
* Decision (@@Architect): include symlinks in `Drive::list`.
  The chan-drive write-side refusal of special files is a write
  guarantee; the read side should let users see the same symlinks
  that the inspector and graph already classify. The frontend
  renders a symlink badge via the classifier payload from
  [backsystacean-2](./backsystacean-2.md).
* Fix: relax the `Drive::list` symlink filter; symlink entries
  land in the listing with their `path_class` populated. The
  write-side gate stays as-is (special files refused on write).
* Test: list a drive containing a symlink; assert the entry is
  present with `path_class.kind == "symlink"`.

### OBS-WT6-WTA-5: fs-graph collapses FIFO and socket to ghost

* Symptom: `/api/fs-graph` returns `kind: "ghost"` for FIFO and
  socket files, the same bucket as off-drive symlink targets.
  The FIFO-vs-socket distinction that `path_class` surfaces in
  `/api/inspector` is lost at the graph layer.
* Decision (@@Architect): surface `path_class.kind` through
  fs-graph node payloads so the graph component can render a
  distinct dead-end badge per special kind. The traversal
  behavior stays the same (special files are dead-ends); only
  the rendering metadata is richer.
* Fix: include `path_class` (or just `path_class.kind`) in the
  fs-graph node payload for special-file nodes.
* Test: add a fixture with a FIFO + a socket + an off-drive
  symlink; assert each node carries the right `path_class.kind`.

### Block / character device coverage gap

* @@WebtestA notes block / character device classifier paths
  weren't exercised on the live fixture (needs sudo / `mknod`).
* Decision (@@Architect): accept the chan-drive unit-test
  coverage from [backsystacean-2](./backsystacean-2.md) as
  sufficient for this phase. Not a backsystacean-8 item; recorded
  here for traceability.

## Relevant links

* [webtest-2](./webtest-2.md) round 2 (the three OBS entries).
* [backsystacean-3](./backsystacean-3.md) inspector route.
* [backsystacean-4](./backsystacean-4.md) chan.kind registry.

## Acceptance criteria

* Hardlink dedupe shown by a focused test.
* `frontmatter_kind` (or `chan_kind` spec) field present in
  inspector payload for markdown entries, populated via
  `chan_kind` lookup.
* Canonical (nested) frontmatter shape documented; any flat
  example removed from docs / fixtures.
* `Drive::list` returns symlink entries with `path_class`
  populated; write-side refusal unchanged.
* `/api/fs-graph` node payload for special files carries
  `path_class` (or at minimum `path_class.kind`) so the inspector
  can render distinct badges per kind.

## Tests

* `cargo test -p chan-server inspector`.
* `cargo test -p chan-report` if the dedupe lands in the report
  crate.
* Pre-push gate green.

## Review and hardening

* @@Backsystacean self-review on the dedupe path (does the watcher
  / incremental indexer see the same dedupe?).
* @@Architect to confirm the canonical frontmatter shape decision
  matches [architect-2](./architect-2.md).

## Progress notes

* Inspector aggregation now scopes directory/root summaries through
  a deduped explicit file list. On Unix the dedupe key is `(dev,
  ino)` from `symlink_metadata`; non-Unix keeps the previous path
  behavior.
* Inspector payloads now include `frontmatter_kind` on every response:
  `"contact"` for registered nested `chan: { kind: contact }`
  markdown and `null` for ordinary / unknown-kind markdown.
* `Drive::list` keeps symlink entries visible; `/api/files?dir=...`
  returns their existing `path_class` metadata.
* `/api/fs-graph` nodes carry `path_class` alongside the existing
  kind, permission, link count, and symlink escape metadata. FIFO and
  socket nodes remain graph dead-ends with `kind: "ghost"`, but no
  longer lose the underlying filesystem kind.
* `crates/chan-drive/design.md` now states the canonical YAML shape
  explicitly as a nested `chan:` map, not flat `kind: chan.contact`.

Self-review on the dedupe path: this is intentionally inspector
presentation-layer dedupe only. The watcher, indexer, search, and graph
remain path-based so two hardlink names are still discoverable and can
still produce hardlink edges; the inspector rollup avoids double-counting
the same inode for totals.

## Completion notes

Focused checks:

* `cargo test -p chan-server inspector -- --test-threads=1`
* `cargo test -p chan-server fs_graph -- --test-threads=1`
* `cargo test -p chan-server directory_listing_keeps_symlink_with_path_class -- --test-threads=1`
* `cargo test -p chan-drive list_keeps_symlink_entries_visible -- --test-threads=1`
* `cargo test -p chan-server -- --test-threads=1`
* `cargo test -p chan-drive list -- --test-threads=1`
* `cargo fmt --check`
* `scripts/pre-push`

Pre-push gate green.
