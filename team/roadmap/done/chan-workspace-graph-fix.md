# Unify Workspace Search and Graph Traversal

> Status: shipped in [v0.71.0](../../release/release-v0.71.0.md).

## Summary

Add one bounded search and traversal contract to `chan-workspace` and use it from `cs`, `chan workspace`, chan-server, and `chan-llm`. The contract combines content retrieval, lexical entity lookup, exact typed seeds, and bounded graph traversal without transferring the whole visualization graph.

The existing SPA graph is a compatibility boundary for this work. Keep `/api/graph`, `/api/fs-graph`, `/api/graph/languages`, streaming graph delivery, the filesystem spine, `GraphPanel` traversal, and `cs graph [path]` unchanged while the new contract proves semantic parity. A later change may migrate one SPA lens at a time, but whole-workspace visualization continues to use `/api/graph`.

No query mutates the workspace registry, enables reports, starts a report scan, rebuilds an index, or opens a second handle around a live workspace lock. Cross-workspace search is fan-out over independent workspace-local results. It never creates cross-workspace nodes or edges.

## Current Contract

### SPA graph

The SPA consumes several graph projections:

- `graphData.svelte.ts` streams `/api/graph` and caches the unified semantic graph.
- `GraphPanel.svelte` also loads `/api/fs-graph` for the initial filesystem spine and filesystem-only views.
- `/api/graph` joins the Markdown graph database with filtered filesystem entries, report buckets, language nodes and edges, contact classification, resolved mentions, and resolved on-disk link targets.
- `/api/graph/languages` retains a separate language-to-directory roll-up for the language overview.
- `GraphPanel` applies the selected lens after loading the graph. File lenses use forward BFS, tag and mention lenses use bidirectional BFS, contact lenses use bidirectional BFS from the contact file, language lenses use one hop, and directory lenses use filesystem depth and expanded ancestors.
- Tag, mention, and contact lenses add the bounded incident meta-node closure from `lensClosure.ts` so a surfaced file keeps its other tag, mention, and language relationships.
- Every surfaced file is reattached to the workspace root by following `contains` edges toward its ancestors.
- `GRAPH_DEPTH_HARD_MAX` and `FS_GRAPH_DEPTH_MAX` are both 10.

This is the behavior the shared traversal engine must reproduce for equivalent seeds. It is not an instruction to move the SPA onto the new endpoint during this change.

### `cs`

`cs graph [path]` sends `ControlRequest::OpenGraph` and opens the graph UI in the current workspace window. It returns no graph data.

`cs search` sends `ControlRequest::Search` to the current tenant and formats the JSON reply as Markdown or JSON. The server calls `Workspace::search` with `SearchOpts::default()`, whose mode is BM25. This differs from `/api/search/content`, which selects hybrid retrieval when semantic search is enabled and the configured model is available.

### `chan workspace`

`chan workspace search <path> <query>` and `chan workspace graph <path>` call `ensure_workspace_registered` before opening a workspace. A read-only query can therefore mutate the registry. The open then fails with `WorkspaceLocked` or `WorkspaceAlreadyOpen` if a server already holds the workspace.

`chan workspace graph --scope all` reads `GraphView::files()` and then calls `neighbors()` once per file. Its result contains graph-indexed Markdown or text file paths plus link, tag, and mention edges. It does not carry the SPA's directory nodes, containment spine, report-backed languages, link-target resource normalization, contact node classification, or resolved mention-to-contact targets.

`--scope file` and `--scope directory` delegate to `chan_server::build_fs_graph`, so the top-level CLI mixes a chan-workspace semantic graph with server-owned filesystem graph behavior. `--scope all` also performs one neighbor query per node and can dump an unbounded node set before the text-only edge display limit is applied.

### `chan-llm`

`chan-llm` exposes four overlapping read tools: `search_content`, `graph_neighbors`, `graph_tags`, and `graph_files_with_tag`.

- `search_content` always uses the default BM25 mode instead of the effective mode used by the UI.
- `graph_neighbors` calls `GraphView::backlinks()` for inbound traversal. `backlinks()` has `AND kind = 'link'`, so inbound tag and mention relationships can never appear even when requested.
- `graph_files_with_tag` documents and schemas a tag with its leading `#`, then `GraphView::files_with_tag` adds another leading `#`. A documented input such as `#design` queries `##design`.
- Directory, containment, contact, resolved mention, language, linked resource, and source-file semantics do not match the SPA graph.
- `graph_tags` is a separate taxonomy API that the general search contract can cover with a query-free tag domain.

`chan-llm` is MCP-only and remains scoped to its active workspace. Multi-workspace fan-out belongs to the top-level `chan workspace` command.

### Architectural gap

Graph meaning is currently split among `chan-workspace`, chan-server route helpers, `chan` CLI glue, chan-llm tool handlers, and frontend traversal. The result depends on which surface is asked. The shared contract moves search policy, entity identity, relationship normalization, and bounded traversal into `chan-workspace`; outer crates only parse, route, aggregate, or format it.

## Core Contract

### Public entry point

