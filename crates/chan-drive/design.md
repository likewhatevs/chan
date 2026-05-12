# chan-drive design

Canonical design reference for `chan-drive`. Update in the same
commit as any change that affects the on-disk layout, the public
API shape, the locking model, or the schema versions.

## 1. Problem and scope

`chan-drive` is the local-first storage layer for a single-user
markdown editor. It owns the per-machine registry of known drives,
exposes a path-based sandboxed filesystem API rooted at each drive,
and wraps the per-drive search index and link graph. The same
crate backs the `chan` CLI today and is shaped for native iOS /
Android shells via uniffi later.

In scope:

  - Filesystem primitives (`read` / `write` / `stat` / `list` /
    `rename` / `remove` / `list_tree`) rooted at a drive.
  - Drive registry persisted to `~/.chan/config.toml` (or the OS
    sandbox equivalent on iOS / Android).
  - Per-drive search index (tantivy 0.24, BM25; optional dense
    via candle + BGE-small for hybrid).
  - Per-drive graph database (sqlite, single writer, r2d2 pool
    for readers).
  - Filesystem watcher (callback-based, drive-scoped, drops
    `.chan/` and `.git/` noise).
  - Cross-process advisory writer lock (fs4 / flock / LockFileEx).
  - Path-traversal sandboxing and editable-text whitelist.
  - Atomic writes for every chan-drive-managed file.
  - Per-drive blob storage for opaque host JSON: pane sessions
    and assistant chat history. chan-drive stores bytes keyed
    by a flat identifier; the schema is the host's choice.

Out of scope:

  - HTTP, WebSocket, frontend bundle. Those live in
    `chan-writer/chan`.
  - LLM tool calls, API key storage, prompt content. The blob
    storage above gives a place to PUT chat history; chan-llm
    owns the schema and the HTTP / FFI orchestrator calls
    `put_assistant`.
  - Editor preferences (fonts, theme, keybindings, attachments
    dir). App-level; the consumer owns its own config file.
  - User authentication, multi-user collaboration, cloud sync of
    chan-drive state. Single-user, single-machine, local-first.
  - Cross-drive linking. One drive at a time; explicit non-goal.

## 2. Architecture overview

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
              |  | safe | tivy | (Mutex   | |
              |  |      |+cand.| + pool)  | |
              |  +------+------+----------+ |
              |           Watcher           |
              +-----------------------------+
