# CLAUDE.md

Contribution guidelines for Claude Code (claude.ai/code) when working
on `chan-core`.

## What This Project Is

This repo is a Cargo workspace. The headline crate is `chan-core`,
the low-level Rust library extracted from the chan markdown editor:
it owns the per-machine registry of known drives, exposes a
path-based, sandboxed filesystem API rooted at each drive, and
wraps the per-drive search index and graph database. The contract
documented below is for the `chan-core` crate specifically.

Sibling crates in the same workspace add layers that build on
chan-core's primitives:

  - `chan-tunnel-{proto,client,server}` — h2/yamux drive tunnel
    used by the gateway terminator and embedded into `chan serve`.
  - `chan-llm` — LLM backends, embedded prompts, the tool sandbox
    the assistant uses to read/edit chan drives via the chan-core
    API, and the `chan-llm-mcp` MCP server binary.

Each sibling crate has its own `CLAUDE.md` for crate-specific
guidance. The `chan-core` crate itself stays HTTP/WS/LLM/UI free;
those concerns live in the sibling crates and in downstream apps:

  - `chan-writer/chan`        CLI + embedded web editor (HTTP, WS,
                              frontend bundle, LLM tool calls,
                              editor preferences, API keys).
  - `chan-writer/chan-ios`    SwiftUI app linking chan-core via FFI
                              (later).
  - `chan-writer/chan-android` Compose app linking chan-core via FFI
                              (later).

The release artifact is a Rust library crate. FFI bindings to Swift
and Kotlin are produced by uniffi (added in a later iteration); the
public Rust API is shaped to be uniffi-compatible from day one.

## Build & Test

