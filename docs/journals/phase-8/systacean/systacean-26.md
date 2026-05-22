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

## 2026-05-22 — implementation complete; ready for smoke

Picked up `-26` per the architect's dispatch poke. Routed (A) — extend chan-drive with prefix-aware unified-path ops. Single API entry per the recommended shape ("make `Drive::read_text` + `Drive::write_text` themselves prefix-aware").

### What landed

* **`crates/chan-drive/src/drive.rs`**:
  * `Drive.drafts_dir_handle: cap_std::fs::Dir` — new field, opened in `Drive::open` against `paths.drafts` (just after `drafts::ensure_root`). Sandboxed cap-std handle parallel to the existing `dir` handle for drive root.
  * `Drive::resolve_io(rel) -> Result<(&Dir, PathBuf)>` — new helper. Strips `Drafts/` prefix when present + returns the drafts handle + validated sub-path; otherwise returns the drive handle + validated path unchanged. Empty `Drafts/` (no sub-path) errors as Io.
  * `read_text` + `read_text_with_stat` + `write_text` + `write_text_if_unchanged` — refactored to use `resolve_io`. The editable-text gate still runs against the FULL unified rel (so `.md` etc. matches uniformly).
  * `Drive::next_untitled_draft_name() -> Result<String>` — smallest-unused-N picker. Returns bare `untitled` if free; else `untitled-1`; else `untitled-2`; etc. Fills gaps (smallest unused, not last+1).

### Atomic-write + watcher self-write annotation parity

* **Atomic write**: drafts writes go through `fs_ops::atomic_write_in` on the drafts cap-std handle — same tmp + fsync + rename + parent-dir fsync semantics as drive-root writes. No behavioral divergence.
* **Watcher self-write annotation**: this is chan-server's responsibility via the `SelfWrites` tracker (chan-drive doesn't own that mechanism). Since chan-server keys `SelfWrites` on the rel string passed to `Drive::write_text`, and chan-server now calls `Drive::write_text("Drafts/...", content)` for drafts writes, the same suppression flows through. `-25`'s watcher prefix means the inbound watcher event arrives with the matching `Drafts/<...>` key. Parity achieved without chan-server changes.

### Sandbox + traversal safety

The cap-std Dir handle prevents traversal-escape on either route — drafts writes can't read or write outside the drafts dir even if a `..` slipped past `fs_ops::validate_rel` (which it can't, but defense in depth). Same invariant as drive-root.

### Tests (+6)

All in `crates/chan-drive/src/drive.rs::tests`:

1. `unified_path_read_write_roundtrip_for_drafts` — write/read against `Drafts/untitled-1/draft.md` succeeds; content round-trips.
2. `unified_path_write_text_atomic_for_drafts` — overwrite atomically replaces; final content matches the second write.
3. `unified_path_rejects_drafts_root_as_target` — `read_text("Drafts/")` + `write_text("Drafts/", ...)` both error (no file at the drafts root itself).
4. `unified_path_drive_root_paths_unchanged` — drive-root paths (e.g. `notes/intro.md`) route to the drive dir, land on disk under the drive root, NOT the drafts root. Backward-compat regression check.
5. `next_untitled_draft_name_counts_up_through_gaps` — first → `untitled`; with `untitled` present → `untitled-1`; with `untitled` + `untitled-1` + `untitled-3` present → `untitled-2` (smallest gap-fill).
6. `unified_path_write_text_if_unchanged_for_drafts` — optimistic-concurrency parity. None-mtime + missing file = create; stale mtime = `WriteConflict`; current mtime = succeed.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `read_text("Drafts/some-draft/draft.md")` returns content | ✓ |
| 2 | `write_text("Drafts/some-draft/draft.md", content)` atomic; self-write annotation flows through chan-server's SelfWrites | ✓ |
| 3 | `next_untitled_draft_name()` returns `untitled` first; `untitled-1` if `untitled` exists; etc. | ✓ |
| 4 | Existing drive-root callers unchanged (backward-compat regression) | ✓ |

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-drive --lib`: **446 passed; 0 failed; 2 ignored** (+6 vs `-25` baseline of 440).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                       | +    | -   |
|--------------------------------------------|------|-----|
| `crates/chan-drive/src/drive.rs`           | +179 | -8  |

Plus task tail + outbound poke. 3 paths total.

### Suggested commit subject

```
chan-drive: prefix-aware unified-path read_text + write_text + next_untitled_draft_name for Drafts (systacean-26)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-26-smoke` on a fresh smoke branch. Expected: all 5 jobs green. Pure additive chan-drive; backward-compat preserves the existing API contract. No chan-server changes; no SPA changes.

Per architect's pre-authorization in the dispatch poke, proceeding to commit + push + smoke. Will surface verdict.
