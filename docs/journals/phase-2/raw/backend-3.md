# @@Backend task 3: Ghost missing indexed files in content graph

Owner: @@Backend
Status: Ready for specialist review

## Goal

Make `/api/graph` use the filesystem as display truth for indexed file nodes.
If a file still exists in the graph DB but is missing or no longer a regular
file on disk, emit it as a ghost file node instead of a normal file.

## Relevant Links

- [[phase-2/request.md]]
- [[phase-2/syseng-1.md]]

## Acceptance Criteria

- Indexed files present on disk continue to emit normal file nodes.
- Indexed files missing on disk emit `missing: true` file nodes.
- Link edges targeting missing indexed files are marked broken.
- The graph remains stable while the watcher/indexer catches up.

## Test Expectations

- Add focused Rust coverage for the filesystem-presence helper.
- Run `cargo test -p chan-server`.

## Progress Notes

- Picked up after [[phase-2/syseng-1.md]] identified
  content graph filesystem truth as a backend/syseng-sensitive surface.
- Updated `/api/graph` to lstat indexed file paths from the drive root and emit
  stale graph rows as `missing: true` file nodes.
- Link-edge `broken` now uses the present-on-disk file set, while link
  resolution still uses all indexed file paths so extensionless links can land
  on a missing indexed file's ghost node.

## Completion Notes

Changed files:

- `crates/chan-server/src/routes/graph.rs`
- `phase-2/backend-3.md`

Tests run:

- `cargo test -p chan-server routes::graph::tests`
- `cargo test -p chan-server`

Review expectations:

- @@Syseng should review the lstat semantics and stale-row behavior.
- @@Webtest should smoke deleting a graphed file while the graph overlay is open
  or before reload, then confirm it renders as a ghost/missing node.

Commit readiness:

- Ready after @@Syseng review and web smoke.
- Known risk: `/api/graph` now performs one `symlink_metadata` call per indexed
  file row. This is intentional to make the filesystem the display truth.
