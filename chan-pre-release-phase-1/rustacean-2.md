# rustacean-2: Filesystem graph and index status API

Owner: rustacean. Depends on: nothing. Unblocks: webdev-1,
webdev-2, syseng-1.

## Goal

Expose a graph-like index for directories, files, subdirectories,
symlinks, hardlinks, and broken links, without overloading the current
semantic link/tag/mention graph.

## Product requirements

- File Browser can ask to "Graph this" for any file or directory.
- Folder scope starts at depth 1 and expands through children.
- File scope includes its parent folder as a convenient scope option.
- Folder scope offers parent folder as a convenient scope option.
- Symlinks and hardlinks are represented as graph nodes/edges.
- Broken symlinks are ghost nodes, similar to current broken link ghosts.

## Architecture direction

- Prefer adding the durable filesystem graph to chan-drive. This repo
  should expose a narrow route/CLI wrapper.
- If chan-core already has enough filesystem metadata, reuse it. Do not
  traverse arbitrary paths with `std::fs` outside drive safety helpers.
- Keep wire types distinct from `/api/graph` semantic nodes unless a
  tagged union can represent both clearly.

## Candidate API

Freeze this in the task before webdev-1 starts:

- `GET /api/fs-graph?scope=file|folder&path=<rel>&depth=<n>`
- response `{ nodes, edges, root, scope }`
- node kinds: `folder`, `file`, `symlink`, `hardlink`, `ghost`
- edge kinds: `contains`, `symlink`, `hardlink`

Names are negotiable if the existing route conventions point elsewhere.

## Acceptance criteria

1. Server route returns folder/file graph data for drive-relative paths.
2. Depth 1 for a folder includes direct children and relation edges.
3. Symlink, hardlink, and broken symlink fixtures are covered by tests.
4. Errors preserve drive sandbox behavior and return existing JSON error
   style.
5. Wire type is documented in this task before frontend work consumes it.

## Verification

