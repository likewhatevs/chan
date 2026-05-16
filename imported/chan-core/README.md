# chan-core

Native Rust libraries that form the cross-platform core of
[chan](https://github.com/chan-writer/chan), a local-first markdown
editor. Built as plain libraries (not an app) so the same primitives
back the `chan` CLI today and Swift / Kotlin shells via uniffi as
native clients land.

## Why a workspace this size for a markdown editor

chan is more than a text box. It treats a directory of markdown as
a "drive": a sandboxed filesystem with full-text and graph search,
soft-delete to a per-drive trash, optional public access via a
self-hosted tunnel, and an embedded LLM assistant whose tool calls
route through the same filesystem gates the editor uses. That
feature surface is shared across the CLI and future native clients;
porting it per platform was not an option, so the primitives live
in cross-platform Rust here. Each consumer (CLI, iOS, Android)
links the same crates and gets the same behavior in lockstep.

## Two design constraints shape every public API

1. **FFI-shape from day one.** Every public type has to survive a
   uniffi boundary later. No lifetimes on public types; owned
   `String` / `PathBuf` only; `Arc`-able handles; one umbrella
   error enum per crate with primitive payloads; streaming through
   callback trait objects, never `impl Stream` or
   `tokio::sync::mpsc::Receiver`. uniffi bindings produce when the
   first native shell lands; today the CLI links the crates
   directly.
2. **Abstract chan's business logic.** Filesystem sandboxing,
   atomic writes, the editable-text gate, the symlink/special-file
   policy, the trash, the search and graph indexes, the LLM tool
   contract, the tunnel handshake: all of these are chan's product
   decisions, not implementation details of any one consumer.
   Holding them here keeps the CLI, the desktop app, and future
   mobile shells consistent. A backend or a frontend cannot drift
   away from the gates because the gates are the only API on offer.

## Crates

```
Crate                 Role
--------------------  -----------------------------------------------
chan-drive            Library / Drive handles. Sandboxed FS,
                      tantivy search (BM25 + dense hybrid), sqlite
                      graph DB, watcher, per-drive trash and blob
                      storage.
chan-tunnel-proto     Wire types and control frames. Pure data,
                      no I/O, no async.
chan-tunnel-client    Dial + Hello handshake; embedded into
                      `chan serve` to expose a drive on a public
                      URL via the tunnel terminator.
chan-tunnel-server    Server-side of the tunnel. Library that
                      terminates tunnels dialed by `chan serve`
                      and exposes the drive-side substreams to a
                      public-facing router.
chan-llm              CLI-agent backends (Claude CLI, Gemini CLI,
                      Codex CLI), embedded prompts,
                      and the tool sandbox routed through
                      chan-drive. Optional `mcp` feature ships a
                      stdio MCP server and the `chan-llm-mcp`
                      binary.
```

Each crate ships a `README.md` (pitch, install, public surface at
a glance, build) and a `design.md` (canonical design reference:
problem, architecture, on-disk layout, invariants, error model,
consumers, what's wired). The workspace-wide `CLAUDE.md` collects
contributor guidelines that apply to every crate.

## Who consumes this

```
Consumer                              Direct deps
------------------------------------  ----------------------------
chan-writer/chan (chan binary)        chan-drive, chan-llm
chan-writer/chan (chan-server)        chan-drive, chan-llm,
                                      chan-tunnel-client,
                                      chan-tunnel-proto
chan-writer/chan (fetch-models)       chan-drive (embeddings)
chan-writer/chan-gateway              chan-tunnel-server (proto
  (drive-proxy)                       pulled transitively)
chan-writer/chan-ios (later)          chan-drive, chan-llm
                                      via uniffi
chan-writer/chan-android (later)      chan-drive, chan-llm
                                      via uniffi
```

The `chan` binary pulls the tunnel crates transitively through
`chan-server`; only `chan-server` and `drive-proxy` use the tunnel
public API.

`chan-server` is the HTTP + WebSocket layer wrapping `chan-drive`'s
`Library` in axum routes and forwarding `LlmSession` events to the
web frontend. `drive-proxy` terminates user-dialed tunnels at the
public gateway and routes incoming HTTP requests to the right live
tunnel. See each crate's `design.md` for the per-consumer wiring.

## Status

Pre-alpha. The public API is shaped and the chan-drive primitives
(registry, sandboxed FS, atomic writes, cross-process writer lock,
tantivy hybrid search, sqlite graph reads / writes, watcher) are
real. A built-in watcher consumer that drives reindex from
`WatchEvent`s is still ahead. The chan-llm backends are wired and
stream end-to-end. uniffi bindings have not landed yet; they ship
with the first native shell.

## Build

```
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The Rust toolchain is pinned in `rust-toolchain.toml`. Install
[rustup](https://rustup.rs/); it picks up the pin automatically the
first time you run `cargo` in this directory and downloads the
matching compiler. CI uses the same file, so local and cloud
clippy lint sets stay locked together. Bumping Rust means editing
`rust-toolchain.toml` and fixing any new clippy findings in the
same commit.

Install the pre-push hook once per clone:

```
./scripts/install-hooks
```

The hook runs `cargo fmt --check`, `cargo clippy -- -D warnings`,
`cargo test --all-targets`, and
`cargo build --no-default-features` with `RUSTFLAGS=-D warnings`
before every push, mirroring CI. A passing local push therefore
will not fail in GitHub Actions.

## License

Apache-2.0. See [`LICENSE`](LICENSE).
