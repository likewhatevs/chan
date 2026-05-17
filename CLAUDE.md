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
  chan                  the binary. Parses CLI args, dispatches
                        subcommands, mounts the embedded frontend.
                        Self-upgrade lives in src/update.rs.
  chan-server           HTTP + WebSocket surface. Wraps chan-drive
                        in axum routes; exposes the in-process MCP
                        server over a Unix-domain socket. Per-area
                        handlers live in src/routes/{drive, files,
                        search, graph, fs_graph, report, sessions,
                        attachments, storage, preferences, contacts,
                        build_info, terminal, ws, health}.rs;
                        lib.rs holds ServeConfig + build_app + serve
                        + serve_via_tunnel + router(). Top-level
                        modules: auth, bus, config, embed_seed,
                        error, indexer, mcp_bridge, preferences,
                        qr, self_writes, signal, state,
                        static_assets, store, tunnel_guard, util.
  chan-drive            filesystem boundary, drive registry, search
                        + graph indexer, watch, report engine. The
                        only crate that touches user content on
                        disk.
  chan-llm              MCP-only library after Phase 5: the chan
                        MCP `Server`, tool schemas, embedded prompt
                        text, and the MCP key/config plumbing.
                        chan-server consumes only
                        `chan_llm::mcp::Server` via
                        `crates/chan-server/src/mcp_bridge.rs`.
  chan-report           report engine shared with chan-drive.
  chan-tunnel-{proto,
    client, server}     h2/yamux drive tunnel. chan-server pulls
                        chan-tunnel-client; the standalone tunnel
                        server lives next door for the cloud side.
  fetch-models          build helper. Pre-fetches the BGE-small
                        embedding model into chan-server/resources/
                        so release builds bundle it. Run via
                        `make models`; not invoked by `cargo build`
                        directly.

web/                    Svelte frontend, embedded into the binary
                        at build time via rust-embed.

desktop/                Tauri shell. Cross-platform desktop wrapper
                        (`chan-desktop`) that launches `chan serve`
                        per drive and mounts the editor in a
                        webview window. Per-window state is keyed
                        by a `w=<window-label>` URL parameter.
```

Phase 5 collapsed the chan-core sibling workspace into this repo:
chan-drive, chan-llm, chan-report, and the three chan-tunnel-*
crates are all workspace members here. Native shells (iOS / Android)
still link `chan-drive` via uniffi without dragging in this repo's
HTTP stack.

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

## Test Server Workflow

When the user asks for a test server (e.g. "spin up a test
server", "let's try this in the browser"):

1. **Ask first**: new drive under `/tmp/chan-test-<something>`,
   or reuse an existing registered one? `chan list` shows the
   options. For a new drive, also ask what to seed it with
   (empty, a few sample notes, copy of an existing tree).
2. **Build + launch**: `cargo build -p chan` rebuilds the binary
   with the current `web/dist/` bundle, then
   `./target/debug/chan serve <path>` in the background. The URL
   with the per-launch bearer token lands on stderr.
3. **Reload on frontend changes**: rust-embed bakes the bundle
   in at compile time, so every web edit needs the full cycle:
   stop the server, `npm run build` in `web/`, `cargo build -p
   chan`, restart. There is no hot reload. A stale browser tab
   also needs a hard reload to pick up the new hashed bundle
   filenames.
4. **Tear down**: stop the server process, `rm -rf` the temp
   drive directory if it was a throwaway, then `chan remove
   <path>` to drop the registry entry. `chan remove` takes the
   path, not the display name.

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
`crates/chan-tunnel-proto`.

### App-level vs core

What lives in chan-drive (filesystem, search, graph, watch, report)
vs what lives in chan-server (HTTP, editor preferences, sessions,
attachments, terminal, MCP bridge) is a hard line. Don't push
library concerns into chan-drive, and don't reimplement library
primitives in chan-server. When in doubt, read
`crates/chan-drive/design.md`.

### MCP server only, no in-app agent

Phase 5 removed the in-app Agent overlay and the chan-server
`/api/llm/*` / `/api/assistant/*` HTTP surface. External agents
(claude, codex, gemini) connect through the in-process MCP server
exposed over a Unix-domain socket by `mcp_bridge.rs`; the embedded
terminal exports `CHAN_MCP_SERVER_JSON` and companion `CHAN_MCP_*`
discovery variables. Chan does not write CLI-owned env namespaces;
tools can translate the `CHAN_` descriptor into their own MCP config
shape. Do not reintroduce in-app agent UI or chan-server-side chat
APIs without a phase-level decision.

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
- **chan-llm is MCP-only**: after Phase 5 the crate exposes the
  chan MCP `Server`, its tool schemas, embedded prompts, and key
  resolution. There is no in-app agent session and no CLI
  backend wrappers; external agents connect through the in-process
  MCP server in `crates/chan-server/src/mcp_bridge.rs`.
- **Pinned toolchain**: do not introduce code that requires a
  newer Rust than `rust-toolchain.toml` declares without bumping
  the pin in the same commit.

## Documentation

- **Design and architecture**: [`design.md`](design.md). Single
  load-bearing reference for the workspace layout and the
  chan-drive contract.
- **chan-drive design**: [`crates/chan-drive/design.md`](crates/chan-drive/design.md).
  Read before proposing chan-drive changes.
- **chan-tunnel-proto design**:
  [`crates/chan-tunnel-proto/design.md`](crates/chan-tunnel-proto/design.md).
- **Issue tracker**: GitHub repo `chan-writer/chan`.
