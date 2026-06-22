# chan

An AI-native IDE for the modern engineer. `chan` is a single static binary that bundles a CLI and a local HTTP server; the server serves a hybrid workspace (editor, terminal, multi-agent Team Work, file browser, graph, dashboard) of tiling tabs and panes over a folder on disk.

Modern engineers drive projects in Markdown, so `chan` is built for it: write your design docs, specs, and tasks, then put AI to work on them. Agents create, review, refine, and harden that work and then execute it. Multiple agents (Claude, Codex, Gemini) run in the embedded terminal and coordinate with each other through `chan`'s `cs` tooling and the in-process MCP server. Cross-file `[[wiki-link]]` autocomplete, BM25 + embedding hybrid search, a workspace graph, and code reports are built in.

Single-user, single-machine. The HTTP server binds loopback by default. An opt-in tunnel reaches the same workspace from another device: `chan serve` publishes a workspace over an outbound tunnel to a gateway, or chan-desktop connects to a remote `chan serve` directly over HTTP/2. The tunnel's server side ships in this repo under `gateway/`, so you can self-host the whole path; the maintainer's own deployment at `workspace.chan.app` is experimental, with sign-in off by default, and is not the product.

## Quickstart

Install the CLI, point it at any git repo (or clone chan's own), and the IDE opens in your browser:

```bash
# 1. Install the standalone chan CLI (Linux x86_64/aarch64, macOS aarch64).
curl -fsSL https://chan.app/install.sh | sh

# 2. Open any existing git repo, or clone chan's source to try it out.
git clone https://github.com/fiorix/chan
chan open ./chan
```

`chan open` starts a loopback server, prints a URL carrying a per-launch bearer token on stderr, and opens your default browser. From there you drive the workspace like it is your own machine: editor, terminal, Team Work, file browser, graph, and dashboard as tiling tabs and panes.

The in-browser experience is the full IDE. Browser keyboard shortcuts are constrained by the browser itself, so the keyboard story is suboptimal there but still powerful: see Hybrid Nav (`Cmd+.`) for keyboard-driven navigation. For a native window and remote-attach support (macOS `.app`, Linux AppImage), use chan-desktop; see ["Reach a workspace remotely"](#reach-a-workspace-remotely) below and the [manual](https://chan.app/manual/).

## Layout

```
crates/
  chan           binary. CLI + dispatch; also answers as `cs` behind
                 a user-created symlink.
  chan-workspace filesystem, search, and graph primitives.
  chan-llm       MCP server/tool sandbox used to expose a workspace to
                 terminal-launched agent CLIs.
  chan-report    language/SLOC/COCOMO report support.
  chan-server    HTTP + WebSocket surface; embeds the web bundle.
  chan-shell     the `cs` terminal-control surface and its
                 control-socket wire types.
  chan-tunnel-*  tunnel protocol, client, and server libraries.
  fetch-models   build helper for the embedding-model bundle.

desktop/         Tauri desktop shell. `desktop/src-tauri` is a
                 workspace member; the app root stays at `desktop/`
                 so Tauri's frontend paths remain conventional.

web/             Svelte frontend, embedded into the binary at build
                 time via rust-embed.

gateway/         self-hostable tunnel gateway (identity, workspace
                 proxy, admin CLI); a nested Cargo workspace of its
                 own.
```

See [`design.md`](design.md) for the architecture and how the frontend embeds into the binary.

## Build

```bash
make                # frontend bundle + debug binary
make build-release  # frontend + embedded model bundle + release binary
make install        # copy target/release/chan to PREFIX/bin
make dev            # run `chan open /tmp/chan-dev --no-token`
```

`make help` lists every target. Manual `cargo` / `npm` calls still work; the Makefile is just a shortcut.

`make install` defaults `PREFIX=$XDG_BIN_HOME` (or `$HOME/.local`), so the binary lands in `~/.local/bin/chan` without sudo. Override for a system-wide install: `make install PREFIX=/usr/local`.

In debug builds, rust-embed reads files from `web/dist/` on each request, so a re-run of `make web` (or just `npm run build`) updates the served bundle without a `cargo build`. In release builds, the bundle is baked into the binary at compile time; `build.rs` re-links chan-server whenever any file under `web/dist/` changes.

The release build also bundles a pre-fetched embedding model (BGE-small, ~80 MB). `make build-release` runs `make models` first, which pre-populates `crates/chan-server/resources/models.tar.zst`; the seeder at first launch zstd-decodes + untars it into the per-machine cache. Plain `cargo build` ships an empty stub: at runtime the seeder downloads from HuggingFace as a fallback.

`HTTPS_PROXY` / `HTTP_PROXY` are honored everywhere chan reaches out (model fetch, self-upgrade probe).

Embedded terminal tabs start at the workspace root and export Chan MCP discovery variables when the server's MCP bridge is available: `CHAN_MCP_SERVER_NAME=chan`, `CHAN_MCP_SOCKET`, `CHAN_MCP_COMMAND`, `CHAN_MCP_COMMAND_JSON`, and `CHAN_MCP_SERVER_JSON`. Chan only writes its own `CHAN_` namespace; terminal-launched tools can translate that descriptor into their own CLI-specific MCP configuration.

## Run

```bash
chan workspace add ~/Notes              # register the workspace
chan open ~/Notes             # bind 127.0.0.1:8787 and open browser
```

The first launch prints the URL on stderr and opens the user's default browser. The URL carries a per-launch bearer token; the same token also accepts an `Authorization: Bearer ...` header. The token is persisted at `<state>/tokens/token` so a `cargo build && chan open` cycle does not invalidate the browser's cached sessionStorage token.

`chan --help` documents the full subcommand surface (serve, index, search, graph, status, config, metadata, reports, contacts, upgrade, ...) and every flag. The [manual](https://chan.app/manual/) covers day-to-day use: serve flags and workspace registration live in [workspaces](docs/manual/workspaces.md), self-upgrade in [upgrade and troubleshooting](docs/manual/upgrade-and-troubleshooting.md).

## Reach a workspace remotely

The tunnel is a core part of chan, not a hosted add-on. Instead of binding a local port, `chan serve` can publish a workspace over an outbound tunnel to a gateway that reverse-proxies it back to you: no inbound ports, no router config. chan-desktop can also open a remote `chan serve` directly over HTTP/2. A third path is `chan devserver`: one headless server on a box hosts many workspaces behind a single port, and chan-desktop attaches to it and lists them in their own group (reach it over `ssh -L`). See the [workspaces manual](docs/manual/workspaces.md).

The gateway is that server side, and it lives in this repo under `gateway/` for you to run yourself. `--tunnel-url` defaults to `https://workspace.chan.app/v1/tunnel`, the maintainer's own deployment of that code; it is experimental, with sign-in off by default. Commands and flags are in the [tunnel manual](docs/manual/tunnel.md); see [`gateway/README.md`](gateway/README.md) to stand up your own gateway.

## Contributing

Agents and contributors: start at [`.agents/README.md`](.agents/README.md).

The Rust toolchain is pinned in `rust-toolchain.toml` (1.95). Install [rustup](https://rustup.rs/); it picks up the pin automatically the first time you run `cargo` here.

Install the pre-push hook once per clone:

```
./scripts/install-hooks
```

The hook runs `make pre-push` before every push: rustfmt, clippy and the test suite with warnings denied, a no-default-features build, the gateway workspace build, and the web checks (svelte-check, vitest, production bundles), mirroring CI. A passing local push therefore will not fail in GitHub Actions.