Add a `workspace_search` module to `chan-workspace`, re-export its public request and result types from `lib.rs`, and add this method:

```rust
impl Workspace {
    pub fn workspace_search(
        &self,
        request: &WorkspaceSearchRequest,
    ) -> Result<WorkspaceSearchResult>;
}
```

The outer `Result` reports infrastructure failures that prevent a coherent response, such as SQLite, Tantivy, or filesystem errors. Request, selector, readiness, ambiguity, and unavailable-domain failures are serializable entries in `WorkspaceSearchResult.errors`. This lets one valid selector still produce a result when another selector fails and gives every transport the same partial-result semantics.

All public wire values derive `Debug`, `Clone`, `Serialize`, `Deserialize`, and the equality traits their fields permit. Keep the types owned and lifetime-free so they remain suitable for the existing native-shell boundary.

### Request

The public request shape is:

```rust
pub struct WorkspaceSearchRequest {
    pub query: Option<String>,
    pub from: Vec<WorkspaceSelector>,
    pub domains: Vec<WorkspaceSearchDomain>,
    pub depth: Option<u8>,
    pub direction: WorkspaceTraversalDirection,
    pub relationship_kinds: Vec<WorkspaceRelationshipKind>,
    pub limit: Option<u32>,
    pub node_limit: Option<u32>,
    pub edge_limit: Option<u32>,
}

pub struct WorkspaceSelector {
    pub kind: WorkspaceSelectorKind,
    pub value: String,
}

pub enum WorkspaceSelectorKind {
    File,
    Directory,
    Tag,
    Mention,
    Contact,
    Language,
}

pub enum WorkspaceSearchDomain {
    Content,
    File,
    Directory,
    Tag,
    Mention,
    Contact,
    Language,
}

pub enum WorkspaceTraversalDirection {
    Auto,
    Out,
    In,
    Both,
}

pub enum WorkspaceRelationshipKind {
    Link,
    Tag,
    Mention,
    Language,
    Contains,
}
```

Use lowercase snake-case serde names. A typed selector serializes as `{"kind":"file","value":"notes/a.md"}`. The CLI spelling `file:notes/a.md` is an adapter representation of the same value, not a second selector model.

Normalize the request once in `chan-workspace` before any query:

- Trim `query`; an empty string becomes absent.
- Deduplicate selectors, domains, and relationship kinds while preserving first occurrence for diagnostics.
- An omitted or zero `limit` becomes 20. Clamp it to 100.
- An omitted or zero `node_limit` becomes 100. Clamp it to 1,000.
- An omitted or zero `edge_limit` becomes 250. Clamp it to 2,500.
- An explicit depth is clamped to 10.
- If at least one explicit selector exists, omitted depth becomes 1.
- With no explicit selector, omitted depth becomes 0. This covers plain text search and query-free taxonomy browsing.
- An empty `domains` list means content plus every entity domain when a query is present, no lexical stage when only explicit selectors are present, and is invalid when there is neither a query nor a selector.
- An empty `relationship_kinds` list means every relationship kind.
- A valid request has a non-empty query, at least one selector, or at least one non-content browse domain. `content` by itself with no query is not a browse request.
- Values above a hard limit are clamped and reported through a structured `limit_clamped` warning. Enum decode errors remain transport parse failures.

Domains restrict content and entity matching. They do not suppress node kinds reached through traversal. Relationship filters control graph traversal and graph relationships, with the containment-spine exception described below.

### Canonical selectors

Every resolved entity has one reusable canonical selector:

- File values are normalized workspace-relative POSIX paths. Contacts do not also appear as file entities.
- Directory values are normalized workspace-relative POSIX paths. The workspace root is the empty string in JSON and `.` in CLI input.
- Tag input accepts `design` or `#design`, strips exactly one optional leading `#`, and emits canonical value `design`.
- Mention input accepts `alice` or `@@alice`, strips exactly one optional leading `@@`, and emits the graph's canonical display spelling without the sigil.
- Contact canonical values are exact contact file paths. A user-provided contact value may match path, title, basename, email, or alias before it resolves to that path.
- Language input matches case-insensitively and emits the report's canonical language spelling.

File and directory normalization rejects absolute paths, `..` escapes, symlinks that escape the workspace, and the same special-file cases as existing Workspace operations. A file selector may address Markdown, text, source, media, or another regular file. It never causes source content to enter the search index.

When a user value can resolve to more than one entity, return `ambiguous_selector` with the original selector and the candidates' canonical selectors. Do not choose a candidate silently. An exact canonical selector always wins over fuzzy fields.

Mention-to-contact normalization is a separate compatibility rule from interactive contact lookup. Centralize the current basename-stem and top-level alias resolver in chan-workspace. It accepts `GraphView::contacts()` order and records every candidate plus the selected compatibility candidate. `/api/graph` continues to use the same selected candidate it emits today; the new engine can surface ambiguity metadata without changing the visualization. Add collision tests so any future policy change is explicit.

### Result

The result shape is:

