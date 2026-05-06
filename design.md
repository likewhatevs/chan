# chan-core - design

This document describes the design of `chan-core`, the low-level
Rust library extracted from the chan markdown editor. It owns the
per-machine registry of known drives, exposes a path-based
sandboxed filesystem API rooted at each drive, and wraps the per-
drive search index and graph database.

This is the canonical design reference for the crate. Update it in
the same commit as any change that affects the on-disk layout, the
public API shape, or the locking model.

## Scope and non-scope

In scope:

  - Filesystem primitives (read / write / stat / list / rename /
    remove / list_tree) rooted at a drive.
  - Drive registry persisted to `~/.chan/config.toml` (or the OS
    sandbox equivalent).
  - Per-drive search index (tantivy, BM25 only at v1).
  - Per-drive graph database (sqlite, single-writer with
    `Mutex<Connection>`).
  - Filesystem watcher (callback-based, drive-scoped).
  - Cross-process advisory writer lock (fs4 / flock / LockFileEx).
  - Path-traversal sandboxing and editable-text whitelist.
  - Atomic writes for every chan-core-managed file.

Out of scope:

  - HTTP, WebSocket, frontend bundle. Those live in
    `chan-writer/chan`.
  - LLM tool calls, API key storage, assistant chat history I/O.
    chan-core only allocates the per-drive directory; the consumer
    decides the schema.
  - Editor preferences (fonts, theme, keybindings, attachments dir).
    Those are app-level and live in a config file the consumer owns.
  - User authentication, multi-user collaboration, cloud sync of
    chan-core state. Single-user, single-machine, local-first.
  - Cross-drive linking. One drive at a time; explicit non-goal.

## Architecture overview

```
                 +------------------------+
                 |   Library (per-mach.)  |
                 |   ~/.chan/config.toml  |
                 |   Mutex<Registry>      |
                 +-----------+------------+
                             |
                  open_drive |
                             v
              +-----------------------------+
              |     Drive (per directory)   |
              |     DriveLock held          |
              |  +------+------+----------+ |
              |  | FS   | Idx  | Graph    | |
              |  | ops  | tan- | sqlite   | |
              |  | safe | tivy | (Mutex)  | |
              |  +------+------+----------+ |
              |           Watcher           |
              +-----------------------------+
```

  - `Library` is a per-machine handle. Apps construct one at
    startup and keep it alive. It owns the registry and the
    config-file path.
  - `Drive` is one registered directory. Holds the writer lock for
    its lifetime; cheap reads (search query, graph traversal) do
    not contend. Lazily initializes search and graph state on
    first use via `OnceLock`.
  - `WatchHandle` is an opaque return value; drop to stop watching.
    The underlying `notify::RecommendedWatcher` runs on its own
    thread and dispatches into the consumer's `WatchCallback`.

## On-disk layout

### Per-machine state

```
~/.chan/                          (config_dir on desktop)
  config.toml                     drive registry + default drive root

$DATA_DIR/chan/                   (state_dir; persistent)
  sessions/<key>/                 per-drive opaque session blobs
  assistant/<key>/                per-drive assistant chat history
  graph/<key>/graph.sqlite        per-drive graph DB
  locks/<key>/writer.lock         per-drive cross-process lock

$CACHE_DIR/chan/                  (cache_dir; rebuildable)
  index/<key>/                    per-drive tantivy segments
```

`<key>` is `sha256(canonical_path)[..16]` as hex. Renames invalidate
the keys; rebuilds are cheap. There is no rename-tracking.

Per platform:

  Platform    config_dir          state_dir                cache_dir
  ----------  ------------------  -----------------------  ------------------
  macOS       ~/.chan             ~/Library/AppSupport/    ~/Library/Caches/
                                  chan                     chan
  Linux       ~/.chan             $XDG_DATA_HOME/chan      $XDG_CACHE_HOME/chan
  Windows     C:\Users\u\.chan    %APPDATA%\chan           %LOCALAPPDATA%\chan
  iOS         (see below)         $sandbox/Library/        $sandbox/Library/
                                  Application Support/     Caches/chan
                                  chan
  Android     (see below)         $sandbox/files/chan      $sandbox/cache/chan

iOS and Android collapse `config_dir` onto `state_dir` because the
home dir inside the app sandbox is not user-writable. The "brand-
visible" argument for `~/.chan/` does not apply on mobile (the user
cannot browse the sandbox anyway), so the registry lives alongside
the rest of the per-app state.

### Drive contents

