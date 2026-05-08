# chan-core

Cargo workspace for the cross-platform Rust core of chan-writer.
The headline crate is `chan-drive` (filesystem, search, and graph
primitives, named for its public API: a `Library` registry of
`Drive` handles); sibling crates add the tunnel transport
(`chan-tunnel-{proto,client,server}`) and the LLM layer
(`chan-llm`).

`chan-drive` is the low-level core extracted from the chan editor.
It owns the registry of known drives, exposes a path-based,
sandboxed filesystem API rooted at each drive, and wraps the
per-drive search index and graph database. It does not contain
HTTP, WebSocket, LLM, or UI code; those are app-level concerns
that build on top.

## Status

Pre-alpha skeleton. Public API is shaped but search and graph are
stubs (BM25 implementation lands next). The FS primitives,
registry, and cross-process writer lock are real.

## Design

- One `Library` per machine, persisted to `~/.chan/config.toml`.
- One `Drive` per registered directory; held while open via a
  cross-process advisory lock so two processes can't write the
  same drive's index/graph at once.
- All public API is path-based with POSIX-style relative paths
  rooted at the drive. Path traversal is rejected up-front.
- Editable text is whitelisted by extension (`.md`, `.txt`).
  Binary writes go through a separate route.
- The FFI surface is uniffi-friendly: no lifetimes on public
  types, owned strings only, callback-based watcher.

## Layout

```
src/
  lib.rs      public façade
  error.rs    ChanError + Result<T>
  paths.rs    OS-standard locations, drive_paths()
  fs_ops.rs   atomic_write, resolve_safe, list_tree
  registry.rs known-drives TOML registry
  lock.rs     cross-process writer lock (fs4)
  library.rs  Library handle + registry mutators
  drive.rs    Drive handle + FS / search / graph / watch
  search.rs   tantivy-backed index (stub)
  graph.rs    sqlite-backed graph DB (stub)
  watch.rs    notify-backed FS watcher
```

## What's intentionally NOT here

- HTTP server, WebSocket, frontend bundle (lives in `chan` repo).
- LLM / assistant tool calls, API key storage.
- Editor preferences (fonts, theme, keybindings).

## Build

```
cargo build
cargo test
cargo fmt
cargo clippy --all-targets -- -D warnings
```

## Contributing

The Rust toolchain is pinned in `rust-toolchain.toml`. Install
[rustup](https://rustup.rs/); it picks up the pin automatically
the first time you run `cargo` in this directory and downloads
the matching compiler. CI uses the same file, so local and cloud
clippy lint sets stay locked together. Bumping Rust means editing
`rust-toolchain.toml` and fixing any new clippy findings in the
same commit.

Install the pre-push hook once per clone:

```
./scripts/install-hooks
```

The hook runs `cargo fmt --check`, `cargo clippy -- -D warnings`,
`cargo test --all-targets`, and `cargo build --no-default-features`
with `RUSTFLAGS=-D warnings` before every push, mirroring CI. A
passing local push therefore will not fail in GitHub Actions.
