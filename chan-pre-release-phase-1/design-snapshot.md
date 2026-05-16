# Phase 1 Design Snapshot

This is the Phase 1 architecture snapshot for Chan's first public
engineering release. It is a current-state contract, not a changelog.

## Boundaries

- `chan-drive` owns drive-root safety, filesystem traversal, search,
  graph, reports, and per-drive index state.
- `chan-server` owns HTTP/WebSocket contracts, app-level preferences,
  the background indexer coordinator, and the embedded frontend.
- `chan` owns CLI parsing and command dispatch. It calls chan-drive and
  chan-server APIs; it does not host HTTP handlers or direct LLM logic.
- `web/` owns presentation and browser interaction. It should not infer
  drive safety rules locally when a server/core helper can answer.

## First-Release Policy

This release is the first canonical public version of Chan. Code should
not preserve migrations from internal development snapshots unless the
state is still produced by the current release surface.

Keep:

- Fresh-install initialization.
- Default filling for partial current config files.
- Defensive handling of missing/corrupt app state.
- Browser/session compatibility that is only local UI state and does not
  imply persisted product data support.

Remove or re-scope:

- One-shot index/data backfills for internal pre-release schemas.
- Comments that explain old development versions instead of current
  behavior.
- Server routes retained only for already-removed UI/API clients.

## Search And Reports

The search index lifecycle is owned by the background indexer in
`chan-server`, backed by chan-drive. The web dashboard should consume:

- `/api/index/status`
- `/api/index/rebuild`
- `/api/report/prefix?path=`
- `/api/report/file?path=...`

Whole-drive SLOC and language breakdown already exist through
`/api/report/prefix?path=`. `language:<name>` search should use report
data and return files, not invent a parallel language classifier.

## Graphs

There are two graph concepts:

- Semantic graph: current `/api/graph`, with file/tag/mention nodes and
  link/tag/mention edges extracted from content.
- Filesystem graph: new Phase 1 surface for directories, files,
  symlinks, hardlinks, and broken symlink ghosts.

Do not overload the semantic graph response with filesystem-only concepts
unless the response becomes an explicit tagged union. The preferred shape
is a separate route/core helper that returns filesystem nodes and relation
edges by scope/depth.

## Assistant UI

Assistant stream contracts are already carried over WebSocket as
`llm.status`, `llm.activity`, `llm.user_request`, deltas, tool calls, and
tool results. Phase 1 assistant work is presentation only:

- maintain scroll bottom margin while streaming and resizing
- let long bubbles use available width
- converge on the orange-dot thinking badge

No backend stream contract change is required for `webdev-3`.

## CLI Parity

New CLI commands should expose the same durable concepts as the web UI:

- config get/set for app settings
- graph queries by scope
- status for drive/index/graph/report health

The binary should keep output scriptable and route writes through existing
config/store helpers.

## Audit Findings

Real cleanup candidate:

- `crates/chan-server/src/indexer.rs` has a one-shot pre-v3 contact email
  backfill. That is internal-version migration behavior and belongs in
  `rustacean-1`.

Mostly not release blockers:

- `legacy` references in editor code describe internal component
  compatibility while the editor rewrite is in progress.
- `schema_version` in assistant/group session payloads is browser UI
  state, not public drive data.
- `chan-error-v2` comment documents an external wrapper contract version,
  not a migration path.
- `@codemirror/legacy-modes` is an upstream package name.

## Open Risks

- Filesystem graph support may belong in chan-core. If rustacean-2 needs
  chan-core edits, split that explicitly before frontend implementation
  depends on an unstable local route.
- Hardlink identity is platform/filesystem-sensitive. Tests should degrade
  cleanly where hardlinks are unavailable.
- Search dashboard can race rebuild status if it polls report and index
  independently; prefer a consolidated server snapshot if this becomes
  visible.