```rust
pub struct WorkspaceSearchResult {
    pub workspace: WorkspaceSearchIdentity,
    pub search: WorkspaceSearchStatus,
    pub content_hits: Vec<WorkspaceContentHit>,
    pub entity_matches: Vec<WorkspaceEntityMatch>,
    pub nodes: Vec<WorkspaceGraphNode>,
    pub relationships: Vec<WorkspaceRelationship>,
    pub traversal: EffectiveWorkspaceTraversal,
    pub truncation: WorkspaceSearchTruncation,
    pub warnings: Vec<WorkspaceSearchWarning>,
    pub errors: Vec<WorkspaceSearchError>,
}
```

`WorkspaceSearchIdentity` carries the canonical root, stable metadata key, and effective display name. The effective display name is the configured `KnownWorkspace::display_name` when present, otherwise the root basename. Add read-only Workspace accessors for these fields instead of having callers reach into registry state.

`WorkspaceSearchStatus` carries whether content retrieval was requested, whether the search index was ready, and the effective mode: `not_run`, `bm25`, or `hybrid`. Explicit-seed traversal can succeed while content search is not ready.

`WorkspaceContentHit` retains the current path, chunk ID, heading, start line, snippet, and score fields. Collapse chunks to the best hit per file before applying `limit`, matching `/api/search/content` and `cs search` behavior. Break equal scores by path, start line, and chunk ID.

`WorkspaceEntityMatch` carries:

- Stable entity ID.
- Entity kind and display label.
- Canonical selector.
- Match class: `exact`, `prefix`, `substring`, or `browse`.
- Matched field and matched value where useful.
- Optional path, reference count, file count, and code-line count when the entity has them.

Do not expose arbitrary file contents through entity metadata. Contact email and alias values may be returned only when they are the matched field or already part of the contact projection.

Use graph IDs compatible with the current SPA so parity compares IDs directly:

- File and contact: workspace-relative path.
- Directory: `directory:<path>`, with the workspace root ID as the empty string.
- Tag: `#<name>`.
- Mention: `@@<name>`.
- Language: `language:<canonical language>`.

`WorkspaceGraphNode` is a tagged enum with `file`, `directory`, `tag`, `mention`, `contact`, and `language` variants. File nodes carry a file class that distinguishes Markdown/text, source, media, binary, and other regular resources. Contact nodes carry their file path plus title, basename, emails, and aliases. Directory, tag, mention, and language nodes carry the counts already available from graph or report data. Unresolved missing link targets do not become graph nodes or dangling relationships, matching the current `/api/graph` behavior. An explicit missing file or directory seed produces a selector error.

`WorkspaceRelationship` carries source ID, target ID, relationship kind, optional link anchor, and optional `broken` flag. Normal successful traversal emits only relationships whose endpoints are both present. Deduplicate by `(source, target, kind, anchor)`.

`EffectiveWorkspaceTraversal` records normalized depth, requested direction, selected relationship kinds, whether containment spines were forced, and one effective profile per seed. Query-derived content hits and file entity matches are recorded as file profiles; entity matches retain their entity kind.

`WorkspaceSearchTruncation` has independent flags and observed counts for content hits, entity matches, graph nodes, and graph edges. It also reports whether traversal stopped before draining its frontier. A caller must not infer graph completeness from `nodes.len() < node_limit` because an edge limit can stop the walk first.

Warnings and errors are tagged enums with stable lowercase codes and a factual message. Errors include at least:

- `invalid_request`.
- `invalid_selector`.
- `selector_not_found`.
- `ambiguous_selector` with canonical candidates.
- `index_not_ready`.
- `domain_unavailable` with the affected domain.

Warnings include at least:

- `limit_clamped` with requested and effective values.
- `reports_disabled`.
- `reports_unavailable`.
- `hybrid_unavailable` when the semantic preference is on but the model cannot be resolved and BM25 is used.
- `missing_link_target` when traversal encounters and omits an unresolved target.

Adapters treat a result with a non-empty `errors` list as unsuccessful for exit status, but they still render or return all successful fields.

## Retrieval and Entity Lookup

### Effective retrieval mode

Move the BM25/hybrid decision into chan-workspace. Add one helper used by both `Workspace::workspace_search` and compatibility adapters:

```rust
pub fn effective_search_mode(&self) -> Result<EffectiveSearchMode>;
```

With the embeddings feature, hybrid is effective only when the workspace preference is enabled and the configured model resolves. Otherwise BM25 is effective. Without embeddings, BM25 is always effective. Delete the duplicate policy functions from `crates/chan/src/lib.rs` and `crates/chan-server/src/routes/search.rs`, and stop relying on `SearchOpts::default()` at call sites.

A content request never initiates a rebuild. While a cold index is unavailable or reindexing has no queryable snapshot, return `ready: false`, preserve entity and explicit-seed work that can run safely, and add `index_not_ready`. An empty workspace with a ready empty index is ready and returns no hits.

### Entity catalog

Unqualified query text searches content and lexical graph entities. Build the entity catalog from existing maintained metadata plus path-only filesystem information:

