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
  - Per-drive blob storage for opaque host JSON: window/pane
    sessions and assistant chat history. chan-core stores bytes
    keyed by a flat identifier; the schema is the host's choice.
    Native shells (iOS / Android) link these via uniffi and share
    persistence semantics with the chan-server desktop app
    without reimplementing the atomic-write story per platform.

Out of scope:

  - HTTP, WebSocket, frontend bundle. Those live in
    `chan-writer/chan`.
  - LLM tool calls, API key storage, prompt content. The blob
    storage above gives a place to PUT chat history; chan-llm
    decides the schema and chan-server (or a native shell) is
    the orchestrator that calls put_assistant.
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
  tokens/<key>/                   per-drive bearer-token store
                                  (chan-core allocates the dir; the
                                  app, e.g. chan-server, owns the
                                  contents)
  trash/<key>/<id>/               per-drive Trash. Each entry holds
    payload | payload/            the moved file or directory and
    meta.json                     a JSON sidecar (original_path,
                                  deleted_at, is_dir, size). meta is
                                  written last; sweep treats meta-
                                  less entries as crash leftovers.

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
Drive::read_text_with_stat(rel: &str) -> Result<(String, FileStat)>  // gated
Drive::write_text(rel: &str, content: &str) -> Result<()>  // gated
Drive::write_text_if_unchanged(rel: &str,                    // gated, CAS
    expected_mtime: Option<i64>,
    content: &str,
) -> Result<()>
Drive::write_bytes(rel: &str, content: &[u8]) -> Result<()>

Drive::exists(rel: &str) -> bool
Drive::stat(rel: &str) -> Result<FileStat>
Drive::list(rel: &str) -> Result<Vec<DirEntry>>
Drive::list_tree() -> Result<Vec<TreeEntry>>
Drive::create_dir(rel: &str) -> Result<()>
Drive::remove(rel: &str) -> Result<()>                 // soft-delete to trash
Drive::rename(from: &str, to: &str) -> Result<()>

Drive::trash_list() -> Result<Vec<TrashEntry>>
Drive::trash_restore(id: &str) -> Result<()>
Drive::trash_purge(id: &str) -> Result<()>
Drive::trash_empty() -> Result<()>

Drive::put_session(key: &str, content: &[u8]) -> Result<()>
Drive::get_session(key: &str) -> Result<Option<Vec<u8>>>
Drive::list_sessions() -> Result<Vec<String>>
Drive::delete_session(key: &str) -> Result<()>

Drive::put_assistant(key: &str, content: &[u8]) -> Result<()>
Drive::get_assistant(key: &str) -> Result<Option<Vec<u8>>>
Drive::list_assistant() -> Result<Vec<String>>
Drive::delete_assistant(key: &str) -> Result<()>
Drive::clear_assistant() -> Result<()>
```

All `rel` arguments are POSIX-style relative paths. Path traversal
(`..`, absolute roots, Windows drive prefixes) is rejected via
`fs_ops::resolve_safe`, then `fs_ops::resolve_safe_strict` adds a
canonical-form check that the deepest existing ancestor stays
under the canonical drive root (catches mid-path symlinks
escaping the sandbox). The editable-text gate (`.md`, `.txt`)
applies to `read_text` / `read_text_with_stat` / `write_text` /
`write_text_if_unchanged` only; binary I/O routes around it
because attachments and future media browsing need it.

`read_text_with_stat` and `write_text_if_unchanged` are the
optimistic-concurrency pair the editor uses to detect external
edits. The editor reads `(content, stat)`, the user types, and
the save round-trips through `write_text_if_unchanged(rel,
stat.mtime, new_content)`. If another process (terminal, sync
daemon, second pane) has since modified the file, the write
fails with `WriteConflict { current_mtime }` and the editor
prompts to reload, merge, or overwrite. `write_text` (no CAS)
remains for chan-core's own reindex helpers, bulk imports, and
LLM-tool calls where last-write-wins is the intent. Residual
race: between the mtime check and the atomic rename a foreign
write can land; the watcher event for the foreign change fires
on the next dispatch and the editor re-prompts.

`remove` is a soft-delete: it moves the entry into the per-drive
Trash (see below). Recursive directory removal is allowed because
the operation is reversible; the foot-gun guard against accidental
recursive-rm is satisfied by the restore path. Symlinks, FIFOs,
sockets, and char/block devices are rejected with `SpecialFile`;
the trash format only models regular files and directories, and
chan-core never creates the other types itself.

### Trash

The Trash gives the editor and the LLM tool sandbox a safe
delete: every `remove` is reversible until either the user
explicitly purges or the retention window elapses.

```rust
pub struct TrashEntry {
    pub id: String,            // opaque, monotone
    pub original_path: String, // POSIX rel from drive root
    pub deleted_at: i64,       // unix seconds
    pub is_dir: bool,
    pub size: u64,             // file len, or summed for dirs
}
```

  - **Location**: `state_dir/trash/<key>/<id>/{payload[/], meta.json}`.
    Trash lives outside the user's drive directory because chan-
    core stores zero state inside the drive. Trade-off: the trash
    does not sync via iCloud / Dropbox / git. Acceptable; trash is
    per-machine recovery, not collaboration. A user who relocates
    a drive to a new path also leaves their old trash behind
    (drive_key is sha256 of the canonical path).
  - **Atomicity**: same-fs path is one atomic `rename` from drive
    root into the trash payload. Cross-fs path falls back to
    `copy + remove`, writing `meta.json` AFTER the copy and BEFORE
    the source removal, so a remove failure leaves a complete
    trash entry plus a partial source the user can clean up.
  - **Restore conflicts**: refused with `TrashOccupied`. The
    caller renames the live entry first, or `trash_purge` to give
    up. We never silently overwrite live content.
  - **Auto-expiration**: lazy GC. `Drive::open` and every `trash_*`
    call sweep entries older than `TRASH_RETENTION_SECS` (30 days
    at v1). No background thread; matches the codebase's sync-only
    rule. Promote to a `Library` setting later if users want to
    tune it.
  - **Crash recovery**: a half-written entry has no `meta.json`.
    Sweep treats meta-less entries as junk and reclaims them.

What's NOT in v1 (deliberately):

  - Cross-drive trash. Each drive has its own.
  - Sync to cloud storage (deliberate; trash is local-only state).
  - Background timer (lazy GC is enough for an editor that opens
    drives sporadically).
  - Configurable retention. Hardcoded 30 days; revisit if needed.

### Symlink and special-file policy

The Drive entry points enforce three rules so the layer never
accidentally hangs on, follows, or mutates a non-regular file:

1. **Mid-path symlinks**: rejected when their canonical target
   leaves the drive. In-drive symlinks (`alias -> ./real`) pass
   the path-resolve leg.
2. **Final-component symlinks**: rejected by every read / write
   op. Atomic rename would otherwise replace the link with a
   regular file (silently breaking the user's intentional
   alias), and reads would traverse the link off-disk. Users
   who want to write through a symlink delete the link first.
3. **FIFOs, sockets, char/block devices**: rejected by every op
   via the `lstat`-based gate. These types can't appear in a
   note workflow; if they do, it's either a misconfiguration or
   abuse of the read/write API. Without the gate, opening a
   FIFO blocks waiting for a writer; opening `/dev/zero` never
   returns; opening a device sends ioctl-shaped reads.

Walker invariants:

- `walkdir::follow_links(false)` and `same_file_system(true)` so
  symlink loops can't occur and a misregistered drive that
  spans onto a network mount won't drag the indexer into it.
- Iteration drops non-regular non-dir entries, so the UI tree
  and the indexer only ever see real files and dirs.

What's NOT closed today:

- **TOCTOU between resolve and open**: a path that passes the
  strict resolve could be swapped for a symlink before the
  actual open syscall. Closing this requires `openat2(2)` with
  `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` (Linux 5.6+) or per-
  component `O_NOFOLLOW` on platforms without openat2. Tracked
  as future hardening; current threat model (single user,
  single machine, no concurrent attacker) makes this acceptable.

### Drive (search and graph)

```rust
Drive::search(query: &str, opts: &SearchOpts) -> Result<SearchResults>
Drive::reindex() -> Result<IndexStats>
Drive::link_targets(q: &str, limit: u32) -> Result<Vec<LinkTarget>>

