# chan: design

`chan` is the user-facing notes app: a CLI plus an HTTP server that
serves an embedded Svelte WYSIWYG editor for plain markdown drives.
This document is the canonical design reference for the workspace.
Update it in the same commit as any change that affects the crate
boundaries, the on-disk layout, or the migration plan.

## Workspace layout

```
crates/
  chan         the binary. Subcommands (add, list, remove, rename,
               serve, index, search). Embeds the frontend.
  chan-server  HTTP + WebSocket surface. Wraps chan-drive in axum
               routes; consumes chan-llm for assistant routes.

web/           Svelte frontend, embedded into the binary at build
               time. Wires in a follow-up commit.
```

One sibling repo, depended on as a path dep:

- `chan-writer/chan-core` is a Cargo workspace with the
  filesystem / search / graph layer (`chan-drive`), the LLM
  layer (`chan-llm`), and the tunnel transport
  (`chan-tunnel-{proto,client,server}`). chan and chan-server
  pull in `chan-drive`, `chan-llm`, and `chan-tunnel-client` as
  path deps. The drive + LLM split keeps app-level HTTP /
  frontend concerns out of those crates so native shells can
  link them via uniffi without dragging in this repo's axum /
  tower / reqwest stack.

The path deps assume a sibling-checkout layout. Switch to git
or crates.io when the repos go public.

## Crate responsibilities

### chan (binary)

Owns: argument parsing (clap), tracing init, dispatch into
subcommands. Calls `chan_drive::Library` for registry mutations
and `chan_drive::Drive` for per-drive operations. Calls
`chan_server::serve` for `chan serve`. No HTTP routes, no LLM
code, no filesystem access outside chan-drive.

### chan-server

Owns: HTTP + WebSocket routes, per-launch token auth middleware,
embedded-frontend serving (rust-embed), background indexer +
watcher subscription. Depends on `chan-drive` for filesystem +
search + graph + watch primitives, and on `chan-llm` for the
assistant routes.

### chan-llm (in chan-core workspace)

Owns: LLM backends (Anthropic, Gemini, Ollama, Claude CLI),
embedded prompts, the tool execution sandbox (`read_file`,
`write_file`, `list_files`, `search_content` implemented against
`chan-drive`), and key resolution (env / OS keychain /
`<config>/chan/llm.toml`). Tool reads / writes always go through
`chan_drive::Drive` so the filesystem gates apply.

chan-server is one of two consumer shapes:

  - chan-server (here) wraps `LlmSession` in axum routes and
    forwards `SessionListener` events over WebSocket.
  - Native shells (iOS / Android, future) link chan-llm via
    uniffi alongside chan-drive and implement `SessionListener`
    in Swift / Kotlin.

## On-disk layout

`chan-drive` owns the per-drive state and registry; this repo
inherits that layout unchanged. See
`../chan-core/crates/chan-drive/design.md`.

App-level state that lives outside chan-drive:

- Per-launch server token: `<state>/tokens/<drive-key>`.
- API keys (when stored on disk): `~/.config/chan/api-keys.toml`,
  mode 0600. Env var and OS keychain take precedence.
- Editor preferences (fonts, theme, attachments dir, assistant
  backend selection): a separate global config the chan-server
  reads and writes via `/api/config`. Path TBD; not yet wired.

## App-level vs chan-drive

| Concern                            | Lives in    |
|------------------------------------|-------------|
| Filesystem ops (read/write/list)   | chan-drive  |
| Path sandbox + special-file gates  | chan-drive  |
| Drive registry (~/.chan/...)       | chan-drive  |
| Search (tantivy, BM25)             | chan-drive  |
| Graph (sqlite)                     | chan-drive  |
| Filesystem watcher                 | chan-drive  |
| HTTP / WebSocket                   | chan-server |
| Per-launch auth token              | chan-server |
| Embedded frontend bundle           | chan-server |
| LLM backends + tools + keys        | chan-llm    |
| Editor preferences                 | chan-server |
| Sessions / window layouts          | chan-server |
| Assistant chat history             | chan-llm or chan-server |
| Attachments / answers dirs         | chan-server |

The split keeps app-level concerns (HTTP, WebSocket, frontend
bundle, editor preferences, assistant chat persistence) out of
chan-drive so native shells can link the drive layer via uniffi
without dragging in axum / reqwest / the rest of the HTTP stack.

The Tauri desktop / mobile shells are parked. They get
`chan-writer/chan-desktop` (or similar) when the time comes and
link `chan-drive` via uniffi instead of going through the HTTP
server.

## What's NOT here yet

- HTTP routes for sessions / answers / attachments / assistant
  conversation history. App-level persistence.
