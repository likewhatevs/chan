# @@Backend task 4: Language graph endpoint

Owner: @@Backend
Status: Ready for frontend consumption

## Goal

Expose a backend graph view that elevates code languages as first-class graph
nodes connected only to folders. Folder rank per language drives graph depth.

## Relevant Links

- [[chan-pre-release-phase-2/journal.md]]
- [[chan-pre-release-phase-2/rustacean-3.md]]
- [[chan-pre-release-phase-2/frontend-8.md]]

## Frozen Wire Shape

Endpoint:

`GET /api/graph/languages?depth=<n>&language=<name>`

This is the final phase-2 endpoint. It supersedes the journal's earlier
working title, `GET /api/language-graph?path=<rel>&top=<n>`, because
@@Frontend has already started consuming `/api/graph/languages`.

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
- `language` filters language nodes case-insensitively.

## Completion Notes

Implementation landed in:

- `crates/chan-server/src/routes/graph.rs`
- `crates/chan-server/src/routes/mod.rs`
- `crates/chan-server/src/lib.rs`

Tests run:

- `cargo test -p chan-server routes::graph::tests`
- `cargo test -p chan-server`

Review expectations:

- @@Rustacean should review route and builder shape.
- @@Frontend may continue consuming this for
  [[chan-pre-release-phase-2/frontend-8.md]].
- @@Syseng should review report fan-out and rank-depth behavior in
  [[chan-pre-release-phase-2/syseng-2.md]].

Commit readiness:

- Ready after review and frontend/webtest smoke.