chan-core stores ZERO files inside the user's drive directory. The
drive is purely user content (markdown, attachments). This makes it
safe to drop a drive inside an existing git repo, an iCloud /
Google Drive / Dropbox folder, or anywhere else. A stray `.chan/`
left over from an older install or created by a third-party tool
is filtered out by `walk_drive` and `dispatch`; chan-core never
emits events for it or includes it in `list_tree`.

## Public API surface

### Library

```rust
Library::open() -> Result<Self>
Library::open_at(config_path: PathBuf) -> Result<Self>

Library::list_drives() -> Vec<KnownDrive>
Library::default_drive_root() -> Option<PathBuf>
Library::set_default_drive_root(root: Option<PathBuf>) -> Result<()>
Library::effective_default_drive_root() -> PathBuf

Library::register_drive(root: &Path, name: Option<String>) -> Result<KnownDrive>
Library::unregister_drive(root: &Path) -> Result<bool>
Library::rename_drive(root: &Path, name: Option<String>) -> Result<bool>

Library::open_drive(root: &Path) -> Result<Arc<Drive>>
```

Registration is idempotent: re-registering an existing drive only
updates `last_opened` and never clobbers a user-set name. Rename is
the only path that overwrites the name.

### Drive (filesystem primitives)

```rust
Drive::root() -> &Path
Drive::name() -> Option<&str>
Drive::paths() -> &DrivePaths

Drive::read(rel: &str) -> Result<Vec<u8>>
Drive::read_text(rel: &str) -> Result<String>          // gated
Drive::write_text(rel: &str, content: &str) -> Result<()>  // gated
Drive::write_bytes(rel: &str, content: &[u8]) -> Result<()>

Drive::exists(rel: &str) -> bool
Drive::stat(rel: &str) -> Result<FileStat>
Drive::list(rel: &str) -> Result<Vec<DirEntry>>
Drive::list_tree() -> Result<Vec<TreeEntry>>
Drive::create_dir(rel: &str) -> Result<()>
Drive::remove(rel: &str) -> Result<()>                 // file or empty dir
Drive::rename(from: &str, to: &str) -> Result<()>
```

All `rel` arguments are POSIX-style relative paths. Path traversal
(`..`, absolute roots, Windows drive prefixes) is rejected via
`fs_ops::resolve_safe`. The editable-text gate (`.md`, `.txt`)
applies to `read_text` / `write_text` only; binary I/O routes
around it because attachments and future media browsing need it.

`remove` will not recursively delete a non-empty directory. The
caller walks and deletes explicitly. This is a foot-gun guard:
the editor never has reason to recursive-rm, and the LLM tool
sandbox should not be able to either.

### Drive (search and graph)

```rust
Drive::search(query: &str, opts: &SearchOpts) -> Result<SearchResults>
Drive::reindex() -> Result<IndexStats>

Drive::graph() -> Result<&GraphView>
GraphView::neighbors(rel: &str) -> Result<Vec<Edge>>
GraphView::backlinks(rel: &str) -> Result<Vec<Edge>>
GraphView::tags() -> Result<Vec<Tag>>
GraphView::files_with_tag(tag: &str) -> Result<Vec<String>>
GraphView::replace_file(rel, mtime, outgoing, headings) -> Result<()>
GraphView::forget_file(rel: &str) -> Result<()>
```

Search is BM25-only at v1. The `SearchMode` enum reserves a
`Hybrid` variant for the future when an embedder is wired in
(fastembed-rs on desktop / Linux server, CoreML on iOS, NNAPI on
Android). On builds where the embedder is unavailable, hybrid
falls back to BM25 silently.

`reindex` is synchronous and blocking. It runs on the calling
thread; the caller decides whether to spawn a worker. This keeps
the API uniffi-clean and avoids leaking an async runtime through
the FFI boundary.

The graph indexer is a separate concern from the search indexer:
the indexer that updates them lives at a higher layer (the file-
system watcher feeds both). chan-core exposes `replace_file` and
`forget_file` as the write-side primitives; the indexer driving
them is an app-level component (or a future companion crate).

### Drive (watch)

```rust
Drive::watch(cb: Arc<dyn WatchCallback>) -> Result<WatchHandle>
trait WatchCallback: Send + Sync {
    fn on_event(&self, event: WatchEvent);
}
```

Callback-based on purpose. The Swift / Kotlin client implements
the callback by passing an `Arc<dyn WatchCallback>` (uniffi
generates a wrapper around a foreign object). No closures cross
the FFI.

## Locking model

