# systacean-32 — chan-drive Drive::stat unified-path for Drafts (closes -a-66 slice b/c/d data-flow gap)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: HIGH (closes recurring 3-slice data-flow gap)

## Goal

Extend `Drive::stat()` to route `Drafts/`-prefixed
paths through the drafts cap-std handle (same pattern
as `-26` read/write + `-29` list). Closes the
recurring data-flow PARTIAL @@WebtestA flagged
across `-a-66` slices b/c/d.

## Reference

@@WebtestA's triple walk (b2dfead + 9ad002e +
c69756a) — same PARTIAL pattern across slices b/c/d:

* Disk persistence WORKS (Drive::write_text writes
  to Drafts/ correctly via `-26`).
* API listing INCOMPLETE — `/api/files?dir=Drafts/...`
  returns empty.

Architect-side audit (chan-server `routes/files.rs`):

* `api_list_files` calls `list_dir_entries(&drive, dir)`.
* `list_dir_entries` calls `drive.list(&rel)` (✓
  unified post-`-29`).
* For each child: calls `drive.stat(&path)` where
  `path = "Drafts/<sub>/<name>"`.
* `Drive::stat` is NOT unified — routes through the
  drive-root capfs, gets "not found" for Drafts
  paths, emits a `tracing::warn!`, skips the entry.

Net: `list()` returns the children correctly but
`stat()` filters them all out. Empty wire response.

## Fix shape

Apply the same `resolve_io`-style routing from
`-26`/`-29` to `Drive::stat`:

```rust
pub fn stat(&self, rel: &str) -> Result<FileStat> {
    let (dir, sub_path) = self.resolve_io(rel)?;
    let meta = dir.metadata(&sub_path)?;
    // ... existing stat logic ...
}
```

Strips `Drafts/` prefix when present + routes
through the drafts dir handle; else uses the drive
dir handle as today.

## Broader audit (flag at task tail)

While touching this surface, audit OTHER Drive
methods that take a `rel: &str` and may not be
unified. Likely candidates:

* `Drive::delete` (rm)
* `Drive::rename` / `move`
* Anything else that calls `self.dir.<op>(rel)`

If found, EITHER bundle the fixes (if all small +
same pattern) OR flag for a follow-up
`systacean-N`. Implementer's call based on scope.

## Acceptance

1. **`Drive::stat("Drafts/rich-prompt/prompt.md")`
   returns the file metadata** (not "not found").
2. **`/api/files?dir=Drafts/rich-prompt` returns
   `prompt.md`** in the listing.
3. **The SPA's FB tree expands Drafts/** with all
   real children once the user clicks the
   synthetic Drafts row.
4. **Backward-compat regression**: drive-root paths
   unchanged (`Drive::stat("notes/intro.md")`
   works as today).

### Tests

* Round-trip: create draft + write file inside →
  `Drive::stat("Drafts/<name>/<file>")` returns
  correct metadata.
* Drive-root stat unchanged.
* If bundling other methods, similar pins per
  method.

### Gate

`cargo fmt / clippy / test`; `RUSTFLAGS="-D warnings"
cargo build --no-default-features` green.

## Coordination

* @@Systacean lane (chan-drive).
* Closes a recurring 3-walk PARTIAL — HIGH priority
  for v0.12.0 option-C cut.
* Atomic-audit-commit.

## Authorization

Yes for `crates/chan-drive/src/drive.rs` (+ other
unified candidates if bundled) + tests + task tail
+ outbound.

## Numbering

This is `-32`.

## Out of scope

* SPA-side Drafts FB tree polish beyond what the
  unified `stat` unblocks.
* Inspector / chip rendering bugs from slice c
  (separate; `-a-66c` audit needed if API listing
  alone doesn't surface them).
