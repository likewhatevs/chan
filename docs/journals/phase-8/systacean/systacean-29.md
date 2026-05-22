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
