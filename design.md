# chan: design

`chan` is the user-facing notes app: a CLI plus an HTTP server that
serves a Svelte WYSIWYG editor for plain markdown drives. This
document is the canonical design reference for the workspace.
Update it in the same commit as any change that affects crate
boundaries, the module layout under `chan-server`, the on-disk
layout, or the frontend embed / serve story.

## Workspace layout

```
crates/
  chan                  binary. CLI + dispatch into subcommands;
                        embeds the frontend via chan-server's
                        rust-embed bundle.
  chan-server           HTTP + WebSocket surface. Wraps chan-drive
                        in axum routes; hosts the in-process MCP
                        server over a Unix-domain socket.
  chan-drive            filesystem boundary, drive registry, search
                        + graph indexer, watch, report engine.
  chan-llm              MCP-only library: chan MCP server, tool
                        schemas, embedded prompts, key resolution.
  chan-report           report engine shared with chan-drive.
  chan-tunnel-{proto,
    client, server}     h2/yamux drive tunnel: wire protocol, the
                        client chan-server dials, and the
                        standalone server hosted near the gateway.
  fetch-models          build helper. Pre-fetches the default
                        embedding model into chan-server's
                        resources/ so release builds bundle it.
                        Not invoked by `cargo build`.

web/                    Svelte frontend, embedded into the binary
                        at build time via rust-embed.

desktop/                Tauri shell (`chan-desktop`). Embeds
                        chan-server for normal local drives and
                        renders the editor in a webview window.
                        Remote drives are explicit attach modes,
                        not local fallback behavior. Per-window
                        state is keyed by `w=<window-label>`.
```

Phase 5 collapsed the historical `chan-writer/chan-core` sibling
workspace into this repo: chan-drive, chan-llm, chan-report, and
the three chan-tunnel-* crates are workspace members here, not
path deps. The drive split still keeps app-level HTTP / frontend
concerns out of chan-drive / chan-llm so native shells (iOS /
Android, future) can link `chan-drive` via uniffi without
dragging in this repo's axum / tower / reqwest stack.

## Crate responsibilities

### chan (binary)

Owns: argument parsing (clap), tracing init, dispatch into
subcommands. Calls `chan_drive::Library` for registry mutations
and `chan_drive::Drive` for per-drive operations. Calls
`chan_server::serve` (or `serve_via_tunnel`) for `chan serve`.
Self-upgrade flow lives in `crates/chan/src/update.rs`. No HTTP
routes, no LLM code, no filesystem access outside chan-drive.

The binary also exposes two hidden MCP subcommands that external
agent CLIs invoke through environment variables exported by the
embedded terminal: `chan __mcp <drive-root>` runs chan-llm's MCP
server on stdio (used when no running `chan serve` is reachable);
`chan __mcp-proxy <socket>` is a stdio bridge into the in-process
MCP server hosted by a running `chan serve`. The embedded terminal
exports `CHAN_MCP_SERVER_JSON` and companion `CHAN_MCP_*`
discovery variables. Chan deliberately avoids CLI-owned env
namespaces such as `CLAUDE_`, `CODEX_`, and `GEMINI_`; external
tools can translate the `CHAN_` descriptor into their own MCP
configuration.

Subcommand surface today:

```
chan add PATH [--name NAME]
chan list
chan remove PATH
chan rename PATH NAME
chan serve [PATH] [--host ...] [--port ...] [--prefix ...] ...
chan index PATH
chan search PATH QUERY [--limit N]
chan upgrade [-y] [--check] [--version V]
chan contacts import csv FILE --into DIR
                              [--provider google] [--dry-run]
                              [--overwrite] [--drive PATH]
```

`chan contacts import csv` parses a Google Contacts CSV and
writes one markdown note per contact under `--into` (drive-
relative). Notes carry `chan.kind: contact` frontmatter so the
graph builder and editor `@` picker can classify them without a
separate index. The orchestrator lives on `chan-drive`
(`Drive::import_contacts`); this binary just plumbs flags and
prints a per-row summary table. Re-running either skips
existing files (default) or overwrites (`--overwrite`).

### chan-server

Owns: HTTP + WebSocket routes, per-launch token auth middleware,
embedded-frontend serving (rust-embed), background indexer +
watcher subscription, in-process MCP bridge over a Unix-domain
socket, embedded terminal PTY (with MCP env exposure), model-
bundle seeding. Depends on `chan-drive` for filesystem + search
+ graph + watch primitives, on `chan-llm` for the MCP server,
and on `chan-tunnel-client` for tunnel transport.

Module layout (`crates/chan-server/src/`):