- Markdown and text paths from the graph and search indexes.
- Existing resolved graph link targets.
- Source and other report-tracked paths from maintained report data.
- Existing linked media and other on-disk resources by path.
- Directories derived from matching file paths and targeted directory listings.
- Tags and mentions from graph SQL aggregates.
- Contacts from graph node metadata.
- Languages from maintained report rows.

The engine may perform one filtered, metadata-only workspace walk to match arbitrary file and directory paths. It must not read source contents. It must not assemble the SPA's full node-and-edge payload. Link normalization uses the current on-disk file and directory sets, but graph edges are fetched only for the bounded frontier.

Classify contact files only as contacts in entity results so the same path does not occupy two ranks. A contact still participates in `contains`, `link`, `mention`, and language relationships under its path ID.

For a non-empty query, compare a case-folded query against the kind-specific fields and keep the best match class for each entity. Rank by:

1. Exact match.
2. Prefix match.
3. Substring match.
4. Kind order: file, directory, tag, mention, contact, language.
5. Stable entity ID.

Path, title, basename, email, and alias are contact match fields. Path and basename are file match fields. Bare name and sigil-prefixed name are tag and mention match fields. Canonical name is the language match field.

When no query is present, a non-content domain is a bounded browse request:

- Tags and mentions sort by reference count descending, then stable ID.
- Languages sort by file count descending, code lines descending, then stable ID.
- Contacts sort by effective display label, then path.
- Files and directories sort by path.

Apply `limit` independently to content hits and entity matches. Set the corresponding truncation flag when more candidates exist.

### Report availability

The query path must never call `set_reports_enabled`, `boot`, or the current `Workspace::report()` cold path. Add a non-scanning report snapshot accessor:

```rust
pub fn report_if_available(&self) -> Result<Option<Report>>;
```

It returns the in-memory maintained snapshot when the report `OnceLock` is warm. Otherwise, if reports are enabled and a valid persisted JSONL exists, load that JSONL without falling back to `Index::scan`. Return `None` when reports are disabled, the JSONL is absent, or it is invalid. The search engine distinguishes disabled from unavailable for warnings by checking `reports_enabled()` first.

Language entity lookup and language relationships are omitted when report data is unavailable. An explicit language selector produces `domain_unavailable`; an unqualified query or browse request produces a warning and preserves other domains.

## Relationship Normalization

Move the current server-only link and contact normalization into reusable chan-workspace helpers before using it in the new engine:

- Percent-decode link targets.
- Resolve workspace-rooted, source-relative, and ancestor-relative candidates in the current `/api/graph` order.
- Try exact, `.md`, and `.txt` paths.
- Recognize existing linked source and media files as resources.
- Drop directory link targets and unresolved missing targets from the graph projection.
- Preserve link anchors.
- Resolve mention destinations to contact paths using the current basename-stem and alias rules.
- Leave unresolved mentions as `@@name` nodes.

The helpers operate on supplied graph rows, contact rows, and filesystem path sets. They do not query chan-server state and do not write normalized values back to SQLite. Update `/api/graph` to call the helpers with the same inputs it uses today. Snapshot and behavior tests must show semantically identical nodes and relationships before the new engine relies on them.

Do not add a persistent resource, directory, language, or normalized-edge table. Link, language, and containment projection remains derived from the graph SQLite data, maintained report data, and current filesystem metadata.

## Traversal

### Seed formation

Build seeds in this stable order:

1. Explicit `from` selectors in request order after canonical deduplication.
2. Content hits in ranked order when depth is greater than 0.
3. Entity matches in ranked order when depth is greater than 0.

At depth 0, explicit selectors still emit their resolved seed nodes and complete containment spines, but no relationship frontier is expanded. A query-only or browse-only depth-0 request returns ranked matches with empty graph arrays.

For mixed automatic seeds, apply each seed's profile independently and union the results by node and relationship ID. Track minimum distance for deterministic ordering. Do not let a node visited by one profile suppress required post-processing for another profile.

### Automatic profiles

The `auto` direction reproduces current SPA behavior:

- File: forward traversal along outgoing relationships for `depth` hops.
- Tag: bidirectional traversal for `depth` hops.
- Mention: bidirectional traversal for `depth` hops.
- Contact: bidirectional traversal from the contact file for `depth` hops.
- Language: exactly one bidirectional hop, regardless of requested depth.
- Directory: select descendants by filesystem depth, where depth 1 means direct children. Semantic relationships do not cause directory traversal to escape the selected subtree.

An explicit `out`, `in`, or `both` replaces the automatic direction for semantic traversal. Language still has a one-hop cap. Directory membership remains filesystem-depth based: `out` and `both` enumerate descendants, while `in` emits the selected directory plus its mandatory ancestor spine. Relationship filters control which incident semantic relationships are retained for selected directory files.

Use only selected relationship kinds to expand the frontier. The default includes link, tag, mention, language, and contains. A link relationship points from source file to resolved target. Tag and mention relationships point from file to meta-node or resolved contact. Language points from language to file. Contains points from directory to child.

### Closure and containment

