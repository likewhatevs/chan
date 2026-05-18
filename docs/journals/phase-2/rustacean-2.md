# @@Backend task rustacean-2: Filesystem-as-truth graph load

Owner: @@Backend
Status: Ready for specialist review

## Goal

Freeze and implement the `/api/graph` behavior for indexed files that no longer
exist on disk.

## Relevant Links

- [[phase-2/journal.md]]
- [[phase-2/backend-3.md]]
- [[phase-2/syseng-1.md]]

## Frozen Wire Shape

`/api/graph` keeps the existing graph response shape. Indexed file rows that
are missing or no longer regular files on disk are emitted as file nodes with
`missing: true`.

```json
{
  "nodes": [
    {
      "kind": "file",
      "id": "notes/deleted.md",
      "label": "deleted",
      "path": "notes/deleted.md",
      "missing": true
    }
  ],
  "edges": [
    {
      "source": "notes/live.md",
      "target": "notes/deleted.md",
      "kind": "link",
      "broken": true
    }
  ]
}
```

Present files omit `missing` as before. Non-link edges still omit `broken`.

## Implementation Notes

- Implemented in [[phase-2/backend-3.md]].
- `/api/graph` uses `symlink_metadata` from the drive root and treats only
  regular files as present.
- Link resolution still uses all indexed file paths so extensionless links can
  resolve to the missing indexed file's ghost node.

## Tests

- `cargo test -p chan-server routes::graph::tests`
- `cargo test -p chan-server`

## Review

- Ready for @@Syseng review.
- Ready for @@Frontend consumption by [[phase-2/webdev-3.md]].

