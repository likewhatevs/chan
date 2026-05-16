# @@Backend task rustacean-3: Language graph wire shape

Owner: @@Backend
Status: Ready for frontend consumption

## Goal

Expose a backend graph view that elevates code languages as first-class graph
nodes connected only to folders, with folder rank per language defining graph
depth.

## Relevant Links

- [[chan-pre-release-phase-2/journal.md]]
- [[chan-pre-release-phase-2/request.md]]
- [[chan-pre-release-phase-2/backend-4.md]]

## Frozen Wire Shape

Canonical task file: [[chan-pre-release-phase-2/backend-4.md]].

Endpoint:

`GET /api/graph/languages?depth=<n>&language=<name>`

Query:

- `depth`: optional 1-based max folder rank per language. Omitted or `0`
  returns max depth.
- `language`: optional exact case-insensitive language filter.

Response:

```json
{
  "max_depth": 3,
  "nodes": [
    {
      "kind": "language",
      "id": "language:Rust",
      "label": "Rust",
      "language": "Rust",
      "files": 12,
      "code": 2400
    },
    {
      "kind": "folder",
      "id": "folder:crates/chan-server/src",
      "label": "src",
      "path": "crates/chan-server/src",
      "files": 8,
      "code": 1800
    }
  ],
  "edges": [
    {
      "source": "language:Rust",
      "target": "folder:crates/chan-server/src",
      "kind": "language",
      "rank": 1,
      "files": 8,
      "code": 1800
    }
  ]
}
```

Folder `path` is `""` for files at the drive root; the root folder node id is
`folder:` and label is `/`.

## Acceptance Criteria

- Language nodes are sourced from the whole-drive report.
- Language nodes connect only to folder nodes.
- Folder ranking is per language, descending by file count, then code lines,
  then folder path.
- `depth` limits by that per-language rank.

## Test Expectations

- Add focused Rust tests for the language graph builder.
- Run `cargo test -p chan-server`.

## Progress Notes

- Started after [[chan-pre-release-phase-2/journal.md]] listed L1 as the next
  @@Backend-owned task.
- Added `GET /api/graph/languages`.
- Implemented the language graph builder from whole-drive report file rows.
- Wired route export and router entry.

## Completion Notes

Changed files:

- `crates/chan-server/src/routes/graph.rs`
- `crates/chan-server/src/routes/mod.rs`
- `crates/chan-server/src/lib.rs`
- `chan-pre-release-phase-2/rustacean-3.md`

Tests run:

- `cargo test -p chan-server routes::graph::tests`
- `cargo test -p chan-server`

Review expectations:

- @@Rustacean should review route and builder shape.
- @@Frontend can consume the frozen endpoint for
  [[chan-pre-release-phase-2/frontend-8.md]].
- @@Syseng should review report fan-out / rank-depth behavior if graph render
  cost becomes a concern on large drives.

Commit readiness:

- Ready after review and frontend smoke.