After BFS, apply the same bounded closure the SPA uses:

- For tag, mention, and contact profiles, add every incident tag, mention, and language meta-node for each surfaced file when that relationship kind is selected.
- Closure adds meta-nodes only. It never follows through a meta-node to another file.
- File and language profiles do not receive this extra meta-node closure.
- Directory profiles retain the incident selected meta-nodes of files already selected by filesystem depth, but do not use them to leave the subtree.

Every surfaced existing file or contact receives its complete `contains` chain to the workspace root. Spine nodes and edges are mandatory structural output even when `contains` is absent from `relationship_kinds` or the direction points outward. Record `spine_forced: true` in the effective traversal settings so callers can distinguish this exception.

Reserve node and edge capacity for a file's missing spine before admitting the file. If the complete spine cannot fit, omit that candidate file and set graph truncation rather than returning a parentless file. The explicit seed and its spine have first claim on the budget. Meta-node closure is bounded by the remaining limits and may set truncation without removing an already admitted file.

### Batched frontier queries

Add batched read methods to `GraphView`:

```rust
pub fn edges_from(
    &self,
    nodes: &[String],
    kinds: &[EdgeKind],
) -> Result<Vec<Edge>>;

pub fn edges_to(
    &self,
    nodes: &[String],
    kinds: &[EdgeKind],
) -> Result<Vec<Edge>>;
```

These methods query all semantic relationships for a frontier batch. `edges_to` must not call or inherit `backlinks()`, because `backlinks()` intentionally filters to link edges for its existing callers. Keep `neighbors()` and link-only `backlinks()` as compatibility methods.

Use chunked `IN` queries with bound parameters and `ORDER BY src, kind, dst, anchor`. A frontier may require several chunks to stay below SQLite's bind limit, but it must not issue one query per node. The existing primary key supports outgoing lookup and `edges_dst_idx` supports incoming lookup, so no graph schema migration is required.

Containment and language relationships are joined to the frontier in memory from one path map and one available report snapshot per request. Do not call `report()` or walk the full filesystem once per hop.

### Bounds and ordering

Traversal is breadth-first. Sort each fetched relationship batch before admitting new nodes. Deduplicate visited nodes by stable ID and relationships by their full key so cycles terminate naturally.

Return nodes ordered by minimum hop, node-kind order, and stable ID. Return relationships ordered by the hop that admitted them, relationship-kind order, source, target, and anchor. Spine edges for a node share that node's hop. This ordering is identical across SQLite plans, platforms, transports, and repeated runs.

Stop admitting ordinary traversal work at the effective node or edge limit. Always finish the already-reserved complete spine for an admitted file. Never exceed the hard maxima of 1,000 nodes or 2,500 relationships.

## Surface Mapping

### `cs search`

Replace the current search-only arguments with:

```text
cs search [QUERY...] [--from TYPE:VALUE]... [--domain DOMAIN]...
          [--depth N] [--direction auto|out|in|both]
          [--edge-kind KIND]... [--limit N]
          [--node-limit N] [--edge-limit N]
          [--json [--pretty]]
```

Clap accepts an absent query because selectors and browse domains are valid alternatives. Perform the core validity check after parsing so `cs search --domain tag` works and a bare `cs search` prints one precise usage error.

A plain query has depth 0 and performs no graph frontier work. It keeps the cheap content-search path while also returning bounded lexical entity matches. `--depth N` expands ranked content and entity matches. `--from` without an explicit depth performs one-hop traversal.

Human output remains Markdown and is divided into non-empty Content, Entities, Graph, Warnings, and Errors sections. JSON prints the `WorkspaceSearchResult` returned by the server without reshaping it; `--pretty` changes whitespace only.

Do not change `ShellAction::Graph`, `ControlRequest::OpenGraph`, or their dispatch. `cs graph [path]` remains a UI opener.

### Control socket

Add `ControlRequest::WorkspaceSearch { request: WorkspaceSearchRequest }`. The workspace tenant calls the core method on its live `Arc<Workspace>` and returns compact core JSON in `ControlResponse::Ok.message`. A terminal-only tenant receives the existing workspace-only refusal family.

Make `WorkspaceSearchRequest` available to chan-shell's wire layer through a default-feature-free chan-workspace dependency rather than defining a second wire copy. chan-server and chan-library already link chan-workspace without embeddings; this does not create a dependency cycle.

Keep the old `ControlRequest::Search` only as a short-lived internal compatibility adapter while call sites and wire tests move, then remove it in the same implementation because this project does not retain pre-release wire compatibility. Its adapter, if needed during the edit, constructs a depth-0 content request and projects the old response.

Extend the `Identify` reply with optional `workspace_root` and `metadata_key` fields. Workspace tenants fill both from their live Workspace identity. Terminal-only tenants omit both. `kind`, `version`, and `pid` retain their existing meanings.

Add exact serialization tests for `WorkspaceSearch` and the extended identity. Preserve the exact serialized shapes of `OpenGraph { path: None }` and `OpenGraph { path: Some(...) }`; this is the byte/shape guard for `cs graph`.