```bash
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The Rust toolchain is pinned in `rust-toolchain.toml`. `cargo`
auto-installs the pinned version through rustup on first use, so
contributor and CI clippy lint sets stay locked together. The pre-
push hook (`./scripts/install-hooks` to install) runs the same
gate as CI under the pinned compiler, with `RUSTFLAGS=-D warnings`
plus the `--no-default-features` build, so a passing local push
will not fail in the cloud.

Bumping Rust = edit `rust-toolchain.toml` + fix new clippy
findings in the same commit. Don't drift between local and CI.

The crate has no external runtime dependencies beyond the OS:
`tantivy` (search), `rusqlite` with the `bundled` feature (graph),
`notify` (filesystem watcher), `fs4` (cross-process locks). All ship
as static dependencies.

## Project Principles

### One Library handle, many Drive handles

- `Library` is per-machine. It owns the drive registry persisted at
  `~/.chan/config.toml` (or, on iOS / Android where the home dir is
  not user-writable, the platform data dir co-located with state).
  Cheap to clone (Arc inside).
- `Drive` is per registered directory. Held while open via a
  cross-process advisory lock (`fs4::FileExt::try_lock_exclusive`)
  so two processes (e.g. `chan serve` and the desktop app) cannot
  both write the same drive's index or graph DB. Reads do not take
  a lock; tantivy and sqlite handle their own multi-reader
  concurrency.
- `Library::open_drive` returns `Arc<Drive>` so the handle can be
  shared across threads and across the FFI boundary later.

### Filesystem safety

- Every write goes through `fs_ops::atomic_write`: tmpfile in the
  same directory, fsync, atomic rename. Never `std::fs::write`
  directly to the target.
- `fs_ops::resolve_safe` rejects `..` components, absolute roots,
  and Windows drive prefixes. Use it for any path that arrived
  from outside chan-core's trust boundary (HTTP request, link
  target, FFI argument).
- `fs_ops::list_tree` and `walk_drive` skip `.chan/` and `.git/`
  at any depth.

### Path-based public API

- All public Drive methods take `&str` relative paths, POSIX-style
  on every platform. The internal type is `&Path` after
  `resolve_safe`; the `&str` boundary is what the FFI uses.
- Editable text is whitelisted by extension (`.md`, `.txt`) via
  `fs_ops::is_editable_text`. `read_text` / `write_text` enforce
  this. Binary I/O goes through `read` / `write_bytes` and is
  intentionally not gated (attachments, future media browser).

### Drive is the boundary

- Discovery is registry-driven: a path is a drive iff
  `Library::register_drive` has been called for it. There is no
  per-file walk, no upward search, no auto-discovery.
- chan-core stores ZERO chan-managed files inside the user's drive
  directory. The registry, per-drive index, graph DB, sessions,
  assistant history, and locks all live outside the user's notes
  tree. Dropping a drive inside an existing git repo (or anywhere
  else, including iCloud / Google Drive / Dropbox) is fully
  supported and doesn't fight the user's own commit flow.

### Per-machine state lives in OS-standard dirs

- **Config** (`paths::config_dir()`): `~/.chan/` on desktop targets;
  `state_dir()` on iOS / Android (sandbox: home dir is not
  user-writable). Holds the registry only.
- **State** (`paths::state_dir()`): per-drive sessions, assistant
  history, sqlite graph DB, and writer locks. XDG-shaped on Linux;
  `Library/Application Support/chan` on macOS; equivalents on
  Windows / iOS / Android.
- **Cache** (`paths::cache_dir()`): per-drive search-index
  segments. Wipeable; everything inside rebuilds on demand.
- Per-drive subpaths come from `paths::drive_paths(root)`, keyed
  by `sha256(canonical_path)[..16]`. Renames invalidate the keys
  (rebuild on next open; no rename tracking).

### Cross-process safety

- `lock::DriveLock::acquire(lock_dir)` takes an exclusive advisory
  lock on `<lock_dir>/writer.lock`. Held for the lifetime of the
  Drive. A second `Drive::open` for the same drive returns
  `ChanError::DriveLocked` immediately; callers do not block.
- Writes to the registry serialize through a `Mutex<Registry>`
  inside the `Library`, so concurrent threads in one process don't
  race; the on-disk file is always written via `atomic_write` so a
  crash mid-write produces zero state for the writer plus an intact
  previous version.

### Sync, blocking API

- All public methods are synchronous and blocking. `reindex` and
  graph rebuilds run on the calling thread. The caller decides
  whether to spawn its own thread or block. This keeps the API
  uniffi-clean (no async-runtime negotiation across the FFI) and
  matches the editor's pattern of running long-running work on a
  worker thread the caller owns.

### Schema versioning

- The graph DB uses `PRAGMA user_version` for migrations.
  `GraphView::open` runs the migration to the current version on
  every open. Do not hand-edit schema; add a migration step.
- The search index will use a `schema_version` field in its on-disk
  config (forthcoming). Mismatched versions trigger a full rebuild
  on next open. Model swaps automatically force a rebuild because
  embedding dimensions differ per model.

### FFI compatibility (uniffi)

- Public types must not carry lifetimes. Owned `String` / `PathBuf`
  in public method args and returns; no `&str` returned from public
  methods.
- Watcher is callback-based via the `WatchCallback` trait. No
  closures cross the FFI boundary.
- One umbrella `ChanError` enum so the Swift / Kotlin error type is
  a single `enum`, not a tower of mapped variants.

## Writing Rules

- **No em dashes** in comments or documentation. They hurt
  readability in terminals and divert the reader's train of thought.
  Use commas, semicolons, parentheses, or separate sentences.
- **Tables**: pure ASCII, target 80 columns, no Unicode box-drawing.
- **Factual**: no marketing language ("just", "easy", "blazing").
  When reporting test results or benchmarks, include analysis of
  what the numbers mean and whether they meet expectations.
- **Comments**: explain WHY, not WHAT. The code shows what; the
  comment explains the reasoning, the trade-off, or the constraint.

## Contributor Patterns

- **Atomic writes for any chan-core-managed file**: registry,
  per-drive sessions, graph DB control records. Use
  `fs_ops::atomic_write`. A crash mid-write produces zero state
  for the writer plus an intact previous version, never a torn
  file.
- **Path resolution always strict**: every Drive entry point
  uses `fs_ops::resolve_safe_strict` (lexical sandbox plus a
  canonical-form check on the deepest existing ancestor). New
  entry points must do the same; never call `resolve_safe` or
  `Path::join` directly on user-supplied input. The strict
  variant catches mid-path symlinks pointing outside the drive
  (e.g. `Backup -> /Volumes/external` inside the drive) that
  the lexical check would miss.
- **lstat, never stat, on user paths**: read/write/stat/remove
  all use `fs_ops::ensure_regular_file` or
  `std::fs::symlink_metadata` so a symlink's target can't mask
  the link itself. This is the gate that keeps the layer from
  blocking on a FIFO with no writer, draining `/dev/zero`, or
  silently traversing a symlink off the drive. New ops that
  touch user content must apply the same gate.
- **Editable-text gate**: `fs_ops::is_editable_text(rel)` is the
  single predicate for "the editor can safely round-trip this file
  through a UTF-8 buffer." `Drive::read_text` and `Drive::write_text`
  enforce it. Binary callers (attachments, future media browser)
  use `read` / `write_bytes` and own their own gate.
- **Watcher drops `.chan/` events unconditionally**: chan-core
  never writes inside the user's drive directory, so any `.chan/`
  activity is foreign noise. The filter chain in `watch::dispatch`
  short-circuits the whole subtree alongside `.git/`.
- **Tests use isolated config dirs**: `Library::open_at(path)`
  takes an explicit config path so tests never touch the
  developer's real `~/.chan`. Always use this in tests; never call
  `Library::open()` from a test.
- **Sync API only in public surface**: do not introduce `async fn`
  or tokio in public methods. Internal helpers may use threads
  (the watcher does); they must not leak into the API.
- **Send + Sync on public types**: anything held inside `Arc<Drive>`
  must be `Send + Sync`. `rusqlite::Connection` is `Send` but not
  `Sync`, so `GraphView` wraps it in a `Mutex`; tantivy types are
  `Sync` and don't need wrapping.

## Documentation

- **Design and architecture**: [`design.md`](design.md). Single
  load-bearing reference for the on-disk layout, the public API
  shape, and the locking model. Update in the same commit as any
  change that affects them.
- **Issue tracker**: GitHub repo `chan-writer/chan-core`.
