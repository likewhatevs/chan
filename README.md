# chan

Notes app with embedded web editor. The `chan` binary is a CLI plus
an HTTP server that serves a Svelte WYSIWYG editor for plain markdown
drives, with cross-file `[[wiki-link]]` autocomplete and BM25
content search.

## Layout

```
crates/
  chan         the binary. Subcommands (add, list, remove, rename,
               serve, index, search). Embeds the web frontend at
               build time.
  chan-server  HTTP + WebSocket surface. Wraps chan-core in axum
               routes; uses chan-llm for assistant routes.
web/           Svelte frontend, embedded into the binary at build
               time. Wires in a later commit.
```

Two sibling crates pulled in as path deps:

- `chan-writer/chan-core` (filesystem, search, graph, drive
  registry).
- `chan-writer/chan-llm` (LLM backends, embedded prompts, tool
  sandbox, key resolution). Lives in its own repo so native
  shells (iOS / Android) can link it via uniffi without
  dragging in chan-server's HTTP stack.

The workspace assumes the sibling-checkout layout
`~/dev/github.com/chan-writer/{chan,chan-core,chan-llm}`.

## Status

Pre-alpha. Initial commit is a workspace skeleton: `chan add`,
`chan list`, `chan remove`, `chan rename`, `chan index`, and
`chan search` work end-to-end against the new chan-core. `chan
serve` errors with "not implemented yet" until routes finish
porting from `fiorix/chan` into `chan-server`. Same for LLM,
attachments, sessions, and the assistant chat history; all live
in app-level config files once `chan-server` and `chan-llm` land.

## Build

```bash
git clone git@github.com:chan-writer/chan-core ../chan-core
git clone git@github.com:chan-writer/chan-llm ../chan-llm
cargo build
cargo test
cargo run -- list
```

## Contributing

The Rust toolchain is pinned in `rust-toolchain.toml` (1.95.0).
Install [rustup](https://rustup.rs/); it picks up the pin
automatically the first time you run `cargo` here.

Install the pre-push hook once per clone:

```
./scripts/install-hooks
```

The hook runs `cargo fmt --check`, `cargo clippy -- -D warnings`,
and `cargo test --all-targets` before every push, mirroring CI.
A passing local push will not fail in GitHub Actions once CI
lands (cross-repo auth between two private repos is the open
item; tracked in `design.md`).
