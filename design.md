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
  chan           binary. CLI + dispatch into subcommands; embeds the
                 frontend via chan-server's rust-embed bundle.
  chan-server    HTTP + WebSocket surface. Wraps chan-drive in axum
                 routes; consumes chan-llm for assistant routes.
  fetch-models   build helper. Pre-fetches the default embedding
                 model into chan-server's resources/ so release
                 builds bundle it. Not invoked by `cargo build`.

web/             Svelte frontend, embedded into the binary at build
                 time via rust-embed.
```

One sibling repo, depended on as path deps:

- `chan-writer/chan-core` is a Cargo workspace with the filesystem
  / search / graph layer (`chan-drive`), the LLM layer
  (`chan-llm`), and the tunnel transport
  (`chan-tunnel-{proto,client,server}`). chan and chan-server pull
  in `chan-drive`, `chan-llm`, `chan-tunnel-client`, and
  `chan-tunnel-proto` as path deps. The drive + LLM split keeps
  app-level HTTP / frontend concerns out of those crates so native
  shells (iOS / Android, future) can link them via uniffi without
  dragging in this repo's axum / tower / reqwest stack.

The path deps assume a sibling-checkout layout. Switch to git or
crates.io when the repos go public.

## Crate responsibilities

### chan (binary)

Owns: argument parsing (clap), tracing init, dispatch into
subcommands. Calls `chan_drive::Library` for registry mutations
and `chan_drive::Drive` for per-drive operations. Calls
`chan_server::serve` (or `serve_via_tunnel`) for `chan serve`.
Self-upgrade flow lives in `crates/chan/src/update.rs`. No HTTP
routes, no LLM code, no filesystem access outside chan-drive.

The binary also exposes two hidden subcommands the assistant
backends spawn: `chan __mcp <drive-root>` runs chan-llm's MCP
server on stdio; `chan __mcp-proxy <socket>` is a stdio bridge
into the in-process MCP server hosted by a running `chan serve`.

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
socket, model-bundle seeding. Depends on `chan-drive` for
filesystem + search + graph + watch primitives, on `chan-llm` for
the assistant routes, and on `chan-tunnel-client` for tunnel
transport.

Module layout (`crates/chan-server/src/`):

```
auth.rs          per-launch bearer token + axum middleware
bus.rs           watcher / LLM event bridges into the WS broadcast
cli_resolve.rs   resolve claude / gemini binaries from PATH + fallbacks
config.rs        ServerConfig (server.toml)
embed_seed.rs    extract the baked-in model bundle on first launch
error.rs         Error + err_*() response builders
indexer.rs       background search/graph indexer (boot + per-event)
mcp_bridge.rs    Unix-socket MCP server for agent subprocesses
preferences.rs   EditorPrefs (preferences.toml)
qr.rs            terminal QR for the launch banner
self_writes.rs   suppress watcher events that echo our own writes
signal.rs        SIGINT/SIGTERM + idle-timeout watchers; clock
state.rs         AppState, DriveCell
static_assets.rs WebAssets (rust-embed) + SPA fallback
store.rs         shared atomic load/save for TOML configs
util.rs          slug, h1, timestamp, opaque-JSON helpers
lib.rs           ServeConfig, sanitize_prefix, build_app, serve,
                 serve_via_tunnel, router

routes/
  attachments.rs   POST /api/attachments (multipart upload)
  build_info.rs    GET /api/build-info
  drive.rs         GET/PATCH /api/drive, GET /api/cloud-drives
  files.rs         /api/files, /api/files/*path, /api/move
  graph.rs         /api/links, /api/graph, /api/backlinks/*path,
                   /api/link-targets, /api/resolve-link, /api/headings
  health.rs        GET /api/health
  llm.rs           /api/llm/* (status, tools, complete, keys, models)
  preferences.rs   /api/server/config + /api/config (unified view)
  search.rs        /api/search/{files,content}, /api/index/*
  sessions.rs      /api/session*, /api/assistant/conversation*,
                   /api/answers
  storage.rs       POST /api/storage/reset
  ws.rs            GET /ws (watcher + LLM streaming side channel)
```

### chan-llm (in chan-core workspace)

Owns: LLM backends (Anthropic, Gemini, Ollama, claude_cli,
gemini_cli), embedded prompts, the tool execution sandbox
(`read_file`, `write_file`, `list_files`, `search_content`
implemented against `chan-drive`), and key resolution (env / OS
keychain / `<config>/chan/api-keys.toml`). Tool reads / writes
always go through `chan_drive::Drive` so the filesystem gates
apply.

chan-server is one of two consumer shapes:

- chan-server (here) wraps `LlmSession` in axum routes and
  forwards `SessionListener` events over WebSocket.
- Native shells (iOS / Android, future) link chan-llm via uniffi
  alongside chan-drive and implement `SessionListener` in
  Swift / Kotlin.

The agentic backends (claude_cli, gemini_cli) launch the local
CLIs as subprocesses and route the agent's file edits through an
MCP server. chan-server hosts that MCP server in-process behind a
Unix-domain socket; the agent's MCP child connects via `chan
__mcp-proxy`, which is just a stdio<->socket pipe. This sidesteps
chan-drive's per-drive flock that would otherwise reject the
child's `Library::open_drive`.

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
  (`drive.chan.app/{user}/{drive}` is stripped by the gateway
  before forwarding into the tunnel substream), but the SPA still
  needs to know the public path so its API URLs resolve from the
  browser's origin. On `chan_tunnel_client::TunnelEvent::Connected`
  the server swaps the same `chan-prefix` meta value in.

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
  listener. The tunnel client dials `tunnel.chan.app`, runs a
  Hello/HelloAck handshake that names the drive, and serves
  yamux substreams with the router. The bearer token is forced
  off in tunnel mode: `drive.chan.app/{user}/{drive}` itself is
  the trust boundary (default behavior bounces anonymous visitors
  to id.chan.app; `--tunnel-public` opts out of that gate).

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
| Assistant chat history             | chan-drive (storage), chan-server (HTTP) |
| Attachments / answers dirs         | chan-server |
| LLM backends + tools + keys        | chan-llm    |
| MCP server (in-proc + bridge)      | chan-llm + chan-server |
| Tunnel transport                   | chan-tunnel-client (chan-core) |
| Self-upgrade flow                  | chan binary |

The split keeps app-level concerns (HTTP, WebSocket, frontend
bundle, editor preferences, assistant chat persistence) out of
chan-drive so native shells can link the drive layer via uniffi
without dragging in axum / reqwest / the rest of the HTTP stack.

The Tauri desktop / mobile shells are parked. They get
`chan-writer/chan-desktop` (or similar) when the time comes and
link `chan-drive` via uniffi instead of going through the HTTP
server.