### HTTP

Add an authenticated route:

```text
POST /api/search/workspace
Content-Type: application/json

WorkspaceSearchRequest -> WorkspaceSearchResult
```

The handler lives in `crates/chan-server/src/routes/search.rs`, runs the synchronous core call on `spawn_blocking`, and returns the core result without a server-owned response copy. Invalid JSON or enum values are HTTP 400. A syntactically valid request with structured core errors remains HTTP 200 so successful partial fields are not discarded.

Keep `GET /api/search/content` and its current response shape. Implement it as a depth-0, content-only compatibility projection using the core retrieval-mode helper and content stage. Preserve its legacy `scope` prefix as adapter-only behavior; do not add that prefix field to the shared request. Keep `/api/search/content` tests for one-hit-per-file collapse, empty queries, readiness, and mode.

Keep `/api/graph`, `/api/fs-graph`, `/api/graph/languages`, and graph streaming routes unchanged except for calling the moved normalization helpers.

### `chan workspace`

Replace the positional workspace path and incomplete graph scope with:

```text
chan workspace search [QUERY...] [shared search/traversal flags]
                      [--workspace SELECTOR]... | [--all-workspaces]

chan workspace graph --from TYPE:VALUE [shared traversal flags]
                     [--workspace SELECTOR]... | [--all-workspaces]
```

`graph` is a traversal-only alias. It requires at least one `--from`, sets no query or browse domains, and sends the same core request. Remove `GraphScope`, `--scope`, `--target`, and the whole-workspace graph dump.

Workspace selection uses the read-only `Library::list_workspaces()` snapshot:

- A selector first matches a registered canonical or recorded root path, then an exact metadata key, then a case-insensitive display name.
- A display name must resolve uniquely. Otherwise return a structured workspace-selection ambiguity with the matching roots and metadata keys.
- With no selector, canonicalize the current directory and choose the registered root that contains it. If nested workspaces match, choose the longest root.
- Repeatable explicit selectors preserve request order and deduplicate by metadata key on first occurrence.
- `--all-workspaces` queries every registered workspace sorted by canonical root for stable output.
- `--workspace` and `--all-workspaces` conflict.
- No selection path calls `register_workspace` or updates `last_seen_at`.

Return a top-level aggregate for this command:

```rust
pub struct MultiWorkspaceSearchOutput {
    pub results: Vec<WorkspaceSearchResult>,
    pub errors: Vec<WorkspaceExecutionError>,
}
```

Each result remains workspace-local. Never merge nodes, edges, ranks, or limits across workspaces. Keep successful results when another workspace fails. JSON always emits the aggregate, including for one workspace. Human output uses one workspace heading per result and prints per-workspace failures after the successful sections. Exit nonzero when aggregate errors exist or any core result has structured errors.

### Live workspace routing

Read-only direct opening is not safe around the writer lock because it either fails or risks stale sidecar access if the lock is bypassed. Route each selected workspace with this algorithm:

1. Read the registered entry and its lock state.
2. If a live foreign holder exists, enumerate the holder process's pid-named and stable control sockets, issue bounded `Identify` requests, and select the socket whose `workspace_root` and `metadata_key` both match the registry entry.
3. Send `WorkspaceSearch` to that exact tenant. A pid match alone is insufficient because a devserver or desktop process can serve several workspaces.
4. If no live holder exists, call `Library::open_workspace` normally and execute the core method on the direct handle.
5. If the direct open returns `WorkspaceLocked`, re-read the lock, resolve the exact live tenant, and retry over its control socket once. This closes the race where a server acquires the lock after step 1.
6. If a live lock has no reachable exact tenant socket, return `served_workspace_unreachable`. Do not open SQLite or Tantivy sidecars directly and do not fall back to an arbitrary socket from the same pid.

Use the existing bounded stable-socket probe timeout. Factor socket enumeration and identity matching so `chan ps`, `chan close`, and workspace search share discovery primitives where their matching requirements overlap. `chan close` may remain path-routed, but workspace search must use the exact tenant.

Run multi-workspace queries sequentially in selected order. Each query is already bounded, and sequential execution avoids opening many Tantivy indexes and SQLite pools at once.

### `chan-llm`

Replace `search_content`, `graph_neighbors`, `graph_tags`, and `graph_files_with_tag` with one canonical `workspace_search` tool. Keep `read_file`, `write_file`, `read_media`, `list_files`, `resolve_path`, `repo_report`, and unrelated tools unchanged.

The MCP input uses typed selector objects rather than `TYPE:VALUE` strings:

```json
{
  "query": "design",
  "from": [
    { "kind": "tag", "value": "architecture" },
    { "kind": "file", "value": "docs/design.md" }
  ],
  "domains": ["content", "tag", "file"],
  "depth": 2,
  "direction": "auto",
  "relationship_kinds": ["link", "tag", "contains"],
  "limit": 20,
  "node_limit": 100,
  "edge_limit": 250
}
```

The tool returns the serialized core result unchanged. It covers content search, query-free entity browsing, exact neighbors, tag membership, mention and contact resolution, language-to-file relationships, linked source and media resources, and filesystem containment.