```
register / open               read                          write
-----------------             -----------                   ------
Library::register_drive       Drive::read*                  Drive::write_*
  Mutex<Registry> lock          fs_ops::resolve_safe          fs_ops::atomic_write
  atomic_write registry         (no lock)                     (no lock)

Drive::open                   Drive::search                 Drive::reindex
  fs4::try_lock_exclusive       tantivy reader (no lock)      tantivy writer
  on writer.lock                                              (held by lock)

                              Drive::graph
                                Mutex<Connection> lock
                                (intra-process)
```

Two distinct concurrency primitives:

  - `DriveLock` (cross-process): held for the lifetime of `Drive`.
    A second process opening the same drive errors immediately
    with `ChanError::DriveLocked`; we do NOT block. Callers
    handle the error explicitly (CLI prints a message and exits;
    desktop app falls back to opening another drive).
  - `Mutex<Registry>` and `Mutex<Connection>` (intra-process):
    serialize concurrent calls from the same process. Cheap.

Reading does not take the cross-process lock. tantivy is multi-
reader-safe by design; sqlite's WAL mode (set via
`PRAGMA journal_mode = WAL` in a future revision) lets concurrent
readers proceed alongside the writer.

## Error model

One umbrella `ChanError` enum so the Swift / Kotlin error type is
a single tagged union. All foreign errors (io, toml, rusqlite,
notify) collapse into `ChanError::Io`, `::ConfigDecode`,
`::Graph`, `::Watch` with their `Display` text preserved.

Variants intentionally do not carry rich nested types: uniffi can
encode an enum with primitive payloads; nested error chains do
not round-trip cleanly across the FFI.

## Schema versioning

Graph DB uses `PRAGMA user_version`. Migrations are idempotent
applied on every `GraphView::open`. Current version: 1.

Search index uses an on-disk `schema_version` field (TBD; lands
with the tantivy wiring). Mismatched versions trigger a full
rebuild on next open. Model swaps (when hybrid lands) force a
rebuild because embedding dimensions differ.

## What's wired vs. what's still ahead

Wired and tested:

  - **Markdown parser** (`src/markdown/`): native-only port of
    `chan-shared`'s frontmatter, ATX heading, link, and reference-
    token extractors. Powers both the search title/body extraction
    and the graph indexer. The wasm-only smart-node serialization
    stays in the chan repo as an editor concern.
  - **Search index** (`src/search.rs`): tantivy 0.24, BM25 only,
    one document per file at v1. Schema versioned via
    `<index_dir>/.schema_version`; mismatches wipe and rebuild on
    next open. `Drive::search`, `Drive::index_file`,
    `Drive::reindex`, `Drive::forget_file` all functional.
  - **Graph reads + writes** (`src/graph.rs`): `neighbors`,
    `backlinks`, `tags`, `files_with_tag`, `files`, `headings_of`
    return real data. `replace_file` inserts outgoing edges
    (links + tag/mention tokens) and headings with computed
    anchors. `clear` wipes everything for a full rebuild.

Still ahead:

  - **Per-section search chunking**: hits return per-file
    snippets. Adding section-level chunking would let
    `Snippet::heading_path` carry the breadcrumb to the matched
    section and would tighten relevance for long files. Bumps
    `SCHEMA_VERSION` when it lands, triggers a forced rebuild.
  - **Wiki-link resolution**: graph stores link targets as
    written (`recipes/pasta`, no extension). A resolver step
    that maps each target to a real file path (with `.md`
    extension lookup, prefix-match, etc.) lives at the consumer
    layer for now. Could move into chan-core if every consumer
    needs the same logic.
  - **Watcher consumer**: `Drive::watch` is wired and the
    watcher filters drive-internal noise, but no built-in
    consumer feeds events into `index_file` / `forget_file`.
    Apps run their own loop.
  - **Hybrid (BM25 + dense) search**: `SearchMode::Hybrid` is
    a placeholder that falls through to BM25. Wiring an
    embedder (fastembed-rs on desktop, CoreML on iOS, NNAPI on
    Android) is gated behind a future `embeddings` feature.

## Future API extensions (sketch, not committed)

  - `Drive::graph_indexer()`: returns a structured indexer that
    accepts `WatchEvent`s and calls `replace_file` / `forget_file`
    accordingly. Apps run it on a worker thread pulling from the
    watcher channel.
  - Remote-backed `Drive` impl: a future trait split could let a
    thin client call `read` / `write` / `search` against an HTTP
    endpoint while the server runs the real chan-core. Not
    designed for now; the API surface is shaped to allow it
    later.
  - Background reindex job handle: if synchronous reindex turns
    out painful in practice, add a `JobHandle` returned by
    `reindex_async()` that the caller polls. Defer until needed.
