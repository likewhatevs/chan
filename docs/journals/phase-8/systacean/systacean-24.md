# systacean-24 — chan-drive Drafts metadata folder (backend: filesystem + indexer + graph emit)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Add Drafts metadata folder to chan-drive alongside the
existing Trash folder. Drafts hold draft directories
(each `Drafts/untitled-N/` carrying `draft.md` + any
associated files), AND Rich Prompt history
(`Drafts/rich-prompt-N/`). Drafts always indexed.
Graph emits a special Drafts root node with a distinct
edge to drive.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md)
"## Flow for the 'New Draft' action" + "### Extra"
sections.

Key spec points:

* Drafts folder lives in chan-drive METADATA (outside
  drive root), alongside Trash.
* Each draft is a DIRECTORY (`Drafts/untitled-N/`),
  not a single file. The dir lets users paste images
  + drop config files alongside `draft.md`.
* Rename/Move action moves the draft directory into
  the drive root (promotion). Links inside should
  already be relative.
* Drafts also stores Rich Prompt history
  (`Drafts/rich-prompt-N/`).
* Drafts always indexed.
* Graph: Drafts root emitted as a special folder
  node; edge to drive root is distinct (different
  styling / color than regular `contains`).
* Click Drafts node in graph → opens same inspector
  as the FB Drafts folder.
* Click sub-files/dirs/media inside the graph →
  same behavior as drive files.

## Scope (this task — chan-drive backend)

### 1. Filesystem primitive

* `chan_drive::Drive::drafts_dir() -> &Path` (parallels
  the existing `trash_dir`).
* `chan_drive::Drive::create_draft_dir(name) ->
  Result<DraftRef>`: creates `Drafts/{name}/` in
  metadata; returns a handle for subsequent ops.
* `chan_drive::Drive::list_drafts() ->
  Result<Vec<DraftRef>>`: enumerates existing draft
  dirs.
* Atomic-write surfaces inside drafts mirror the
  drive-root surfaces (same `Drive::write_text` etc.
  but targeting the drafts subtree).

### 2. Indexer integration

* The watcher / indexer ALWAYS walks the Drafts
  subtree (no opt-out). Drafts contribute to search
  + graph just like drive content.
* Per-draft directories appear in the graph emit
  alongside drive directories.

### 3. Graph emit

* Drafts root emitted as a graph node with a
  distinguishing kind (e.g. `kind: "drafts"` or
  `kind: "directory"` + `meta: {drafts_root: true}`
  — your call which shape reads cleanest).
* The contains-edge from drive-root → Drafts-root
  carries a distinct attribute (e.g.
  `kind: "drafts_link"` or `meta: {drafts: true}`)
  so the SPA can style it differently.
* Sub-directories + files within Drafts use the
  normal contains-edge shape (they ARE regular
  directories/files from the graph's perspective;
  only the root is special).

### 4. Promotion (move from drafts to drive)

* `chan_drive::Drive::promote_draft(draft_name,
  target_drive_path) -> Result<()>`: moves the
  draft directory into the drive root at the target
  path. Atomic via filesystem rename when possible.

### 5. Schema decisions

* Use the existing `chan_drive::Drive` API surface;
  the Drafts subtree is just a structured subdirectory
  of the metadata dir.
* No new SerDe shapes needed in `chan-drive` proper;
  paths are strings under the metadata root.

## Out of scope (this task)

* SPA new-draft flow (Cmd+N → spawn draft dir) —
  separate task `fullstack-a-66`.
* FB rendering of Drafts folder with distinct color —
  `fullstack-a-66`.
* Rich Prompt history reuse — `fullstack-a-66`.
* chan-server route for the draft promotion — fire
  scope poke if chan-server needs new IPC; otherwise
  the SPA can use existing fs routes.

## Acceptance

1. `chan_drive::Drive::drafts_dir()` returns a path
   alongside `trash_dir`.
2. `create_draft_dir(name)` creates `Drafts/{name}/`
   atomically; returns handle.
3. `list_drafts()` enumerates current drafts.
4. Watcher emits events for Drafts subtree changes.
5. Indexer includes Drafts content in search results.
6. Graph emit carries a Drafts root node + distinct
   drive→Drafts edge attribute.
7. `promote_draft(name, target)` atomically moves
   draft into drive.

### Tests

* `Drive::drafts_dir` returns the expected path.
* Round-trip: create_draft_dir → list_drafts → write
  inside → read back.
* Graph emit includes Drafts root + distinct edge.
* Promote draft moves the directory.

### Gate

* `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test -p chan-drive` green.
* `RUSTFLAGS="-D warnings" cargo build
  --no-default-features` green.

## Coordination

* @@Systacean lane (chan-drive primary). Some chan-
  server graph route may need a small mirror; if so
  fire scope poke + I'll route the chan-server piece
  separately OR you bundle if minimal.
* `fullstack-a-66` consumes the API surface; their
  pickup waits on yours.
* Atomic-audit-commit discipline.

## Authorization

**Yes** for `crates/chan-drive/src/*.rs` + Drafts
primitive + indexer integration + graph emit hooks
in chan-drive's index emission + tests + task tail
+ outbound. If chan-server graph route needs
companion changes, scope-poke first.

## Numbering

Highest committed `systacean-N` is `-23`. This is
`-24`.
