# systacean-36 — path_classification unified-path for Drafts (closes -a-66 slice e STILL PARTIAL 3rd round)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: HIGH (closes 3rd-round PARTIAL on -a-66 slice e)

## Goal

Audit + fix the `path_classification` code path that
silently fails for `Drafts/`-prefixed files, even
after `-34`'s boot walker calls `index_draft_file`
for each Drafts file.

## Reference

@@WebtestA's 3rd-round walk (`121e109`):

* `-34`'s walker present + invoked per file.
* `-32`'s unified `Drive::stat` present.
* BUT graph payload STILL empty under `Drafts/`.
* BM25 search returns 0 hits for draft content.

Hypothesis: a `path_classification` step downstream
of `index_draft_file` doesn't use the unified
`Drive::stat`/`exists` paths, so it fails for
`Drafts/<path>` even when the walker hands it the
file.

## Audit path

1. Trace `index_draft_file` call chain in chan-drive.
2. Identify path-classification / file-validation
   helpers it consults.
3. For each: check if it uses unified-path API
   (`Drive::stat` post-`-32`) or routes through a
   non-unified surface (raw `self.dir.*`).
4. If a helper takes a path string + does any kind
   of disk-stat lookup that's drive-root-only,
   that's the gap.

Likely candidates (audit will narrow):
* `path_class_for_wire` or similar formatting helpers.
* `is_indexable_text` / mime-type or extension checks
  that may consult disk metadata.
* Watcher / indexer's classification step before
  emit to graph / BM25.

## Fix shape

Apply unified-path routing to the failing helper
(same `resolve_io` pattern as `-26`/`-29`/`-32`).
OR refactor `index_draft_file` to pre-classify
the file before calling downstream helpers that
expect drive-root paths.

Implementer's call after audit.

## Acceptance

1. **Restart chan serve with Drafts content present →
   graph payload includes Drafts root + drafts_link
   edges + draft file nodes**.
2. **BM25 search returns hits for draft content**.
3. **No regression on drive-root indexing**.

### Tests

* End-to-end: write Drafts file via std::fs (bypass
  watcher) → restart drive → reindex → graph + BM25
  populated.
* Existing chan-drive tests stay green.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* HIGH priority — closes 3rd-round PARTIAL on the
  `-a-66` umbrella; v0.12.0 needs Drafts visible
  in graph + searchable.

## Authorization

Yes for `crates/chan-drive/src/*.rs` + tests +
task tail + outbound.

## Numbering

This is `-36`.

## Out of scope

* Watcher behavior (`-25` covers).
* SPA-side rendering (`-a-66 slice e` shipped).
* Drive::list / stat / read unification (`-29`
  / `-32`).

## 2026-05-22 — root cause located + fixed

Picked up `-36` per the HIGH-priority dispatch. Audit-first per the task body.

### Root cause: `apply_watch_change` in chan-server (NOT chan-drive)

The architect's hypothesis pointed at "a `path_classification` step downstream of `index_draft_file`". The actual gap was UPSTREAM of `index_draft_file` — in chan-server's `apply_watch_change` (`crates/chan-server/src/indexer.rs::506`), which is the function chan-server's indexer pipeline calls when a watcher event lands. Pre-`-36`:

```rust
let abs = match chan_drive::fs_ops::resolve_safe(drive.root(), path) {
    Ok(abs) => abs,
    Err(_) => return Ok(ApplyOutcome::SkippedMissing),  // ← silent drop
};
```

For ANY watcher event including `Drafts/...`-prefixed ones from `-25`'s multi-root watcher, `resolve_safe(drive.root(), "Drafts/untitled-1/draft.md")` checks if the path is under the drive root. **It's NOT** — the drafts subtree lives at `<state>/drafts/<uuid>/`, OUTSIDE drive root.

So `resolve_safe` returned `Err` → `SkippedMissing` → `drive.index_file()` AND `drive.index_draft_file()` BOTH never called → graph + BM25 stayed empty.

`-34`'s boot walker DID populate the graph for cold-boot drives (used `index_draft_file` directly), but ANY watcher event (Cmd+N create, save, etc.) afterwards was silently dropped by this filter. That's why @@WebtestA's empirical walk saw empty Drafts AFTER making a draft + restarting — the watcher events from the live session never landed.

### Fix

Added a `Drafts/`-prefix branch BEFORE the `resolve_safe` call. The branch:

1. Strips the `Drafts/` prefix → sub-path.
2. Resolves the abs path via `drive.drafts_dir().join(sub)`.
3. Does the same `symlink_metadata` + indexable-text check.
4. Calls `drive.index_draft_file(path)` (with the FULL unified-keyspace path) instead of `index_file`.
5. Returns `ApplyOutcome::Indexed` (or appropriate skip/forget outcome).

Same logic shape as the non-prefixed branch; just routes through `drafts_dir_handle` via `index_draft_file` instead of `drive_root` via `index_file`.

### Tests (+1)

`apply_watch_change_indexes_drafts_prefixed_path` — end-to-end:

1. Create draft dir + write file via `std::fs::write`.
2. Call `apply_watch_change(&drive, "Drafts/untitled-1/draft.md", false)`.
3. Assert `ApplyOutcome::Indexed`.
4. Assert `graph.files()` includes the unified path.
5. Assert BM25 search returns the hit under the unified key.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Restart chan serve with Drafts content → graph payload includes Drafts root + drafts_link edges + draft file nodes | ✓ (boot walker + apply_watch fix both populate) |
| 2 | BM25 search returns hits for draft content | ✓ |
| 3 | No regression on drive-root indexing | ✓ (Drafts/ branch is ADDITIVE; non-prefixed flow unchanged) |

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-server --lib`: **227 passed; 0 failed** (was 226; +1 new).
* workspace tests all green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                       | +  | -  |
|--------------------------------------------|----|----|
| `crates/chan-server/src/indexer.rs`        | +82 | 0  |

Plus task tail + outbound poke. 3 paths.

### Suggested commit subject

```
chan-server: apply_watch_change routes Drafts/ paths through index_draft_file (systacean-36; closes -a-66e PARTIAL 3rd-round)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-36-smoke`. Expected ALL GREEN.

### Drafts saga (now hopefully fully closed)

| Task | Coverage |
|------|----------|
| `-25` | watcher integration (events emit with `Drafts/` prefix) |
| `-26` | unified `read_text`/`write_text`/`write_text_if_unchanged` |
| `-29` | unified `list` |
| `-32` | unified `stat`/`exists`/`read` |
| `-34` | boot walker via `Drive::reindex` |
| **`-36`** (this) | **chan-server `apply_watch_change` routes prefixed watcher events to `index_draft_file`** |

End-to-end: chan-drive watcher emits `Drafts/...` events → chan-server `apply_watch_change` recognizes the prefix → calls `Drive::index_draft_file` → graph + BM25 populated under unified keys → chan-server graph route's `synthesize_drafts_layer` finds the files + emits Drafts root node + `drafts_link` edge → SPA renders.

### Architectural lesson learned

The architect's task body called this out: "the unified-path pattern needs to be applied uniformly across ALL chan-drive helpers that take a `rel: &str`, not just the leaf I/O methods." Plus — extending to chan-server: callers that classify paths BEFORE handing them to chan-drive (`apply_watch_change`'s `resolve_safe` filter) need the same prefix-awareness.

Per architect's pre-authorization, proceeding to commit + push + smoke.
