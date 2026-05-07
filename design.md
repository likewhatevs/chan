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
               routes; consumes chan-llm for assistant routes.

web/           Svelte frontend, embedded into the binary at build
               time. Wires in a follow-up commit.
```

Two sibling repos depended on as path deps:

- `chan-writer/chan-core`: filesystem, search, graph, drive
  registry.
- `chan-writer/chan-llm`: LLM backends, embedded prompts, tool
  sandbox, API-key resolution. Owns its own repo so native
  shells (iOS / Android, future) can link it via uniffi
  alongside chan-core, without dragging in this repo's axum /
  tower / reqwest stack. chan-server is just one of two
  consumer shapes.

Both are pulled in via path deps that assume a sibling-checkout
layout. Switch to git or crates.io when the repos go public.

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

### chan-llm (sibling repo, not in this workspace)

Owns: LLM backends (Anthropic, Gemini, Ollama), embedded prompts,
the tool execution sandbox (`read_file`, `write_file`,
`list_files`, `search_content` implemented against `chan-core`),
and key resolution (env / OS keychain / `<config>/chan/llm.toml`).
Tool reads / writes always go through `chan-core::Drive` so the
filesystem gates apply.

Lives at `chan-writer/chan-llm`, NOT in this workspace, because
chan-server is one of two consumer shapes:

  - chan-server (here) wraps `LlmSession` in axum routes and
    forwards `SessionListener` events over WebSocket.
  - Native shells (iOS / Android, future) link chan-llm via
    uniffi alongside chan-core and implement `SessionListener`
    in Swift / Kotlin.

Putting chan-llm in this workspace would force native shells to
either drag in axum / tower / reqwest's HTTP stack or
reimplement the LLM logic. Both are worse than a small extra
repo.

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
3. **Port LLM into the sibling `chan-writer/chan-llm` repo**:
   - Public API contract (LlmConfig, LlmSession, SessionListener,
     tool sandbox, key resolution) already shipped at
     chan-llm@14162fc.
   - Backends (Anthropic / Gemini / Ollama) port from the old
     `fiorix/chan/crates/chan-core/src/llm/{claude,gemini,ollama}.rs`
     in chan-llm.
   - Real prompts replace the placeholders in chan-llm's
     `prompts.rs`.
   - chan-server wires `/api/llm/*` to `chan_llm::LlmSession`,
     bridging `SessionListener` events to the WebSocket. Answers
     dir + attachments dir live as chan-server-side helpers
     (they're persistence concerns, not LLM-layer concerns).
4. ~~**Port the frontend**~~ done. `web/` ported from
   fiorix/chan; rust-embed in chan-server bakes `web/dist/` at
   release-build time (debug reads from disk). `build.rs`
   invalidates Cargo's cache on bundle changes. SPA fallback
   serves `index.html` for any non-API non-asset path so
   client-side routes work; `/api` and `/ws` misses return a
   real 404 instead of HTML.
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
- HTTP routes for sessions / answers / attachments / assistant
  conversation history. App-level persistence; phase 7.
- LLM backends. chan-llm's Anthropic / Gemini / Ollama modules
  are still stubs; the route surface in chan-server is locked
  and the real port drops in without surface changes here.
- Real prompts in chan-llm. Placeholders today; bumping them is
  a chan-llm-side commit that doesn't touch this repo.
