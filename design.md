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
  chan-server  HTTP + WebSocket surface. Wraps chan-core in axum
               routes.
  chan-llm     LLM backends (Anthropic, Gemini, Ollama) + the
               tool sandbox.

web/           Svelte frontend, embedded into the binary at build
               time. Wires in a follow-up commit.
```

`chan-core` (filesystem, search, graph, drive registry) lives at
the sibling repo `chan-writer/chan-core`. We depend on it as a
path dep that assumes a sibling checkout layout. Switch to git
or crates.io when the repos go public.

## Crate responsibilities

### chan (binary)

Owns: argument parsing (clap), tracing init, dispatch into
subcommands. Calls `chan_core::Library` for registry mutations and
`chan_core::Drive` for per-drive operations. Calls
`chan_server::serve` for `chan serve`. No HTTP routes, no LLM
code, no filesystem access outside chan-core.

### chan-server

Owns: HTTP + WebSocket routes, per-launch token auth middleware,
embedded-frontend serving (rust-embed), background indexer +
watcher subscription. Depends on `chan-core` for filesystem +
search + graph + watch primitives, and on `chan-llm` for the
assistant routes.

### chan-llm

Owns: LLM backends (Anthropic, Gemini, Ollama), tool execution
sandbox (`read_file`, `write_file`, `list_files`, `search_content`
implemented against `chan-core`), key resolution (env / OS
keychain via `keyring` crate / `~/.config/chan/api-keys.toml`).
Tool reads / writes always go through `chan-core::Drive` so the
filesystem gates apply.

## On-disk layout

`chan-core` owns the per-drive state and registry; this repo
inherits that layout unchanged. See `../chan-core/design.md`.

App-level state that lives outside chan-core:

- Per-launch server token: `<state>/tokens/<drive-key>`.
- API keys (when stored on disk): `~/.config/chan/api-keys.toml`,
  mode 0600. Env var and OS keychain take precedence.
- Editor preferences (fonts, theme, attachments dir, assistant
  backend selection): a separate global config the chan-server
  reads and writes via `/api/config`. Path TBD; not yet wired.

## App-level vs chan-core

| Concern                            | Lives in    |
|------------------------------------|-------------|
| Filesystem ops (read/write/list)   | chan-core   |
| Path sandbox + special-file gates  | chan-core   |
| Drive registry (~/.chan/...)       | chan-core   |
| Search (tantivy, BM25)             | chan-core   |
| Graph (sqlite)                     | chan-core   |
| Filesystem watcher                 | chan-core   |
| HTTP / WebSocket                   | chan-server |
| Per-launch auth token              | chan-server |
| Embedded frontend bundle           | chan-server |
| LLM backends + tools + keys        | chan-llm    |
| Editor preferences                 | chan-server |
| Sessions / window layouts          | chan-server |
| Assistant chat history             | chan-llm or chan-server |
| Attachments / answers dirs         | chan-server |

The split was set when chan-core was extracted from the old
`fiorix/chan/crates/chan-core`. Routes / LLM / preferences that
used to be in chan-core now live in this repo.

## Migration from `fiorix/chan`

The fiorix/chan repo holds the original `chan-cli` + `chan-core`
+ `chan-shared` + `chan-app` (Tauri shell) + `web/`. We migrate
in phases that each end with a working `chan` binary:

1. **Initial commit (this one)**: workspace skeleton.
   `chan add/list/remove/rename/index/search` work against the
   new chan-core. `chan serve` errors with "not implemented yet".
   `chan-server` and `chan-llm` are stubs that compile.
2. **Port routes from `fiorix/chan/crates/chan-core/src/server.rs`**
   into `chan-server` cluster by cluster:
   - Files routes (`/api/files`, `/api/move`).
   - Drive metadata (`/api/drive`, `/api/cloud-drives`).
   - Search (`/api/search/*`, `/api/index/*`).
   - Graph (`/api/graph`, `/api/links`).
   - Watcher WebSocket (`/ws`).
3. **Port LLM** into `chan-llm`:
   - Tool sandbox + key resolution + Anthropic / Gemini / Ollama
     backends.
   - Wire `/api/llm/*` and `/api/answers` and `/api/attachments`
     into chan-server.
4. **Port the frontend**: copy `web/` from fiorix/chan, wire
   rust-embed in the binary so `chan serve` ships a working
   editor.
5. **Sessions / preferences / assistant history**: app-level
   storage paths land here, not in chan-core.
6. **Deprecate fiorix/chan**: once feature-equivalent, archive
   the original repo.

The Tauri desktop / mobile shells are parked. They get
`chan-writer/chan-desktop` (or similar) when the time comes and
link `chan-core` via uniffi instead of going through the HTTP
server.

## What's NOT here yet

- Cross-repo CI auth between `chan-writer/chan` (private) and
  `chan-writer/chan-core` (private). The pre-push hook is the
  only quality gate at the moment; CI lands once we have a
  story for the auth (deploy key, PAT in secrets, or making
  chan-core public).
- Embedded frontend. `chan serve` errors out so the missing
  frontend is impossible to miss.
- HTTP server. Bare scaffold in `chan-server`; routes port in
  follow-up commits.
- LLM backends. Stub crate; backends port in follow-up commits.