```
auth.rs          per-launch bearer token + axum middleware
bus.rs           watcher event bridge into the WS broadcast
config.rs        ServerConfig (server.toml)
embed_seed.rs    extract the baked-in model bundle on first launch
error.rs         Error + err_*() response builders
host.rs          in-process multi-drive host runtime
indexer.rs       background search/graph indexer (boot + per-event)
mcp_bridge.rs    Unix-socket MCP server for external agent CLIs
preferences.rs   EditorPrefs (preferences.toml)
qr.rs            terminal QR for the launch banner
self_writes.rs   suppress watcher events that echo our own writes
signal.rs        SIGINT/SIGTERM + idle-timeout watchers; clock
state.rs         AppState, DriveCell
static_assets.rs WebAssets (rust-embed) + SPA fallback
store.rs         shared atomic load/save for TOML configs
tunnel_guard.rs  middleware refusing settings writes in --tunnel-public
util.rs          slug, h1, timestamp, opaque-JSON helpers
lib.rs           ServeConfig, sanitize_prefix, build_app, serve,
                 serve_via_tunnel, router

routes/
  attachments.rs   POST /api/attachments (multipart upload)
  build_info.rs    GET /api/build-info
  contacts.rs      POST /api/contacts/import (multipart CSV)
  drive.rs         GET/PATCH /api/drive, GET /api/cloud-drives
  files.rs         /api/files, /api/files/*path, /api/move.
                   Editable file opens support JSON reads and
                   NDJSON streaming reads through
                   `GET /api/files/*path?stream=1`.
  fs_graph.rs      GET /api/fs-graph (filesystem-shaped scopes)
  graph.rs         /api/links, /api/graph, /api/graph/languages,
                   /api/backlinks/*path, /api/link-targets,
                   /api/resolve-link, /api/headings
  health.rs        GET /api/health
  preferences.rs   /api/server/config + /api/config (unified view)
  report.rs        /api/report/{file,prefix}
  search.rs        /api/search/{files,content}, /api/index/*
  sessions.rs      /api/session* (per-window editor session blob)
  storage.rs       POST /api/storage/reset
  terminal.rs      GET /api/terminal/ws (PTY WebSocket; exports
                   CHAN_MCP_* env by default)
  ws.rs            GET /ws (watcher side channel)
```

### chan-llm

Owns: the chan MCP server (`chan_llm::mcp::Server`), tool schemas
exposed over MCP, embedded prompt text, and MCP key resolution.
Tool reads / writes always go through `chan_drive::Drive` so the
filesystem gates apply.

Phase 5 narrowed chan-llm to this MCP-only surface. The in-app
`LlmSession`, CLI backends (`claude_cli`, `codex_cli`,
`gemini_cli`), and their associated tool-loop and listener
plumbing were removed when the in-app Agent overlay was deleted.
External agent CLIs (claude, codex, gemini) connect to the chan
MCP server by reading the `CHAN_MCP_*` environment variables the
embedded terminal exports and translating them to their own MCP
configuration.

chan-server hosts the MCP server in-process behind a Unix-domain
socket (`crates/chan-server/src/mcp_bridge.rs`). External
subprocesses connect via `chan __mcp-proxy <socket>`, which is a
stdio<->socket pipe. This sidesteps chan-drive's per-drive flock
that would otherwise reject a child's `Library::open_drive`.

## Frontend embed: build, serve, prefix

The frontend lives under `web/` (Svelte + Vite + Tailwind) and
ships as a build artifact under `web/dist/`. It is consumed by
chan-server through rust-embed:

- Debug build: rust-embed reads files from `web/dist/` on every
  request. `make web` (or `npm run build` directly) is enough to
  see updates without a cargo rebuild.
- Release build: the entire `web/dist/` tree is baked into the
  binary at compile time. `crates/chan-server/build.rs` emits
  `cargo:rerun-if-changed=...` for every file under `web/dist/` so
  a re-bundled frontend triggers a relink.

Vite is configured with `base: "./"` so asset URLs in the bundle
are relative to whatever path the SPA shell is loaded from. That
matters for two paths:

- `--prefix /seg`: a reverse proxy can mount many `chan serve`
  instances under one host, e.g.
  `drive.example.com/{user}/`. The router is `Router::new().nest(prefix, inner)`,
  and every `index.html` response gets a
  `<meta name="chan-prefix" content="/seg">` injected after the
  `<head>` tag (`static_assets::inject_chan_prefix`). The frontend
  reads that meta tag at boot and prepends the prefix to every
  fetch and WebSocket URL.
- Tunnel mode: chan-server runs at root inside the tunnel
  (`{user}.drive.chan.app/{drive}` is stripped by the gateway
  before forwarding into the tunnel substream; the upstream sees
  `/`, `/assets/...`), but the SPA still needs to know the public
  path so its API URLs resolve from the browser's origin. On
  `chan_tunnel_client::TunnelEvent::Connected` the server swaps
  the same `chan-prefix` meta value in.

Single-page-app fallback: any path that isn't an `/api` route, a
`/ws` upgrade, or a baked asset returns `index.html` so
client-side routes work. Misses on `/api/*` and `/ws` return real
404s instead of the SPA shell so callers don't silently get HTML
when they expected JSON.

## Bind vs tunnel

`chan serve` uses one of two transports:

- Local bind (default): `axum::serve(TcpListener, app)` on
  127.0.0.1 (or `--host` / `-6`). Per-launch bearer token gates
  every `/api/*` and `/ws` route, accepted as `?t=TOKEN` or
  `Authorization: Bearer TOKEN`. No TLS; loopback bind is the
  trust boundary.
- Tunnel (`CHAN_TUNNEL_TOKEN=...`): same axum router, but the
  transport is `chan_tunnel_client::run` instead of a TCP
  listener. The tunnel client dials
  `drive.chan.app/v1/tunnel`, runs a Hello/HelloAck handshake
  that names the drive, and serves yamux substreams with the
  router. The bearer token is forced off in tunnel mode:
  `{user}.drive.chan.app/{drive}/` is the trust boundary (default
  behavior 404s anonymous visitors; the drive owner opens the
  drive from id.chan.app's dashboard via a short-lived drive-gate
  token, drive-proxy validates and issues a host-only session
  cookie scoped to that drive; `--tunnel-public` opts out of that
  gate).

`build_app` produces the byte-identical axum app for both paths.
The two `serve*` functions differ only in transport, signal
wiring, and whether they bring up the launch banner / browser
handoff.

Both paths install signal watchers (SIGINT / SIGTERM on Unix,
Ctrl-C on Windows) that fire a single `tokio::sync::watch` channel
the server future drains on. A side task uses the same channel to
cancel any in-flight reindex so the runtime drop returns within at
most one file's worth of work. After the signal fires, both paths
race the server future against a 10-second grace timer and force
exit on grace expiry.

## On-disk layout

`chan-drive` owns the per-drive state and registry; this repo
inherits that layout unchanged. See
`../chan-core/crates/chan-drive/design.md`.

App-level state that lives outside chan-drive:

- Per-launch server token: `<state>/tokens/token` (mode 0600 on
  Unix). Atomic write through `chan_drive::fs_ops::atomic_write`.
- API keys (when stored on disk): `<config>/chan/api-keys.toml`,
  mode 0600. Env var and OS keychain take precedence.
- Editor preferences (fonts, theme, pane widths, line spacing,
  date format): `<config>/chan/preferences.toml`. Loaded at boot,
  mutated through `PATCH /api/config`.
- Server preferences (attachments_dir, answers_dir):
  `<config>/chan/server.toml`. Loaded at boot, mutated through
  `PATCH /api/server/config` (or via the unified `PATCH
  /api/config`).
- Update-check state: `<config>/chan/update-check.json`. Throttle
  + last-known-latest tag for the self-upgrade banner.
- MCP socket: `/tmp/chan-mcp-<pid>-<8 hex>.sock`. Created at boot,
  unlinked when `serve()` returns.

Both TOML config files round-trip through `crate::store::{load_toml,
save_toml}`, which write atomically through chan-drive's `fs_ops`
helper so the file + parent-dir fsync invariant matches user-content
writes.

## App-level vs chan-drive

| Concern                            | Lives in    |
|------------------------------------|-------------|
| Filesystem ops (read/write/list)   | chan-drive  |
| Path sandbox + special-file gates  | chan-drive  |
| Drive registry (`<config>/chan/`)  | chan-drive  |
| Search (tantivy BM25 + embeddings) | chan-drive  |
| Graph (sqlite)                     | chan-drive  |
| Filesystem watcher                 | chan-drive  |
| HTTP / WebSocket / SPA fallback    | chan-server |
| Per-launch auth token              | chan-server |
| Embedded frontend bundle           | chan-server |
| Editor preferences                 | chan-server |
| Server preferences                 | chan-server |
| Sessions / window layouts          | chan-drive (storage), chan-server (HTTP) |
| Attachments dir                    | chan-server |
| Embedded terminal PTY              | chan-server |
| MCP server (in-proc + bridge)      | chan-llm + chan-server |
| Tunnel transport                   | chan-tunnel-client |
| Self-upgrade flow                  | chan binary |

The split keeps app-level concerns (HTTP, WebSocket, frontend
bundle, editor preferences, terminal PTY) out of chan-drive so
native shells can link the drive layer via uniffi without
dragging in axum / reqwest / the rest of the HTTP stack.

The Tauri desktop / mobile shells are parked. They get
`chan-writer/chan-desktop` (or similar) when the time comes and
link `chan-drive` via uniffi instead of going through the HTTP
server.
