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

## Crates

```
Crate                 Role
--------------------  -----------------------------------------------
chan-drive            Library / Drive handles. Sandboxed FS,
                      tantivy search, sqlite graph DB, watcher,
                      per-drive trash and blob storage.
chan-tunnel-proto     Wire types and control frames. Pure data,
                      no I/O.
chan-tunnel-client    Dial + Hello handshake; embedded into
                      `chan serve` to expose a drive on a public
                      URL via the tunnel terminator.
chan-tunnel-server    Server-side of the tunnel implementation.
                      Library that terminates tunnels dialed by
                      `chan serve` and exposes the drive-side
                      substreams to a public-facing router.
chan-llm              LLM backends (Anthropic, Gemini, Ollama,
                      Claude CLI), embedded prompts, and the tool
                      sandbox routed through chan-drive. Optional
                      `mcp` feature ships a stdio MCP server and
                      the `chan-llm-mcp` binary.
```

Each crate has its own `README.md` with the canonical design
reference; the workspace-wide `CLAUDE.md` collects contributor
guidelines for all of them.

## Who consumes this

- `chan-writer/chan`: the CLI and embedded web editor. Today's
  primary consumer. Links chan-drive directly; speaks to chan-llm
  for the assistant; embeds chan-tunnel-client in `chan serve`.
- `chan-writer/chan-ios` (later): SwiftUI shell linking
  chan-drive + chan-llm via uniffi.
- `chan-writer/chan-android` (later): Compose shell linking the
  same crates via uniffi.

The crates are FFI-shaped from day one: no lifetimes on public
types, owned strings only, callback-based watcher and assistant
streaming, no public `async fn`. uniffi bindings produce when the
first native shell lands.

## Status

Pre-alpha. The public API is shaped and the chan-drive primitives
(registry, sandboxed FS, atomic writes, cross-process writer lock,
tantivy BM25, sqlite graph reads / writes, watcher) are real.
Hybrid (BM25 + dense) search and a built-in watcher consumer are
still ahead. The chan-llm backends are wired and stream; uniffi
bindings have not landed yet.

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