Drive::graph() -> Result<&GraphView>
GraphView::neighbors(rel: &str) -> Result<Vec<Edge>>
GraphView::backlinks(rel: &str) -> Result<Vec<Edge>>
GraphView::tags() -> Result<Vec<Tag>>
GraphView::files_with_tag(tag: &str) -> Result<Vec<String>>
GraphView::replace_file(rel, title, mtime, outgoing, headings) -> Result<()>
GraphView::forget_file(rel: &str) -> Result<()>
GraphView::link_targets(q: &str, limit: u32) -> Result<Vec<LinkTarget>>
```

#### Link autocomplete (`[[`)

`link_targets` drives the editor's `[[` typeahead: the user types a
fragment and gets back files plus headings to anchor a wiki link to.
The graph DB is the source of truth (`nodes` for files, `headings`
for in-file anchors); BM25 over filename and heading text in the
search index serves a parallel purpose for free-text search but is
not used for the picker.

```rust
pub enum LinkTargetKind { File, Heading }

pub struct LinkTarget {
    pub kind: LinkTargetKind,
    pub path: String,            // rel path of the file (both kinds)
    pub title: Option<String>,   // file title; None for headings
    pub heading: Option<String>, // heading text; None for files
    pub anchor: Option<String>,  // heading anchor; None for files
    pub level: Option<u8>,       // heading depth 1..=6; None for files
    pub mtime: Option<i64>,      // file mtime; None for headings
}
```

  - **Empty `q`**: most-recently-edited files first, up to `limit`.
    Useful as the picker's initial state before any keystroke.
  - **Non-empty `q`**: four-tier ASCII case-insensitive match.

      rank 1  basename starts with q   ("carb" -> "carbonara.md")
      rank 2  basename contains q      ("bona" -> "carbonara.md")
      rank 3  title contains q         (h1 / frontmatter title hit)
      rank 4  heading text contains q  (in-file anchor target)

    Within a rank, files sort by `mtime DESC NULLS LAST, rel_path
    ASC`; headings sort by `rel_path, ord`. Heading hits are
    capped at `limit / 2` so a single TOC-heavy file cannot drown
    out file matches.

  - **Wildcard escaping**: `%`, `_`, and `\` in `q` are escaped
    against SQLite's LIKE engine so a filename "100%off.md" is not
    matched by a raw `%` query.
  - **Case folding**: ASCII only. SQLite's `LOWER` does not fold
    Unicode without ICU; non-ASCII queries match case-sensitively.
    Acceptable for v1; revisit when a Unicode-aware backend
    becomes a priority.

After picking a file, the editor calls `GraphView::headings_of(rel)`
to populate the `[[file#` second phase from that file alone.

##### Free-text search and the new index fields

The search index gained `filename` (basename stem, tokenized) and
`headings` (newline-joined heading texts, tokenized) at
`SCHEMA_VERSION = 3`. They make `Drive::search("carbonara")` find
both `recipes/carbonara.md` (filename match) and any file whose
section heading mentions it, even when the body never does. The
schema version bump triggers an automatic wipe + rebuild on next
open; user data is unaffected.

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