Define one `WorkspaceSearchParams` in chan-llm, deserialize it for standard dispatch, and use the same type to generate both the standard tool JSON schema and rmcp schema. Convert it mechanically into `chan_workspace::WorkspaceSearchRequest`. Do not keep separate hand-written schema properties in `tools.rs` and `mcp.rs`. If schema generation requires `schemars` in the non-MCP chan-llm build, make that existing dependency unconditional rather than duplicating the schema.

Update the shared prompt catalog and session directive to name only `workspace_search` for these operations. Keep the description parity test, but make it compare generated schemas and the one shared description rather than duplicated macro literals where rmcp permits.

The tool always uses the active `ToolContext` workspace. It does not accept workspace selection or fan out.

## SPA Compatibility Boundary

The initial implementation does not change the frontend production data path:

- Do not call `/api/search/workspace` from `GraphPanel`.
- Do not replace `/api/graph` or its NDJSON streaming batches.
- Do not remove `/api/fs-graph` or its cursor-paged traversal.
- Do not change `GraphPanel` scope parsing, BFS, filesystem expansion, filter chips, meta-node closure, or containment-spine logic.
- Do not change the language overview endpoint.
- Do not change `cs graph [path]`.

Retain and extend tests for:

- Filesystem spine construction.
- Filesystem depth caps and expansion.
- File forward BFS.
- Tag bidirectional BFS.
- Mention bidirectional BFS.
- Contact bidirectional BFS.
- Language one-hop behavior.
- Meta-node closure.
- Parent-edge invariants.

Add a shared golden fixture under `fixtures/workspace-search/`. The fixture contains a normalized full graph input plus expected visible node IDs and relationship keys for file, directory, tag, mention, contact, and language lenses at depths 0, 1, and 2 where meaningful. Rust core tests execute the new engine against the workspace fixture and compare the result sets. Vitest executes the current lens helpers against the normalized graph input and compares the same expected sets. This pins semantic parity without coupling the production SPA to the new endpoint.

Refactoring contact or link normalization is allowed only after existing `/api/graph` JSON and streaming behavior tests pass with the same final node and relationship sets. Streaming batch boundaries need not be byte-identical, but the final merged graph must be semantically identical.

## Implementation Layout

Keep the public API narrow and put implementation details behind the workspace method:

- `crates/chan-workspace/src/workspace_search.rs`: public request/result types, normalization, entity ranking, traversal coordinator, limits, and ordering.
- `crates/chan-workspace/src/graph.rs`: batched incoming/outgoing queries and compatibility wrappers.
- `crates/chan-workspace/src/graph_normalize.rs`: reusable link and mention/contact normalization.
- `crates/chan-workspace/src/report.rs` and `workspace.rs`: non-scanning report snapshot and Workspace identity accessors.
- `crates/chan-server/src/routes/search.rs`: POST adapter and legacy content projection.
- `crates/chan-server/src/routes/graph.rs`: consume shared normalizers while retaining the visualization response.
- `crates/chan-server/src/control_socket.rs` and `crates/chan-shell/src/wire.rs`: live request and extended identity.
- `crates/chan-shell/src/cli.rs`: shared `cs search` flags and formatting.
- `crates/chan/src/lib.rs`: selector resolution, live/direct routing, aggregation, and the new `chan workspace` CLI.
- `crates/chan-llm/src/tools.rs`, `mcp.rs`, and `prompts.rs`: canonical tool replacement and generated schemas.

Do not create a generic graph framework or a new crate. The shared contract is one workspace-local operation with small normalization and traversal helpers.

Update `crates/chan-workspace/design.md`, `crates/chan-shell/design.md`, `crates/chan-llm/design.md`, and the relevant chan-server design section in the implementation commit because the exported interface, wire contract, and ownership boundary change.

## Test and Acceptance Plan

### Shared fixture

Build one workspace fixture containing:

- Nested directories at more than two levels.
- Markdown and text files.
- Rust, TypeScript, and at least one other report-recognized source language.
- Normal Markdown links, wiki links, anchored links, source-relative links, ancestor-relative links, and percent-encoded paths.
- A linked image, a linked source file, another linked regular resource, and an unresolved missing target.
- A two-file link cycle and a longer cycle reached at depth 2.
- Several tags, including one shared by multiple files and one exact/prefix collision.
- Unresolved mentions.
- Contact notes with title, basename, email, and aliases.
- A resolved basename mention, a resolved alias mention, and an ambiguous contact lookup.
- Files in several language and directory combinations.

Put a unique token in a source file body and assert content search cannot find it. Assert a path or report-metadata query can find the source entity.

### Core tests

Cover:

