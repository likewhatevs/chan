# Agents

The canonical home for agent and contributor standards in this repo.
Read this file first, then follow the read order below.

## What This Project Is

`chan` is the user-facing AI-native IDE for the modern engineer: a CLI
plus an HTTP server that serves an embedded hybrid workspace (editor,
terminal, Team Work, file browser, graph, dashboard) over a folder on
disk. You drive projects in Markdown and put AI to work on them;
agents run in the terminal and coordinate through `cs` and the
in-process MCP server. The CLI subcommands manage the workspace
registry, workspace contents, search, and the running server. The
server is loopback-only, single-user, single-machine.

The release artifact is a single static binary with the frontend
bundle embedded via rust-embed.

## Read Order

1. [principles.md](principles.md) - the load-bearing project invariants.
2. [writing-rules.md](writing-rules.md) - documentation and comment style.
3. [patterns.md](patterns.md) - contributor patterns for code changes.
4. [roster/README.md](roster/README.md) - agent roster and contact cards.
5. [playbook.md](playbook.md) - cross-phase operational lessons.
6. [skills/](skills/) - executable workflows (test server, release, gate)
   plus vendored general skill profiles.

Subsystem guides: [desktop.md](desktop.md) (chan-desktop),
[gateway.md](gateway.md) (the cloud gateway workspace).

## Layout

```
crates/
  chan                  the binary. Parses CLI args, dispatches
                        subcommands, mounts the embedded frontend.
                        Self-upgrade lives in src/update.rs.
  chan-server           HTTP + WebSocket surface. Wraps chan-workspace
                        in axum routes; exposes the in-process MCP
                        server over a Unix-domain socket. Per-area
                        handlers live in src/routes/{workspace, files,
                        search, graph, fs_graph, report, sessions,
                        attachments, storage, preferences, contacts,
                        build_info, terminal, ws, health}.rs;
                        lib.rs holds ServeConfig + build_app + serve
                        + serve_via_tunnel + router(). Top-level
                        modules: auth, bus, config, embed_seed,
                        error, host, indexer, mcp_bridge, preferences,
                        qr, self_writes, signal, state,
                        static_assets, store, tunnel_guard, util.
  chan-workspace        filesystem boundary, workspace registry, search
                        + graph indexer, watch, report engine. The
                        only crate that touches user content on
                        disk.
  chan-llm              MCP-only library after Phase 5: the chan
                        MCP `Server`, tool schemas, embedded prompt
                        text, and the MCP key/config plumbing.
                        chan-server consumes only
                        `chan_llm::mcp::Server` via
                        `crates/chan-server/src/mcp_bridge.rs`.
  chan-report           report engine shared with chan-workspace.
  chan-tunnel-{proto,
    client, server}     h2/yamux workspace tunnel. chan-server pulls
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
                        (`chan-desktop`) that embeds chan-server for
                        normal local workspaces and mounts the editor
                        in a webview window. Remote workspaces are
                        explicit attach modes, not local fallback
                        behavior. Per-window state is keyed by a
                        `w=<window-label>` URL parameter. Agent guide:
                        .agents/desktop.md.

gateway/                Account / sign-in / reverse-proxy surface for
                        chan.app (id.chan.app + workspace.chan.app).
                        Separate nested Cargo workspace (profile,
                        identity, workspace-proxy, admin,
                        gateway-common); Postgres-backed, linux amd64/
                        arm64 only. NOT a member of the root workspace,
                        so the core build stays Postgres-free. Agent
                        guide: .agents/gateway.md.
```

Phase 5 collapsed the chan-core sibling workspace into this repo:
chan-workspace, chan-llm, chan-report, and the three chan-tunnel-*
crates are all workspace members here. Native shells (iOS / Android)
still link `chan-workspace` via uniffi without dragging in this repo's
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
same commit. See [skills/gate/SKILL.md](skills/gate/SKILL.md) for the
full pre-push gate and the isolated/own-gate model.

## Documentation

- **Design and architecture**: [`design.md`](../design.md). Single
  load-bearing reference for the workspace layout and the
  chan-workspace contract.
- **chan-workspace design**:
  [`crates/chan-workspace/design.md`](../crates/chan-workspace/design.md).
  Read before proposing chan-workspace changes.
- **chan-tunnel-proto design**:
  [`crates/chan-tunnel-proto/design.md`](../crates/chan-tunnel-proto/design.md).
- **Issue tracker**: GitHub repo `fiorix/chan`.