```

  - `Library` is a per-machine handle. Apps construct one at
    startup and keep it alive. It owns the registry and the
    config-file path.
  - `Drive` is one registered directory. Holds the writer lock
    for its lifetime; cheap reads (search query, graph
    traversal) do not contend. Lazily initializes search and
    graph state on first use via `OnceLock`.
  - `WatchHandle` is an opaque return value; drop to stop
    watching. The underlying `notify::RecommendedWatcher` runs
    on its own thread and dispatches into the consumer's
    `WatchCallback`.

## 3. Components

### Library

Per-machine singleton-in-practice. Owns the `Registry`
(`Mutex<Registry>` intra-process), the config-file path, and the
default drive root. `open_drive` resolves the canonical path,
hashes it into a 16-hex `<key>`, and constructs a `Drive` that
holds the cross-process writer lock for its lifetime.

`reset_drive` wipes per-drive chan-managed state (search index,
graph DB, session and assistant blobs, app tokens). It never
touches the user's notes tree. The trash is preserved (it holds
user-deleted files, recoverable user data, not chan-managed
cache). The lock dir is preserved (cross-process coordination, no
data). `ResetMode::Everything` additionally drops the registry
entry so the next `open_drive` against the path treats it as a
fresh, never-seen drive.

Precondition: caller must drop any open `Arc<Drive>` for the
target root first. `reset_drive` acquires the writer lock to
verify exclusive access; if any process (including the caller)
holds it, the call fails with `DriveLocked`. On Unix this is
defense-in-depth (open files survive unlink); on Windows it is
load-bearing because removing files-in-use fails. Skeleton
recreation happens lazily on the next `open_drive` plus first
`index()` / `graph()` access; no explicit init step.

Registration is idempotent: re-registering an existing drive only
updates `last_opened` and never clobbers a user-set name. Rename
is the only path that overwrites the name.

### Drive

One registered directory. All `rel` arguments are POSIX-style
relative paths. Path traversal (`..`, absolute roots, Windows
drive prefixes) is rejected via `fs_ops::resolve_safe`, then
`fs_ops::resolve_safe_strict` adds a canonical-form check that
the deepest existing ancestor stays under the canonical drive
root (catches mid-path symlinks escaping the sandbox).

The editable-text gate (`fs_ops::is_editable_text`, `.md` /
`.txt`) applies to `read_text` / `read_text_with_stat` /
`write_text` / `write_text_if_unchanged` only; binary I/O routes
around it because attachments and future media browsing need it.

#### Supported file types

A drive is the user's directory tree, untouched. Adding a drive
registers a root and walks everything under it (skipping `.git/`
and `.chan/`); chan-drive never restricts what can sit on disk.
What changes by class is how each file is handled by the API.
`fs_ops::classify(rel)` is the single predicate; consumers (this
crate, chan-server, the editor) should switch on `FileClass`
rather than re-deriving extension rules.

  Class           Extensions
  --------------  ---------------------------------------------
  EditableText    .md, .txt
  Image           .png, .jpg, .jpeg, .gif, .webp, .svg, .avif
  Pdf             .pdf
  Other           everything else

Behaviour by class:

  - **EditableText**: full read / write through `read_text` and
    `write_text`. Indexed by tantivy (BM25 + dense vectors when
    embeddings are enabled). Parsed for graph edges, headings,
    tags, mentions. The CAS pair `read_text_with_stat` +
    `write_text_if_unchanged` is available for editor-style
    optimistic concurrency.
  - **Image**: opaque bytes via `read` / `write_bytes`. Not
    indexed and not a graph node, but markdown embeds
    (`![alt](img.png)`) emit edges pointing at the image so
    `backlinks("media/foo.png")` returns the notes that embed
    it. The editor renders previews inline; rename / remove are
    supported.
  - **Pdf**: same I/O contract as Image. Held as a distinct
    class because the editor / inspector pane will render PDFs
    through a dedicated viewer (browser PDF.js today,
    inspector-side preview later).
  - **Other**: opaque bytes via `read` / `write_bytes`. Walkable,
    visible in `list_tree`, renameable, removeable into Trash.
    Not indexed and not a graph node. No assumptions about
    encoding or shape.

`rename` and `remove` operate on every class. Link rewriting in
`rename_with_link_rewrite` only touches `EditableText` bodies
(images and other binaries have no in-body links to rewrite); the
graph edge `dst` gets updated regardless of target class.

Extension matching is ASCII case-insensitive. Files with no
extension collapse to `Other`.

`read_text_with_stat` and `write_text_if_unchanged` are the
optimistic-concurrency pair the editor uses to detect external
edits. The editor reads `(content, stat)`, the user types, and
the save round-trips through `write_text_if_unchanged(rel,
Some(stat.mtime), new_content)`. If another process (terminal,
sync daemon, second pane) has since modified the file, the write
fails with `WriteConflict { current_mtime }` and the editor
prompts to reload, merge, or overwrite. `write_text` (no CAS)
remains for chan-drive's own reindex helpers, bulk imports, and
LLM-tool calls where last-write-wins is the intent. Residual
race: between the mtime check and the atomic rename a foreign
write can land; the watcher event for the foreign change fires
on the next dispatch and the editor re-prompts.

`remove` is a soft-delete: it moves the entry into the per-drive
Trash. Recursive directory removal is allowed because the
operation is reversible; the foot-gun guard against accidental
recursive-rm is satisfied by the restore path. Symlinks, FIFOs,
sockets, and char/block devices are rejected with `SpecialFile`;
the trash format only models regular files and directories, and
chan-drive never creates the other types itself.

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
    drive stores zero state inside the drive. Trade-off: the
    trash does not sync via iCloud / Dropbox / git. Acceptable;
    trash is per-machine recovery, not collaboration. A user who
    relocates a drive to a new path also leaves their old trash
    behind (`<key>` is sha256 of the canonical path).
  - **Atomicity**: same-fs path is one atomic `rename` from
    drive root into the trash payload. Cross-fs path falls back
    to `copy + remove`, writing `meta.json` AFTER the copy and
    BEFORE the source removal, so a remove failure leaves a
    complete trash entry plus a partial source the user can
    clean up.
  - **Restore conflicts**: refused with `TrashOccupied`. The
    caller renames the live entry first, or `trash_purge` to
    give up. We never silently overwrite live content.
  - **Auto-expiration**: lazy GC. `Drive::open` and every
    `trash_*` call sweep entries older than
    `TRASH_RETENTION_SECS` (30 days, `30 * 24 * 60 * 60`). No
    background thread; matches the codebase's sync-only rule.
    Promote to a `Library` setting later if users want to tune
    it.
  - **Crash recovery**: a half-written entry has no `meta.json`.
    Sweep treats meta-less entries as junk and reclaims them.

What's NOT in v1 (deliberately):

  - Cross-drive trash. Each drive has its own.
  - Sync to cloud storage (deliberate; trash is local-only state).
  - Background timer (lazy GC is enough for an editor that opens
    drives sporadically).
  - Configurable retention. Hardcoded 30 days; revisit if needed.

### Search index

`tantivy` 0.24 backs the per-drive index at
`<cache_dir>/index/<key>/`. BM25 over `path`, `filename`,
`headings`, and `body`. Schema version lives in
`<index_dir>/config.toml` as the `schema_version` field of the
`IndexConfig` struct (alongside the embedding model id and the
chunking strategy). Mismatched versions trigger a wipe and full
rebuild on next open; user data is unaffected.

With the `embeddings` feature on (default), the index also stores
per-chunk dense vectors via candle + BGE-small. `SearchMode::Hybrid`
runs BM25 and dense in parallel, then fuses with reciprocal-rank
fusion. On builds where the embedder is unavailable
(`--no-default-features`, currently iOS), hybrid falls back to
BM25 silently.

Chunking is configurable per-index via `Chunking` (`Headings`,
`WholeDoc`, `Fixed { chars }`). The default is `Headings`
(one chunk per ATX section, files without headings collapse to a
single whole-doc chunk).

`reindex` is synchronous and blocking. It runs on the calling
thread; the caller decides whether to spawn a worker.
`reindex_with` accepts a progress callback driven by `BuildStage`
and `BuildProgress`; both flavors return `BuildSummary`. This
keeps the API uniffi-clean and avoids leaking an async runtime
through the FFI boundary.

### Graph

`sqlite` (rusqlite, bundled) backs the per-drive graph at
`<state_dir>/graph/<key>/graph.sqlite`. Schema lives in `nodes`
(files), `edges` (links + tags + mentions), and `headings`
(in-file anchors).

Single-writer semantics, multi-reader: the writer connection sits
behind a `Mutex<Connection>` (sqlite's contract); reads pull from
an `r2d2::Pool<SqliteConnectionManager>` so editor link-
autocomplete queries do not queue behind a reindex write or
another typeahead. WAL mode plus a uniform per-connection PRAGMA
init keep the writer and the pool agreeing on `journal_mode`,
`busy_timeout`, and `synchronous`.

`replace_file` inserts outgoing edges (links + tag/mention tokens)
and headings with computed anchors. `forget_file` removes a file
and all its edges. `clear` wipes everything for a full rebuild.

#### Link resolution

Two path forms coexist in chan-drive on purpose. The Drive API
(`read`, `write_text`, `list`, MCP tool `path` args, graph row
keys) speaks one canonical form: a drive-rooted POSIX rel path
with no leading slash and no `..`. Inside markdown bodies the
user (or an agent) writes hrefs in whatever form the renderer
expects, which for GitHub-style markdown is "relative to the file
that contains the link" so the document keeps rendering when
viewed outside chan (GitHub web, Obsidian, a pasted preview).
The normalizer below is the bridge: file bodies stay portable,
the graph and queries see one canonical destination.

Markdown link hrefs and image embeds (`[label](href)`,
`![alt](src)`) are run through `markdown::normalize_href` before
the graph builder writes the edge `dst`. The normalizer is a
pure function: input is `(href, source_dir)`, output is
`Option<String>` (clean drive-relative POSIX path; `None` for
external schemes, fragment-only refs, empty hrefs, and lexical
escapes past the drive root).

Resolution rules, in order:

1. Fragment-only `#anchor` refs return `None` (intra-document).
2. URL schemes (`https:`, `mailto:`, `tel:`, ...) detected by a
   `:` ahead of any `/` `#` `?` return `None`.
