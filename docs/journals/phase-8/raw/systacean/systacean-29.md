# systacean-29 — chan-drive Drive::list unified-path for Drafts (unblock fullstack-a-66b)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Extend `Drive::list(rel)` (consumed by chan-server's
`/api/files?dir=<path>` listing) to route
`Drafts/<name>` prefixes to the drafts metadata
dir, matching the `-26` read/write unification
pattern.

## Reference

@@FullStackA's scope poke at the tail of
[`../alex/event-fullstack-a-architect.md`](../alex/event-fullstack-a-architect.md):
"## 2026-05-22 — scope poke (fullstack-a-66b needs
chan-drive Drive::list unified-path)".

Routing: **Option A** (chan-drive extension) —
matches `-26` precedent + smallest blast radius.

## Scope

Apply the same `resolve_io`-style routing from `-26`
to `Drive::list`:

* If `rel.starts_with("Drafts/")` → list against the
  drafts cap-std dir handle (stripping the `Drafts/`
  prefix to get the sub-path).
* Else → list against the drive-root dir handle (as
  today).
* Empty `Drafts/` (list of drafts root itself) →
  return the list of draft directory entries (each
  `untitled-N` etc.).

## Acceptance

1. `Drive::list("Drafts/")` returns the list of
   `untitled-N` draft directories.
2. `Drive::list("Drafts/untitled-1")` returns
   contents of that draft (`draft.md` + any user-
   pasted images/files).
3. `Drive::list("notes/")` unchanged (drive-root
   path).
4. Existing drive-root list callers unchanged
   (backward-compat regression).

### Tests

* Round-trip: `create_draft_dir("foo") +
  write_text("Drafts/foo/draft.md", "...")` →
  `list("Drafts/foo")` returns `draft.md`.
* `list("Drafts/")` returns all draft dirs.
* Drive-root list unchanged.

### Gate

`cargo fmt / clippy / test`; `RUSTFLAGS="-D warnings"
cargo build --no-default-features` green.

## Coordination

* @@Systacean lane.
* `fullstack-a-66b` (FB Drafts row + expansion)
  resumes once this lands.

## Authorization

Yes for `crates/chan-drive/src/drive.rs` + tests +
task tail + outbound.

## Numbering

This is `-29`.

## 2026-05-22 — implementation complete; ready for smoke

Picked up `-29` per the architect's dispatch. Applied the `resolve_io`-style routing pattern from `-26` to `Drive::list`.

### What landed

`crates/chan-drive/src/drive.rs::Drive::list`:

* New branch routes `Drafts/`-prefixed rels through the drafts cap-std handle (`drafts_dir_handle` from `-26`). Three shapes:
  * `"Drafts"` / `"Drafts/"` → list drafts root (returns one entry per `DraftRef::name`).
  * `"Drafts/<name>"` / `"Drafts/<name>/<sub>"` → list inside the drafts subtree (strips `Drafts/` prefix; passes the sub-path through `validate_rel`).
  * Anything else → drive-root path unchanged.
* No new fields / structs needed (the `drafts_dir_handle` from `-26` is reused).

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `Drive::list("Drafts/")` returns the list of `untitled-N` draft directories | ✓ |
| 2 | `Drive::list("Drafts/untitled-1")` returns contents of that draft | ✓ |
| 3 | `Drive::list("notes/")` unchanged (drive-root path) | ✓ (regression test pin) |
| 4 | Existing drive-root list callers unchanged | ✓ (regression test pin) |

### Tests (+2)

1. `list_unified_routes_drafts_paths_to_drafts_dir` — full round-trip: create 2 drafts, write inside one + a pasted PNG, assert `list("Drafts/")` returns both drafts, `list("Drafts")` (bare) works the same, `list("Drafts/untitled-1")` returns the file + image, `list("notes")` continues to hit the drive root.
2. `list_drafts_root_empty_when_no_drafts` — `Drive::list("Drafts/")` on a fresh drive returns an empty Vec (the drafts root exists but contains nothing). Pins the FB-renders-empty case.

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-drive --lib`: **451 passed; 0 failed; 2 ignored** (was 449; +2 new).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Diff

`crates/chan-drive/src/drive.rs`: +81 / -1. Plus task tail + outbound poke. 3 paths.

### Suggested commit subject

```
chan-drive: prefix-aware Drive::list for Drafts (systacean-29)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-29-smoke`. Expected ALL GREEN. Bounded additive change; backward-compat preserves drive-root list semantics. PTY-test flakiness from `-27` smokes may still appear but isn't `-29`-related.

Per architect's pre-authorization in the dispatch poke, proceeding to commit + push + smoke.