- Request normalization, defaults, hard caps, duplicate removal, and invalid empty requests.
- Selector parsing and canonicalization for every kind, including optional tag and mention sigils.
- Path escape, missing selector, unavailable language selector, and ambiguous contact errors.
- Content ranking, one-hit-per-file collapse, snippets, domain filtering, and stable tie breaks.
- Entity exact, prefix, substring, browse, kind, and stable-ID ordering.
- Contact path, title, basename, email, and alias matching.
- Source paths and linked media/resource discovery without source-content indexing.
- Depth 0, 1, and 2.
- File forward traversal.
- Tag, mention, and contact bidirectional traversal.
- Language one-hop traversal even when depth is greater than 1.
- Directory filesystem-depth traversal.
- Explicit direction overrides.
- Relationship filters.
- Cycles and deduplication.
- Meta-node closure.
- Complete containment spines for every surfaced file.
- Missing link omission.
- Node, edge, content, and entity truncation, including a file omitted when its complete spine cannot fit.
- Deterministic node and relationship ordering.
- Batched frontier query counts, proving query count scales by frontier chunks rather than visited nodes.
- BM25/hybrid effective-mode truth table with and without the embeddings feature.
- Reports disabled and report unavailable without a scan or config mutation.
- Index-not-ready partial results.

### Adapter tests

Cover:

- `POST /api/search/workspace` request and result serialization.
- `/api/search/content` response compatibility, scope, empty query, one-hit-per-file collapse, and mode.
- Control-socket result equality with a direct core call.
- Exact tenant selection when one process serves several workspaces.
- Live workspace routing without `WorkspaceLocked`.
- Direct-open-to-live-lock race and one retry.
- Served but unreachable tenant failure.
- Workspace selection by path, metadata key, display name, containing current directory, repeated selectors, and all workspaces.
- Ambiguous display names.
- Explicit selector ordering, sorted all-workspaces ordering, deduplication, and partial failures.
- Successful results retained with nonzero exit status on any partial failure.
- `cs search` parsing, Markdown sections, compact JSON, and pretty JSON.
- `chan workspace search` and `graph` parsing and aggregate output.
- Byte/shape stability for both `cs graph` `OpenGraph` request forms.
- Removal of `--scope`, `--target`, and full graph-dump behavior.
- chan-llm standard and MCP schema equality.
- chan-llm core JSON response equality.
- Removal of the four superseded tool names from schemas and prompts.

### Graph compatibility tests

Cover:

- Existing `/api/graph` fixtures before and after shared normalization.
- Encoded, relative, ancestor-relative, media, source, directory, and missing link targets.
- Contact basename and alias mention resolution.
- Contact node classification and unresolved mention nodes.
- Language-to-file relationships.
- Filesystem containment and root spines.
- Existing frontend depth, closure, visibility, and containment tests.
- The shared lens-parity golden fixture in Rust and Vitest.

### Verification commands

Run focused checks first:

```sh
cargo test -p chan-workspace workspace_search
cargo test -p chan-server search
cargo test -p chan-server graph
cargo test -p chan-shell
cargo test -p chan-llm
cargo test -p chan
npm run test -w @chan/workspace-app -- src/graph
```

Then run the repository gate, including default and no-default-feature Rust builds:

```sh
make pre-push
```

## Rejected Alternatives

### Fetch the visualization graph in CLI or MCP

Rejected because `/api/graph` is a whole-workspace visualization payload with server-owned enrichment and potentially large filesystem state. CLI and MCP calls need bounded, query-shaped results and must work without transferring or materializing the full plot.

### Keep one traversal implementation per surface

Rejected because the current drift is the bug. Direction, contact resolution, tag spelling, language edges, containment, limits, and retrieval mode must be decided once in chan-workspace.

### Open graph SQLite directly when a workspace is locked

Rejected because SQLite is only one part of the live state, and direct sidecar access can observe stale graph, search, or report data. Exact control-socket tenant routing reuses the server's held Workspace and watcher-maintained state.

### Add cross-workspace graph edges

Rejected because Workspace is the storage and trust boundary. Multi-workspace search is an outer fan-out whose results remain separated.

### Index source-code contents

Rejected because it changes index size, retrieval meaning, rebuild cost, and privacy expectations. This contract discovers source paths and report metadata only.

### Replace `GraphPanel` traversal immediately

Rejected because the SPA graph is proven behavior with richer interactive expansion and filtering. The new engine first proves parity through shared fixtures; a later focused migration can replace one lens at a time.

### Add a persistent normalized graph schema

Rejected because existing graph SQLite rows, report JSONL, and filesystem metadata contain the required truth. The change needs bounded query helpers, not another cache and migration surface.

## Assumptions and Defaults

- This document defines the implementation. The current task creates this design file only.
- The SPA graph plot, filesystem spine, depth behavior, and `cs graph [path]` UI opener are protected behavior.
- Graph expansion is explicit and bounded.
- An unqualified query searches content plus entity names and metadata.
- Source paths and languages are searchable; source contents are not indexed by this work.
- Query-free non-content domains provide bounded taxonomy browsing and replace `graph_tags` use cases.
- Every surfaced file has a complete containment spine to its workspace root.
- Cross-workspace search returns independent per-workspace graphs and never creates cross-workspace relationships.
- Read-only queries never mutate registration, feature flags, report state, or index state.
- No new persistent graph schema or runtime dependency is required.
