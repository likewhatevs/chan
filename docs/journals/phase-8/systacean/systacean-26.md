# systacean-26 — chan-drive unified-path read/write API for Drafts (unblock fullstack-a-66)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Extend chan-drive with unified-path read/write/util
ops that accept `Drafts/`-prefixed paths and route
internally, so chan-server's `/api/files/*path` route
+ the editor's existing autosave path can target draft
files without API branching.

## Reference

@@FullStackA's scope poke at the tail of
[`../alex/event-fullstack-a-architect.md`](../alex/event-fullstack-a-architect.md):
"## 2026-05-22 — scope poke (fullstack-a-66 needs
chan-drive unified-path API extension)".

Routing: Option (A) — chan-drive extends with
unified-path ops. Cleanest contract; smallest blast
radius.

## Audit context (from @@FullStackA's scope poke)

The current state after `systacean-24` + `-25`:

* Drive-root files: `Drive::read_text(rel)` /
  `Drive::write_text(rel, content)` enforce the
  editable-text gate, atomic write, watcher self-
  write annotation.
* Drafts: `chan_drive::drafts::create_dir(...)`
  returns a `DraftRef`. Reads / writes happen via
  RAW `std::fs` against `DraftRef.abs` — no
  editable-text gate, no atomic write, no watcher
  self-write annotation.

`-25` shipped the indexer integration so graph emit
treats `Drafts/<name>/<file>` paths in a unified
keyspace — but the read/write surface that the SPA
editor speaks (`Drive::read_text(rel)`) doesn't see
Drafts.

## Scope

New chan-drive API surface that accepts
`Drafts/`-prefixed paths:

### Unified read/write

* `Drive::read_text_unified(rel) -> Result<String>`:
  if `rel.starts_with("Drafts/")`, route to the
  drafts subtree; else current drive-root path.
* `Drive::write_text_unified(rel, content) ->
  Result<()>`: same routing; same gates as
  `write_text` (editable-text gate, atomic write,
  watcher self-write annotation).

OR (implementer's call on shape):

* Make `Drive::read_text` / `write_text` themselves
  prefix-aware. Backward-compat: paths without the
  `Drafts/` prefix behave exactly as today.

Recommended: **shape it so that `Drive::read_text`
+ `Drive::write_text` accept the unified path** —
fewer API entry points + the unified scheme already
shipped on the indexer / graph side.

### Helpers

* `Drive::next_untitled_draft_name() -> Result<String>`:
  picks the smallest unused `untitled-N` name under
  Drafts; returns the bare name (caller composes
  `Drafts/<name>/draft.md`).
* `Drive::draft_dir_exists(name) -> bool` if needed
  (or surface via `list_drafts`).

### Atomic-write parity

Atomic-write semantics inside drafts should mirror
drive-root: write to tmp + fsync + rename, parent-dir
fsync. The editable-text gate + watcher self-write
annotation also apply (so the editor's autosave loop
+ on-disk-change reconciliation work the same).

## Acceptance

1. `Drive::read_text("Drafts/some-draft/draft.md")`
   returns the file content.
2. `Drive::write_text("Drafts/some-draft/draft.md",
   content)` atomically writes; watcher self-write
   annotation suppresses the indexer's own re-index
   trigger.
3. `Drive::next_untitled_draft_name()` returns
   `untitled` first call; `untitled-1` if `untitled`
   exists; etc.
4. Existing drive-root callers unchanged
   (backward-compat regression check).

### Tests

* Round-trip read → write → read against a draft
  file.
* Atomic-write semantics test (parent fsync; tmp +
  rename).
* `next_untitled_draft_name` count-up test.
* Existing drive-root tests still pass.

### Gate

* `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test -p chan-drive` green.
* `RUSTFLAGS="-D warnings" cargo build
  --no-default-features` green.

## Coordination

* @@Systacean lane (chan-drive).
* Atomic-audit-commit.
* `fullstack-a-66` resumes once this lands.

## Authorization

**Yes** for `crates/chan-drive/src/{drive,drafts,...}.rs`
+ tests + task tail + outbound.

## Numbering

This is `-26`.

## Out of scope

* SPA Cmd+N handler (lives in `fullstack-a-66`).
* chan-server route changes (the unified-path
  Drive API means chan-server's existing
  `/api/files/*path` route works as-is; if any
  small chan-server adjustment is needed, scope-poke).