3. Trailing `?query` and `#anchor` are stripped; the anchor is
   preserved separately on the edge's `anchor` column.
4. Hrefs starting with `/` are drive-rooted (the leading slash
   is stripped); otherwise the href is joined to `source_dir`.
5. `.` / `..` segments collapse lexically. A `..` past the drive
   root returns `None`; chan-drive's no-symlink sandbox rules
   out symlink chasing here as well.

Wiki-link targets (`[[name]]`) keep the picker's existing
drive-rooted-by-default convention: bare `[[Contacts/Jane]]`
resolves to `Contacts/Jane`, an explicit `[[/Contacts/Jane]]`
likewise. Targets prefixed with `./` or `..` opt into source-
relative resolution (`[[../foo]]` from `notes/x.md` walks up to
`foo`).

The same normalizer ships as a hand-port to TS for the editor's
click handler so on-disk edges and in-editor navigation agree on
the resolved path. Edges written before the normalizer landed
keep their literal (unnormalized) `dst` until the next reindex
rewrites the table.

#### Link autocomplete (`[[`)

`link_targets` drives the editor's `[[` typeahead: the user types
a fragment and gets back files plus headings to anchor a wiki
link to. The graph DB is the source of truth (`nodes` for files,
`headings` for in-file anchors); BM25 over filename and heading
text in the search index serves a parallel purpose for free-text
search but is not used for the picker.

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

  - **Empty `q`**: most-recently-edited files first, up to
    `limit`. Useful as the picker's initial state before any
    keystroke.
  - **Non-empty `q`**: four-tier ASCII case-insensitive match.

      rank 1  basename starts with q   ("carb" -> "carbonara.md")
      rank 2  basename contains q      ("bona" -> "carbonara.md")
      rank 3  title contains q         (h1 / frontmatter title hit)
      rank 4  heading text contains q  (in-file anchor target)

    Within a rank, files sort by `mtime DESC NULLS LAST,
    rel_path ASC`; headings sort by `rel_path, ord`. Heading
    hits are capped at `limit / 2` so a single TOC-heavy file
    cannot drown out file matches.

  - **Wildcard escaping**: `%`, `_`, and `\` in `q` are escaped
    against SQLite's LIKE engine so a filename "100%off.md" is
    not matched by a raw `%` query.
  - **Case folding**: ASCII only. SQLite's `LOWER` does not fold
    Unicode without ICU; non-ASCII queries match case-
    sensitively. Acceptable for v1; revisit when a Unicode-aware
    backend becomes a priority.

After picking a file, the editor calls
`GraphView::headings_of(rel)` to populate the `[[file#` second
phase from that file alone.

### Watcher

`Drive::watch` returns a `WatchHandle`; drop to stop. The
underlying `notify::RecommendedWatcher` runs on its own thread
and calls into the consumer's `WatchCallback`.

Callback-based on purpose: the Swift / Kotlin shell implements
the trait by passing an `Arc<dyn WatchCallback>` (uniffi
generates a wrapper around a foreign object). No closures cross
the FFI.

Filter chain in `watch::dispatch` short-circuits both `.chan/`
and `.git/` subtrees. chan-drive never writes inside the user's
drive directory, so any `.chan/` activity is foreign noise.

### Contacts

Imports a third-party contact dump (Google Contacts CSV today,
vCard / Outlook later) as one markdown note per contact.

On-disk shape: slim YAML frontmatter holding only the chan-
internal classifier (`kind: contact`, `provider`, `imported_at`,
`frontmatter_version`, optional `remote_id`); contact data
(emails, phones, organizations, labels) lives in the body as
bullet items so a chan editor that doesn't strip frontmatter
shows a friendly note rather than a YAML dump. Notes from the
import follow.

Indexer reads the frontmatter in `parse_for_graph` to tag the
corresponding `nodes` row as `kind = 'contact'`. Same row,
different kind: backlinks, link-autocomplete, and forget_file
all keep working unchanged because they key on `rel_path`, not
`kind`. Downstream consumers (`Drive::contacts`, editor `@`
picker, `GET /api/contacts`) filter on `kind = 'contact'` to
surface contacts as a distinct UI surface.

Pure-function split:

| File              | Responsibility                                |
| ----------------- | --------------------------------------------- |
| `provider.rs`     | `ProviderKind` enum (parser dispatch tag)     |
| `google.rs`       | Google CSV parser; ` ::: ` multi-value form   |
| `emit.rs`         | `Contact -> markdown`; hand-formatted YAML    |
| `slug.rs`         | filename derivation, sanitization, collisions |
| `import.rs`       | orchestrator: writes via `Drive::write_text`  |

The orchestrator is exposed as `Drive::import_contacts` so the
import flow inherits the path sandbox, editable-text gate, and
atomic-rename rules. One bad contact does not abort a batch: per-
file errors land in `ImportSummary` as `Failed` outcomes.

Imported notes are user-owned the moment they land. chan does
not re-edit them. Re-importing either skips existing files or
overwrites them (per `ImportOpts.overwrite`); there is no merge.

Filename strategy: derive from `display_name`, fall back to
first email local-part, then `phone-<digits>`, then `unnamed-N`.
Sanitize path separators / control chars / Windows-reserved
chars to `_`. Trim to 120 bytes UTF-8-safely. On collision
within a batch, append ` (2)`, ` (3)`, etc. before the `.md`.

Non-goals: OAuth, API integration, two-way sync, on-disk cache.
Contact notes ARE the source of truth; the existing markdown
indexer covers read.

Contact-aware filtering for the editor `@` picker and
`GET /api/contacts` lives at the SQL layer:
`GraphView::contacts_filtered(query, limit)` runs a case-
insensitive `LIKE` against `title` and `basename` with the limit
applied inside SQLite, so per-keystroke calls stay O(limit)
instead of O(N). `GraphView::contacts()` is a convenience wrapper
for callers that want the full list.

Discovery is content-driven, not directory-driven. Any `.md` file
whose frontmatter has `chan.kind: contact` (under the `chan:`
namespace) is classified as `NodeKind::Contact` regardless of
where it sits in the drive. A user who hand-rolls a contact note
in their own folder and drops it in is picked up by the next
indexer pass; the `Contacts/` directory is just the importer's
default destination, not a discovery requirement.

Email-aware `@` picker matching is pushed down at v3. The graph
schema gains a `nodes.emails TEXT` column populated at index time:
`parse_for_graph` runs `contacts::extract_emails` over the body of
every contact-kind file, joins the lowercased addresses with
spaces, and stores them on the row. `contacts_filtered` adds a
third `LIKE ... COLLATE NOCASE` predicate against that column, so
a typed `alice` finds both "Alice Anderson" and a contact whose
only `alice` is in `alice@example.com`. The picker also receives
the deduplicated email list back so it can render a secondary line
under the contact's name.

The v3 migration cannot walk the filesystem (it runs inside
`graph.rs`, with no Drive handle), so contacts indexed before v3
keep `emails IS NULL`. `Drive::contacts_need_email_backfill`
returns `true` while any such row exists; the chan-server indexer
reads the flag on boot and queues a one-shot full rebuild so
email-aware matching works without operator intervention. The
flag clears as soon as every contact row has been re-parsed.

The chan-llm tool sandbox (and the MCP server it backs) does not
expose a contacts-aware tool, by design. Agents reach contacts
through the existing `read_file` / `list_files` / `search_content`
tools: contact files carry `chan.kind: contact` in frontmatter
plus the contact's data as readable bullets in the body, and any
note that wiki-links to a contact creates a graph edge the agent
can follow via `read_file` on the linked path. Adding a dedicated
`list_contacts` / `find_contact` tool would just duplicate
`Drive::contacts_filtered` over a wire the model already has the
primitives to traverse.

## 4. Public API surface

### Library

```rust
Library::open() -> Result<Self>
Library::open_at(config_path: PathBuf) -> Result<Self>

Library::list_drives() -> Vec<KnownDrive>
Library::default_drive_root() -> Option<PathBuf>
Library::set_default_drive_root(root: Option<PathBuf>) -> Result<()>
Library::effective_default_drive_root() -> PathBuf

Library::register_drive(root: &Path, name: Option<String>)
    -> Result<KnownDrive>
Library::unregister_drive(root: &Path) -> Result<bool>
Library::rename_drive(root: &Path, name: Option<String>) -> Result<bool>

Library::open_drive(root: &Path) -> Result<Arc<Drive>>

Library::reset_drive(root: &Path, mode: ResetMode) -> Result<ResetReport>
```

### Drive: filesystem

```rust
Drive::root() -> &Path
Drive::name() -> Option<&str>
Drive::paths() -> &DrivePaths

Drive::read(rel: &str) -> Result<Vec<u8>>
Drive::read_text(rel: &str) -> Result<String>                  // gated
Drive::read_text_with_stat(rel: &str)                          // gated
    -> Result<(String, FileStat)>
Drive::write_text(rel: &str, content: &str) -> Result<()>      // gated
Drive::write_text_if_unchanged(rel: &str,                      // gated, CAS
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

Drive::import_contacts(dir: &str,
    contacts: Vec<Contact>,
    opts: ImportOpts,
) -> Result<ImportSummary>
Drive::contacts() -> Result<Vec<ContactNode>>
Drive::contacts_filtered(query: Option<&str>, limit: usize)
    -> Result<Vec<ContactNode>>
Drive::contacts_need_email_backfill() -> Result<bool>
```

`BYTES_WRITE_LIMIT` and `TEXT_WRITE_LIMIT` cap a single write
call; callers exceeding them get `ChanError::TooLarge`.

### Drive: search and graph

```rust
Drive::search(query: &str, opts: &SearchOpts) -> Result<SearchResult>
Drive::reindex(cancel: Option<&AtomicBool>) -> Result<BuildSummary>
Drive::reindex_with<F>(cancel: Option<&AtomicBool>, progress: F)
    -> Result<BuildSummary>
    where F: FnMut(BuildProgress)
Drive::index_file(rel: &str) -> Result<()>
Drive::forget_file(rel: &str) -> Result<()>
Drive::num_indexed() -> Result<u64>
Drive::index_stats() -> Result<IndexStats>
Drive::link_targets(q: &str, limit: u32) -> Result<Vec<LinkTarget>>
Drive::resolve_link(target: &str) -> Option<ResolvedLink>

Drive::graph() -> Result<&GraphView>
GraphView::neighbors(rel: &str) -> Result<Vec<Edge>>
GraphView::backlinks(rel: &str) -> Result<Vec<Edge>>
GraphView::tags() -> Result<Vec<Tag>>
GraphView::files_with_tag(tag: &str) -> Result<Vec<String>>
GraphView::replace_file(rel, title, mtime, outgoing, headings)
    -> Result<()>
GraphView::forget_file(rel: &str) -> Result<()>
GraphView::link_targets(q: &str, limit: u32) -> Result<Vec<LinkTarget>>
```

### Drive: watch

```rust
Drive::watch(cb: Arc<dyn WatchCallback>) -> Result<WatchHandle>

trait WatchCallback: Send + Sync {
    fn on_event(&self, event: WatchEvent);
}
```

### Contacts

```rust
contacts::google::parse_google_csv(rdr: impl Read) -> Result<Vec<Contact>>
contacts::emit::render_markdown(c: &Contact, ctx: &EmitContext) -> String
contacts::slug::slug_for(c: &Contact, dir: &str,
    taken: &mut HashSet<String>,
    unnamed_counter: &mut usize,
) -> String

ProviderKind::{Google}
ProviderKind::as_str(self) -> &'static str
ProviderKind::parse(s: &str) -> Option<Self>

ImportOpts { overwrite: bool }
ImportOutcome::{Wrote, Overwrote, Skipped, Failed}
ImportSummary { outcomes: Vec<ImportOutcome> }
ImportSummary::counts(&self) -> ImportCounts

// Graph projection (see Components -> Contacts).
NodeKind::{File, Contact}
ContactNode { rel_path, basename, title }
```

### Public types (selected)

`SearchMode { Bm25, Dense, Hybrid }`, `SearchOpts`, `SearchResult`,
`Hit`, `Chunking { Headings, WholeDoc, Fixed { chars } }`,
`IndexConfig`, `IndexStats`, `BuildOptions`, `BuildProgress`,
`BuildStage`, `BuildSummary`. `ResetMode { Cache, Everything }`,
`ResetReport`. `KnownDrive`, `Registry`. `Edge`, `EdgeKind`, `Tag`,
`HeadingRow`, `LinkTarget`, `LinkTargetKind`. `WatchEvent`,
`WatchKind`, `WatchHandle`, `WatchCallback`. `ChanError`, `Result`.

All public types are owned (no lifetimes) and `Send + Sync`.

## 5. Invariants and trust boundaries

### Sandbox and path resolution

  - Every Drive entry point uses `fs_ops::resolve_safe_strict`
    (lexical sandbox plus a canonical-form check on the deepest
    existing ancestor). New entry points must do the same. The
    strict variant catches mid-path symlinks pointing outside
    the drive.
  - `lstat`, never `stat`, on user paths: `read` / `write` /
    `stat` / `remove` use `fs_ops::ensure_regular_file` or
    `std::fs::symlink_metadata` so a symlink target can't mask
    the link. New ops touching user content must apply the same
    gate.
  - `fs_ops::is_editable_text(rel)` is the single predicate for
    "the editor can safely round-trip this file through a UTF-8
    buffer." `read_text` / `write_text` enforce it; binary
    callers use `read` / `write_bytes`.
  - The crate also uses `cap-std` / `cap-tempfile` for sandboxed
    Dir-relative atomic writes on hot paths, closing the TOCTOU
    window between resolve and open by anchoring the open at a
    pre-validated `Dir` handle.

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

  - `walkdir::follow_links(false)` and `same_file_system(true)`
    so symlink loops can't occur and a misregistered drive that
    spans onto a network mount won't drag the indexer into it.
  - Iteration drops non-regular non-dir entries, so the UI tree
    and the indexer only ever see real files and dirs.

What's NOT closed today:

  - Pure-libstd TOCTOU on cold paths: a path that passes the
    strict resolve could in principle be swapped for a symlink
    before a non-cap-std open syscall. The hot writer paths use
    cap-std and so close this; cold paths rely on the strict
    resolve plus single-user threat model. Closing fully on
    Linux requires `openat2(2)` with `RESOLVE_BENEATH |
    RESOLVE_NO_SYMLINKS` (Linux 5.6+) or per-component
    `O_NOFOLLOW` on platforms without openat2. Tracked as
    future hardening.

### Atomic writes

Anything chan-drive-managed (registry, sessions, blob storage,
graph control records, atomic-write user files) routes through
`fs_ops::atomic_write` (or its cap-std equivalent for sandboxed
writes): tmpfile in the same directory, fsync the file, rename
into place, fsync the directory. Mode + xattrs (Finder tags on
macOS, SELinux labels and capabilities on Linux) are captured
from the existing target before the rename and restored on the
new file. Never `std::fs::write` directly to the target. A crash
mid-write must produce zero state for the writer plus an intact
previous version.

### Locking model

```
register / open               read                          write
-----------------             -----------                   ------
Library::register_drive       Drive::read*                  Drive::write_*
  Mutex<Registry> lock          fs_ops::resolve_safe          fs_ops::atomic_write
  atomic_write registry         (no lock)                     (no lock)

Drive::open                   Drive::search                 Drive::reindex
  fs4::try_lock_exclusive       tantivy reader (no lock)      tantivy writer
  on writer.lock                                              (held by lock)

                              GraphView reads               GraphView writes
                                r2d2 pool                     Mutex<Connection>
                                (intra-process)               (intra-process)
```

Two distinct concurrency primitives:

  - `DriveLock` (cross-process): held for the lifetime of
    `Drive`. A second process opening the same drive errors
    immediately with `ChanError::DriveLocked`; we do NOT block.
    Callers handle the error explicitly (CLI prints a message
    and exits; desktop app falls back to opening another
    drive).
  - `Mutex<Registry>`, `Mutex<Connection>` (graph writer), and
    the r2d2 pool (graph readers): intra-process. Cheap.

Reading does not take the cross-process lock. tantivy is multi-
reader-safe by design; sqlite WAL mode plus the r2d2 reader pool
lets concurrent readers proceed alongside the writer.

### Drive-internal noise filter

chan-drive stores ZERO files inside the user's drive directory.
The drive is purely user content (markdown, attachments). This
makes it safe to drop a drive inside an existing git repo, an
iCloud / Google Drive / Dropbox folder, or anywhere else. A stray
`.chan/` left over from an older install or created by a third-
party tool is filtered out by `walk_drive` and `watch::dispatch`;
chan-drive never emits events for it or includes it in
`list_tree`.

## 6. On-disk layout

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
                                  (chan-drive allocates the dir; the
                                  app, e.g. chan-server, owns the
                                  contents)
  trash/<key>/<id>/               per-drive Trash. Each entry holds
    payload | payload/            the moved file or directory and
    meta.json                     a JSON sidecar (original_path,
                                  deleted_at, is_dir, size). meta is
                                  written last; sweep treats meta-
                                  less entries as crash leftovers.

$CACHE_DIR/chan/                  (cache_dir; rebuildable)
  index/<key>/                    per-drive tantivy segments + dense
                                  vectors + config.toml
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
visible" argument for `~/.chan/` does not apply on mobile (the
user cannot browse the sandbox anyway), so the registry lives
alongside the rest of the per-app state.

### Drive contents

User content only: markdown, attachments, whatever the user puts
there. chan-drive never writes inside the drive root. This is a
load-bearing invariant for the "drive a git repo / sync folder"
story.

## 7. Error model

One umbrella `ChanError` enum so the Swift / Kotlin error type
is a single tagged union. All foreign errors (`io`, `toml`,
`rusqlite`, `notify`, `tantivy`) collapse into `ChanError::Io`,
`::ConfigDecode`, `::Graph`, `::Watch`, `::Index` with their
`Display` text preserved.

Variants intentionally do not carry rich nested types: uniffi can
encode an enum with primitive payloads; nested error chains do
not round-trip cleanly across the FFI.

Notable variants:

  - `DriveLocked`: another process holds the writer lock.
  - `WriteConflict { current_mtime }`: CAS write lost the race.
  - `TrashOccupied`: restore would clobber a live entry.
  - `SpecialFile`: target is a symlink, FIFO, socket, or device.
  - `TooLarge`: write exceeds `BYTES_WRITE_LIMIT` /
    `TEXT_WRITE_LIMIT`.

## 8. Schema versioning

  - **Graph DB**: `PRAGMA user_version`. Migrations are
    idempotent and applied on every `GraphView::open`. Current
    version: 2 (basename-backfill bump on top of v1's initial
    schema). The migration writes the schema change and the
    `user_version` bump in a single transaction so a crash
    mid-migration leaves the DB at the previous version with
    intact data.
  - **Search index**: `IndexConfig.schema_version` field
    persisted at `<index_dir>/config.toml` alongside the
    embedding model id and chunking strategy. Current version:
    3 (indexer widened from `.md`-only to every
    `FileClass::EditableText` extension, i.e. `.md` + `.txt`).
    Mismatched versions wipe `bm25/` + `embeddings/` and
    rebuild on next open. Model swaps force a rebuild because
    embedding dimensions and numerical drift differ.

A schema bump in either store is user-data-safe: only chan-
managed cache is destroyed; the user's notes are untouched.

## 9. Consumers

`chan-drive` is consumed by the following crates. The first three
live in the sibling repo `chan-writer/chan` and are pulled as
path deps for now; switch to git or crates.io deps when the
repos go public. The fourth is forward-looking.

### `chan-writer/chan` :: `chan` (CLI binary)

The `chan` binary parses CLI args (clap) and dispatches
subcommands (`add`, `list`, `remove`, `rename`, `serve`, `index`,
`search`, `upgrade`, an internal `__mcp`). It depends on
`chan-drive` directly with `default-features = false`, then
re-enables `embeddings` (and `metal` on macOS, `cuda` opt-in on
Linux) through its own feature passthroughs so `--no-default-
features` propagates end-to-end. It also depends on `chan-server`
for the `serve` subcommand and on `chan-llm` (with `mcp`) for
the in-process MCP server.

Usage shape:

  - `Library::open()` once at startup.
  - `library.list_drives()`, `register_drive`, `rename_drive`,
    `unregister_drive` for the registry subcommands.
  - `library.open_drive(root)` to get an `Arc<Drive>`, then
    `drive.search(...)`, `drive.reindex(...)`, `drive.list_tree()`
    for `index` / `search` / direct CLI access.
  - `Library::reset_drive` for `chan reset`.

### `chan-writer/chan` :: `chan-server` (HTTP + WebSocket)

`chan-server` wraps `chan-drive`'s `Library` / `Drive` handles in
axum routes and serves the embedded Svelte frontend (rust-embed).
It exposes REST endpoints for filesystem ops, search, graph
traversal, link autocomplete, and trash management; a WebSocket
channel for `WatchEvent`s; and the assistant and session blob
endpoints that proxy directly to `put_assistant` /
`put_session`.

It depends on `chan-drive` with `default-features = false` and
forwards `embeddings`, `metal`, `cuda` through its own feature
gates. `chan-server`'s "embeddings on" UI badge reflects the
chan-drive feature state at compile time.

Usage shape:

  - One `Arc<Library>` in the axum extension state.
  - `Arc<Drive>` per-drive in a `RwLock<HashMap<DriveKey,
    Arc<Drive>>>` populated lazily on first request.
  - WebSocket handler installs a `WatchCallback` that
    serializes `WatchEvent`s onto the socket; drops the
    `WatchHandle` on disconnect.
  - All HTTP filesystem ops route through `Drive` so the
    sandbox, special-file refusal, atomic writes, and editable-
    text gate apply automatically. `chan-server` never reads or
    writes drive contents directly.

### `chan-writer/chan` :: `fetch-models` (build helper)

`fetch-models` is a build-time helper that pre-fetches the
default embedding model (`BAAI/bge-small-en-v1.5`, ~130 MB) into
`crates/chan-server/resources/models/` so chan-server's rust-
embed step bundles it into the release binary. It depends on
`chan-drive` with the `embeddings` feature explicitly enabled to
reuse the same hf-hub + tokenizers stack the runtime uses, so a
contributor's `cargo build` does not pay the model download
unless `make models` (or `make build-release`) runs.

The crate uses `chan_drive::DEFAULT_MODEL` and the embedder's
fetcher entry point; it does not open a `Drive`.

### Future native shells (iOS / Android)

`chan-drive`'s public API is shaped to survive a uniffi
boundary: no lifetimes on public types, owned `String` /
`PathBuf` only, `Arc`-able handles, one umbrella `ChanError`
enum with primitive payloads, callback-based streaming through
`Arc<dyn WatchCallback>` instead of `impl Stream` or channels.
The native shells will link the same crate via uniffi and share
the atomic-write / sandbox / blob-storage semantics with the
desktop chan-server without reimplementing them per platform.
The `--no-default-features` build (BM25-only, no candle) is the
expected starting point for iOS until candle-core builds cleanly
for that target.

## 10. What's wired vs. what's ahead

Wired and tested:

  - **Markdown parser** (`src/markdown/`): native-only port of
    `chan-shared`'s frontmatter, ATX heading, link, and
    reference-token extractors. Powers both the search title /
    body extraction and the graph indexer. The wasm-only smart-
    node serialization stays in the chan repo as an editor
    concern.
  - **Search index** (`src/index/`): tantivy 0.24 BM25 plus an
    optional candle-backed dense path under the `embeddings`
    feature. `Hybrid` mode runs both and fuses with reciprocal-
    rank fusion. Schema versioned via `<index_dir>/config.toml`;
    mismatches wipe and rebuild on next open. `Drive::search`,
    `Drive::index_file`, `Drive::reindex`, `Drive::reindex_with`,
    `Drive::forget_file`, `Drive::num_indexed`,
    `Drive::index_stats` all functional.
  - **Graph reads + writes** (`src/graph.rs`): `neighbors`,
    `backlinks`, `tags`, `files_with_tag`, `files`,
    `headings_of` return real data. `replace_file` inserts
    outgoing edges (links + tag/mention tokens) and headings
    with computed anchors. `clear` wipes everything for a full
    rebuild. WAL plus r2d2 reader pool; writer behind a Mutex.
  - **Watcher** (`src/watch.rs`): `Drive::watch` is wired and
    filters drive-internal noise (`.chan/`, `.git/`).

Still ahead:

  - **Per-section search snippet breadcrumbs**: hits return per-
    chunk snippets but the breadcrumb (`heading_path`) is not
    fully populated for every chunking mode. Tightens relevance
    for long files.
  - **Wiki-link resolution**: `Drive::resolve_link` exists for
    direct (`recipes/pasta` to `recipes/pasta.md`) lookups.
    Fuzzier resolution (prefix-match, alias table) lives at the
    consumer layer for now. Could move into chan-drive if every
    consumer needs the same logic.
  - **Built-in watcher consumer**: `Drive::watch` is wired but
    no built-in consumer feeds events into `index_file` /
    `forget_file`. Apps run their own loop. See
    `graph_indexer` in section 11.
  - **TOCTOU hardening on cold paths**: hot writer paths use
    cap-std; cold paths rely on the strict resolve plus single-
    user threat model. Migrate the rest to cap-std or
    `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)` where
    available.

## 11. Future extensions

Sketch only, not committed:

  - `Drive::graph_indexer()`: returns a structured indexer that
    accepts `WatchEvent`s and calls `replace_file` /
    `forget_file` accordingly. Apps run it on a worker thread
    pulling from the watcher channel. Removes the duplicated
    indexer loop from chan / chan-server.
  - Remote-backed `Drive` impl: a future trait split could let
    a thin client call `read` / `write` / `search` against an
    HTTP endpoint while the server runs the real chan-drive.
    Not designed for now; the API surface is shaped to allow
    it later. The chan-tunnel crates already give us the
    transport.
  - Background reindex job handle: if synchronous reindex turns
    out painful in practice, add a `JobHandle` returned by
    `reindex_async()` that the caller polls. Defer until
    needed; today the consumer spawns its own worker thread
    around `reindex_with`.
  - Configurable trash retention as a `Library` setting
    (currently hardcoded to 30 days).
  - Unicode-aware case folding for `link_targets` once an ICU-
    or `unicase`-backed path is wired into the SQLite query.
