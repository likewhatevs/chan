# CLAUDE.md

Contribution guidelines for Claude Code (claude.ai/code) when
working on `chan`.

## What This Project Is

`chan` is the user-facing notes app: a CLI plus an HTTP server
that serves an embedded Svelte WYSIWYG editor for plain markdown
drives. The CLI subcommands manage the drive registry, drive
contents, search, and the running server. The server is loopback-
only, single-user, single-machine; multi-user collaboration is an
explicit non-goal.

The release artifact is a single static binary with the frontend
bundle embedded via rust-embed.

## Layout

```
crates/
  chan          the binary. Parses CLI args, dispatches subcommands,
                mounts the embedded frontend. Self-upgrade lives in
                src/update.rs.
  chan-server   HTTP + WebSocket surface. Wraps chan-drive in axum
                routes; uses chan-llm for assistant routes. Per-area
                handlers live in src/routes/{drive, files, search,
                graph, llm, sessions, attachments, storage,
                preferences, contacts, build_info, ws, health}.rs;
                lib.rs holds ServeConfig + build_app + serve +
                serve_via_tunnel + router(). Top-level modules:
                auth, bus, cli_resolve, config, embed_seed, error,
                indexer, mcp_bridge, preferences, qr, self_writes,
                signal, state, static_assets, store, util.
  fetch-models  build helper. Pre-fetches the BGE-small embedding
                model into chan-server/resources/ so release builds
                bundle it. Run via `make models`; not invoked by
                `cargo build` directly.

web/            Svelte frontend, embedded into the binary at build
                time via rust-embed.
```

One sibling repo, pulled in as a path dep:

- `chan-writer/chan-core` is a Cargo workspace hosting
  `chan-drive` (filesystem, search, graph, drive registry),
  `chan-llm` (LLM backends, embedded prompts, tool sandbox,
  key resolution), and `chan-tunnel-{proto,client,server}`
  (h2/yamux drive tunnel). chan and chan-server pull in
  `chan-drive`, `chan-llm`, and `chan-tunnel-client` as path
  deps. Native shells (iOS / Android) link `chan-drive` and
  `chan-llm` via uniffi without dragging in this repo's HTTP
  stack.

We depend on the chan-core workspace via sibling-checkout path
deps; switch to git or crates.io when the repos go public.

## Build & Test

```bash
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The Rust toolchain is pinned in `rust-toolchain.toml` (1.95.0).
`cargo` auto-installs through rustup on first use, so contributor
and CI clippy lint sets stay locked together. The pre-push hook
(`./scripts/install-hooks` to install) runs the same gate as CI
under the pinned compiler with `RUSTFLAGS=-D warnings` plus
`cargo build --no-default-features`. Bumping Rust = edit
`rust-toolchain.toml` and fix any new clippy findings in the
same commit.

## Project Principles

### Drive is the boundary

All filesystem operations route through `chan_drive::Drive`,
which sandboxes paths under the registered drive root, refuses
non-regular files (symlinks, FIFOs, sockets, devices), and
performs atomic writes. Nothing in this repo should ever call
`std::fs::*` on user content directly.

### Single binary, no runtime deps

No Node.js, no Python, no native daemons at runtime. The frontend
embeds at build time. New dependencies must hold this line.

### Local-first by default, opt-in tunnel

The HTTP server binds `127.0.0.1` by default. Auth is a per-launch
bearer token printed once on stderr and appended to the launch URL.
No TLS at the local hop.

Tunnel mode (`chan serve --tunnel-token ...`, or `CHAN_TUNNEL_TOKEN`
env var) replaces the local listener with a `chan-tunnel-client`
dial to `drive.chan.app/v1/tunnel`. The drive is then published at
`{user}.drive.chan.app/{drive}/*` over yamux substreams. The
single-user, single-machine assumption still holds: one chan serve
process owns the drive's writes; the tunnel just relocates the
inbound transport. The bearer-token gate is auto-disabled in tunnel
mode (the gateway in front of drive.chan.app is the trust boundary;
default behavior 404s anonymous visitors, opt out with
`--tunnel-public`). Wire protocol lives in
`../chan-core/crates/chan-tunnel-proto`.

### App-level vs core

What lives in chan-drive (filesystem, search, graph) vs what
lives here (HTTP, editor preferences, sessions, attachments) is
a hard line. Don't push library concerns into chan-drive, and
don't reimplement library primitives here. When in doubt, read
`../chan-core/crates/chan-drive/design.md`.

## Writing Rules

- **No em dashes** in comments or documentation.
- **Tables**: pure ASCII, target 80 columns.
- **Factual**: no marketing language. Include analysis with
  benchmarks; explain whether numbers meet expectations.
- **Comments**: explain WHY, not WHAT.

## Contributor Patterns

- **Atomic writes via chan-drive**: every user-content write
  goes through `Drive::write_text` or `Drive::write_bytes`.
  These enforce the editable-text gate, the path sandbox, and
  the special-file refusal. Don't bypass.
- **Subcommand parity**: every chan subcommand has a clap
  definition + a `cmd_*` function in `crates/chan/src/main.rs`.
  Help text must reflect actual behavior; don't claim env vars
  or flags that don't exist.
- **Server routes go in chan-server**: never inline an axum
  handler inside the binary crate. The `chan` crate parses args
  and calls `chan_server::serve`. New routes belong in the
  matching `crates/chan-server/src/routes/<area>.rs`; cross-area
  shared types live in the module that owns them. `lib.rs::router()`
  is the only place the route table is assembled.
- **App-level config files**: anything new under `<config>/chan/`
  goes through `crate::store::{load_toml, save_toml}` so atomic
  writes + parent-dir fsync match the rest of the app. Don't roll
  a fresh `tempfile + rename` by hand.
- **LLM lives in chan-llm**: backends, tools, prompts, and key
  resolution all live in the chan-llm crate (chan-core
  workspace). chan-server's /api/llm/* routes wrap
  `chan_llm::LlmSession`. The `chan` binary never directly
  invokes a backend.
- **Pinned toolchain**: do not introduce code that requires a
  newer Rust than `rust-toolchain.toml` declares without bumping
  the pin in the same commit.

## Documentation

- **Design and architecture**: [`design.md`](design.md). Single
  load-bearing reference for the workspace layout and the
  chan-drive contract.
- **chan-drive design**: `../chan-core/crates/chan-drive/design.md`.
  Read before proposing chan-drive changes.
- **Issue tracker**: GitHub repo `chan-writer/chan`.