- Focused Rust tests for graph fixtures.
- `cargo test -p chan-server`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`

## Done means

Update this file with final route shape, sample payload, changed paths,
tests run, and mark `rustacean-2` REVIEW in `journal.md`.

---

## 2026-05-16 Execution

Status: DONE.

### Decision: route lives in chan-server

The fs-graph walker is implemented in `crates/chan-server/src/routes/
fs_graph.rs` rather than in chan-core. Reasoning:

- The existing `chan_drive::fs_ops::walk_drive` family DROPS symlinks
  and special files (correct for the content index, wrong for a
  graph). Reusing it is impossible (see syseng-1 survey item 2 and
  the architect-syseng-1 advisory).
- Adding a `walk_drive_with_specials` to chan-core would expand the
  chan-core surface for a route that has only one consumer today.
  We can move it later if a second consumer materializes (UniFFI
  mobile shells already link chan-drive without HTTP; they don't
  need an fs-graph walker yet).
- The chan-server implementation still routes path validation
  through `chan_drive::fs_ops::resolve_safe` so `..` traversal is
  rejected before any I/O. Every actual filesystem call uses
  `symlink_metadata` (lstat semantics), matching the chan-drive
  contract.

If chan-core grows a generic walker variant later, the route's
`FsGraphWalker` collapses to a thin wrapper.

### Final wire shape (frozen for webdev-1)

`GET /api/fs-graph?scope=file|folder&path=<rel>&depth=<n>`

Query parameters:
- `scope`: `file` or `folder`. Default `folder`.
- `path`: drive-relative path. Empty / missing / `/` means drive root.
  Leading slash is trimmed; `..` traversal returns the standard 400
  error shape via `resolve_safe`.
- `depth`: 1..=6 for `scope=folder`. Default 1. Clamped to 6 if a
  caller asks for more. Ignored for `scope=file` (depth reported as
  0 in the response).

Response body:

```
{
  "root": "<absolute drive root>",
  "scope": "file"|"folder",
  "path": "<normalized relative path>",
  "depth": <number>,
  "nodes": [
    {
      "id": "<id>",
      "kind": "folder"|"file"|"symlink"|"ghost",
      "name": "<basename>",
      "path": "<drive-relative path or ''>",
      "size": <bytes>,
      "mtime": <unix seconds or omitted>,
      "target": "<readlink target if symlink, else omitted>",
      "outside": true if symlink target is outside the drive,
      "broken": true if missing / unreadable
    }
  ],
  "edges": [
    {"source": "<id>", "target": "<id>", "kind": "contains"|"symlink"|"hardlink"}
  ],
  "truncated": false
}
```

Node id rules:
- In-drive entries use their drive-relative POSIX path. Drive root
  is the empty string `""`.
- Outside-drive symlink targets get a synthetic id
  `outside:<symlink-src-path>` so they appear as a distinct node
  without colliding with any real path.
- In-drive missing targets (broken symlinks) get the would-be
  drive-relative path with `broken: true`.

Edge kinds:
- `contains`: parent folder -> child (file, folder, symlink, ghost).
- `symlink`: symlink node -> classified target node (in-drive,
  ghost broken, or ghost outside).
- `hardlink`: between any two file paths sharing `(st_dev, st_ino)`,
  emitted lexicographically once per pair after the walk.

Caps: `MAX_DEPTH=6`, `MAX_NODES=10_000`. Exceeding either flips
`truncated: true` on the response. The walker prunes `.git/` and
`.chan/` at the drive root to match chan-drive's content-index
exclusions.

### Sample payload

Folder scope, depth 1, drive root with one file, one folder, one
in-drive symlink, one broken symlink, one outside-drive symlink,
one hardlink pair:

```
{
  "root": "/Users/alice/notes",
  "scope": "folder",
  "path": "",
  "depth": 1,
  "nodes": [
    {"id": "alias.md", "kind": "symlink", "name": "alias.md",
     "path": "alias.md", "size": 0, "target": "top.md"},
    {"id": "broken.md", "kind": "symlink", "name": "broken.md",
     "path": "broken.md", "size": 0, "target": "missing.md"},
    {"id": "escape.md", "kind": "symlink", "name": "escape.md",
     "path": "escape.md", "size": 0, "target": "/etc/hosts"},
    {"id": "missing.md", "kind": "ghost", "name": "missing.md",
     "path": "missing.md", "size": 0, "target": "missing.md",
     "broken": true},
    {"id": "outside:escape.md", "kind": "ghost",
     "name": "/etc/hosts", "path": "", "size": 0,
     "target": "/etc/hosts", "outside": true},
    {"id": "sub", "kind": "folder", "name": "sub", "path": "sub",
     "size": 0},
    {"id": "top.md", "kind": "file", "name": "top.md",
     "path": "top.md", "size": 8, "mtime": 1715844723},
    {"id": "twin.md", "kind": "file", "name": "twin.md",
     "path": "twin.md", "size": 8, "mtime": 1715844723}
  ],
  "edges": [
    {"source": "", "target": "alias.md", "kind": "contains"},
    {"source": "", "target": "broken.md", "kind": "contains"},
    {"source": "", "target": "escape.md", "kind": "contains"},
    {"source": "", "target": "sub", "kind": "contains"},
    {"source": "", "target": "top.md", "kind": "contains"},
    {"source": "", "target": "twin.md", "kind": "contains"},
    {"source": "alias.md", "target": "top.md", "kind": "symlink"},
    {"source": "broken.md", "target": "missing.md", "kind": "symlink"},
    {"source": "escape.md", "target": "outside:escape.md", "kind": "symlink"},
    {"source": "top.md", "target": "twin.md", "kind": "hardlink"}
  ],
  "truncated": false
}
```

(Field order is illustrative; `serde_json` keeps map-insertion order
which matches the struct declaration.)

### Files changed

- `crates/chan-server/src/routes/fs_graph.rs` (new)
- `crates/chan-server/src/routes/mod.rs` (module + re-export)
- `crates/chan-server/src/lib.rs` (import + route registration at
  `/api/fs-graph`)

### Tests (11 new)

- `folder_scope_depth_one_lists_direct_children`
- `folder_scope_deeper_includes_grandchildren`
- `drive_internal_dirs_are_hidden`
- `symlink_in_drive_target_existing` (unix)
- `symlink_broken_emits_ghost` (unix)
- `symlink_outside_drive_emits_outside_ghost` (unix)
- `symlink_loop_terminates` (unix; regression for `a -> b -> a`,
  which previously caused infinite recursion via `visit_entry ->
  emit_symlink_target -> visit_entry`)
- `hardlink_emits_hardlink_edge` (unix)
- `fifo_surfaces_as_ghost` (unix; shells out to mkfifo, skips if
  unavailable)
- `file_scope_emits_parent_contains_edge`
- `normalize_rel_strips_leading_slash_and_dot`

### Verification

```
cargo fmt --all -- --check                # clean
cargo build -p chan-server                # ok
cargo clippy --all-targets -- -D warnings # clean
cargo test -p chan-server                 # 78 passed (67 prior + 11 new)
```

### Residual risks

- The route picks up symlinks/hardlinks/specials with `std::fs`
  calls outside chan-drive's higher-level helpers. Path safety is
  preserved via `resolve_safe` on the request path; the walker
  never receives an arbitrary path. If a future maintainer pushes
  the walker into chan-core, keep the lstat-only contract and the
  visited-`(dev, ino)` set.
- Outside-drive symlink classification depends on `canonicalize`
  matching the canonical drive root when available. The fallback is
  now conservative: it only accepts clean lexical descendants of the
  drive root and rejects `..` escape components, so missing in-drive
  targets still show as ghosts without treating `root/../outside` as
  in-drive.
- The `MAX_NODES=10_000` cap is per-response. If a user expects a
  dashboard view of a large vault, they need to narrow the scope
  (folder + path filter) rather than expecting the whole tree.
  webdev-5 now surfaces `truncated: true` in the graph status bar.
