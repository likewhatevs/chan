# backend-2: Graph data and URL-state support audit

Owner: @@Backend+Rustacean.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [frontend-3.md](./frontend-3.md)
- [syseng-1.md](./syseng-1.md)

## Goal

Audit graph/data support for the phase 3 graph consistency work and screen
reloadability:

- Markdown/link graph mode can optionally show parent folders and path-to-root
  nodes as depth increases.
- Folder graph mode can optionally show Markdown cross-links and paths.
- Scope options include parent folders for `.md` files and the common ancestor
  folder when multiple `.md` files are in scope.
- Existing endpoints provide stable identifiers that the frontend can encode in
  the URL.

## Acceptance criteria

- Document what existing graph endpoints already provide.
- Implement backend support only for missing data that the frontend cannot
  derive safely.
- Keep graph types explicit; do not overload semantic Markdown nodes with
  filesystem nodes without type/classification fields.
- Freeze any changed response shape in this task before frontend relies on it.

## Test expectations

- Add focused route/unit tests for any graph response changes.
- Include filesystem edge cases where folders, symlinks, hardlinks, and ghosts
  are involved.
- Record exact verification commands.

## Review expectations

- @@Syseng review for filesystem/path semantics.
- @@Rustacean review for Rust implementation quality if code changes land.

## Progress notes

### Endpoint inventory

Two graph endpoints feed the frontend's graph mode:

#### `GET /api/graph` — Markdown / link graph

`crates/chan-server/src/routes/graph.rs:493-727`. Returns the unified
`{nodes, edges}` shape over chan-drive's link graph:

- Nodes are a tagged union:
  - `{kind: "file", id, label, path, node_kind?, missing}`. `id` and `path`
    are the same drive-relative POSIX path. `node_kind: "contact"` is set
    on files declared with `chan.kind: contact` frontmatter. `missing: true`
    marks ghost nodes (link target that doesn't exist on disk OR an in-drive
    symlink — see `indexed_file_exists` at line 341-345).
  - `{kind: "tag", id: "#name", label: "#name"}`.
  - `{kind: "mention", id: "@@name", label: "@@name"}`. Mentions that resolve
    to a `Contacts/<name>.md` file are rewritten to point at the file node
    (line 590-603) and the standalone `@@name` node is suppressed.
- Edges: `{source, target, kind: "link"|"tag"|"mention", broken?}`. `broken`
  is meaningful only for `link` edges.

Image files referenced by markdown (`![](pic.png)`) are merged in as file
nodes when an actual file exists on disk (line 520-572) so a markdown image
link lands on a real node, not a ghost.

#### `GET /api/fs-graph` — Filesystem / folder graph

`crates/chan-server/src/routes/fs_graph.rs:188-197`. Returns
`{root, scope, path, depth, nodes, edges, truncated}` over the on-disk tree:

- `NodeView{id, kind: "file"|"dir"|"symlink"|"ghost-outside"|"ghost-broken",
  name, path, size, mtime?, target?, outside?, broken?}`. In-drive entries
  use the drive-relative POSIX path as `id`; outside-drive symlink targets
  use `outside:<symlink-src>`.
- `EdgeView{source, target, kind}`. Folder containment is the natural edge
  kind here.
- `MAX_NODES = 10_000` (file `fs_graph.rs:47`). `truncated: true` flags an
  early-stop response so the frontend can surface the partial-graph
  warning.

#### Shared identifier model

Both endpoints use the same drive-relative POSIX path for in-drive files,
so a `Set<path>` join across the two responses is safe and well-defined.
That is the foundation for the cross-overlay work the request describes:
the frontend can fetch both, intersect on `path`, and render the union.

### Per-requirement assessment

#### "Markdown graph shows parent folders / path to root as depth increases"

No backend change required. Each file node already exposes its full
drive-relative `path`. The frontend can synthesize a `folder:<dir>` node
per parent component and emit containment edges. Recommended frontend
node convention: `folder:<path>` (matching the language-graph route's
`folder_node_id` format at `graph.rs:351-353`) with `kind: "folder"`.

If the frontend wants a server-side helper for this later, the data is
deterministic from the existing response — adding it now would just be
duplicating frontend logic on the server.

#### "Folder graph shows Markdown cross-links and paths"

No backend change required. Fetch `/api/fs-graph` for the folder/file
structure, fetch `/api/graph` for the link edges, intersect file ids,
overlay. Both endpoints already speak the same path identifier.

Documented edge case for the frontend: `/api/graph` treats in-drive
symlinks as `missing: true` (the indexer skips them under chan-drive's
lstat semantics — `graph.rs:341-345`), but `/api/fs-graph` exposes them
as `kind: "symlink"`. When the folder graph overlays markdown links, a
symlinked `.md` file will appear as a symlink node from /api/fs-graph
with no incoming link edges from /api/graph. That asymmetry is
deliberate (indexer truth) and shouldn't be smoothed over on the
backend — it's worth a frontend tooltip or muted styling.

#### "Scope options: parent folder of an `.md` file"

No backend change required. Lexical derivation from
`path.lastIndexOf('/')`. The frontend already has this logic for the
graph overlay (`web/src/state/store.svelte.ts::availableGraphScopes`
constructs `dir:<path>` scope ids — see line 2287).

#### "Scope options: common ancestor of multiple `.md` files"

No backend change required. Pure lexical operation on a list of
drive-relative paths (longest common prefix at `/` boundary). The
frontend should walk the visible-file set, compute the LCP, and
surface the result as another `dir:<path>` scope option.

#### "URL-stable identifiers"

Already complete. Stable identifiers across requests:

- File: drive-relative POSIX path. URL-safe after `encodeURIComponent`.
- Folder: drive-relative POSIX path (same encoding).
- Tag: `#name` (URL-safe after encoding the `#`).
- Mention: `@@name` (URL-safe after encoding).
- Conversation: opaque key owned by the frontend
  (`drive` | `file:<path>` | `group:<sorted-paths>`). Already URL-encodable.
- Graph scope: `drive` | `file:<path>` | `dir:<path>` | `language:<lang>` |
  `global` (see store.svelte.ts and language_graph route).
- Index status overlay: no parameter, single-instance.

These are the identifiers the frontend already uses internally; the
URL-state work in [frontend-1.md](./frontend-1.md) can lift them
verbatim. No backend rename/migration required.

### Frozen response shapes

No graph response shape changes in this task. Frontend can rely on the
current `GET /api/graph` and `GET /api/fs-graph` payloads as-is for the
overlay/filter work in [frontend-3.md](./frontend-3.md).

If a future filter/scope requirement reveals data the frontend cannot
synthesize safely (e.g., a "links across folder boundaries" aggregation
that needs server-side counts to avoid pulling the full edge list on a
large drive), spin a follow-up task and add the helper route then — keep
the existing types narrow per the acceptance criteria.

### Tests run

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -p chan-server
```

All pass. No new code in this audit task, so no new tests added.

## Commit readiness notes

No code changes. This is documentation only — nothing to commit. The
frontend work in [frontend-3.md](./frontend-3.md) can proceed against
the existing graph endpoints without waiting on a backend change.
