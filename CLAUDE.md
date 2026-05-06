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
  chan         the binary. Parses CLI args, dispatches subcommands,
               mounts the embedded frontend.
  chan-server  HTTP + WebSocket surface. Wraps chan-core in axum
               routes; uses chan-llm for assistant routes.

web/           Svelte frontend (wires in a follow-up commit).
```

Two sibling repos pulled in as path deps:

- `chan-writer/chan-core` (filesystem, search, graph, drive
  registry).
- `chan-writer/chan-llm` (LLM backends, embedded prompts, tool
  sandbox, key resolution). Lives in its own repo so native
  shells (iOS / Android) can link it via uniffi alongside
  chan-core, without dragging in this repo's HTTP stack.

We depend on both as sibling-checkout path deps; switch to git
or crates.io when the repos go public.

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

All filesystem operations route through `chan_core::Drive`, which
sandboxes paths under the registered drive root, refuses
non-regular files (symlinks, FIFOs, sockets, devices), and
performs atomic writes. Nothing in this repo should ever call
`std::fs::*` on user content directly.

### Single binary, no runtime deps

No Node.js, no Python, no native daemons at runtime. The frontend
embeds at build time. New dependencies must hold this line.

### Local-first / loopback-only

The HTTP server binds `127.0.0.1` by default. Auth is a per-launch
bearer token printed once on stderr and appended to the launch URL.
No TLS. The single-user, single-machine assumption is architectural.

### App-level vs core

What lives in chan-core (filesystem, search, graph) vs what lives
here (HTTP, LLM, editor preferences, sessions, assistant history,
API keys, attachments) is a hard line. Don't push library concerns
into chan-core, and don't reimplement library primitives here.
When in doubt, read `../chan-core/design.md`.

## Writing Rules

- **No em dashes** in comments or documentation.
- **Tables**: pure ASCII, target 80 columns.
- **Factual**: no marketing language. Include analysis with
  benchmarks; explain whether numbers meet expectations.
- **Comments**: explain WHY, not WHAT.

## Contributor Patterns

- **Atomic writes via chan-core**: every user-content write goes
  through `Drive::write_text` or `Drive::write_bytes`. These
  enforce the editable-text gate, the path sandbox, and the
  special-file refusal. Don't bypass.
- **Subcommand parity**: every chan subcommand has a clap
  definition + a `cmd_*` function in `crates/chan/src/main.rs`.
  Help text must reflect actual behavior; don't claim env vars
  or flags that don't exist.
- **Server routes go in chan-server**: never inline an axum
  handler inside the binary crate. The `chan` crate parses args
  and calls `chan_server::serve`. Port routes from the old
  `chan-core/src/server.rs` in `fiorix/chan` one cluster at a
  time.
- **LLM lives in chan-writer/chan-llm**: backends, tools, prompts,
  and key resolution all live in the sibling repo. chan-server's
  /api/llm/* routes wrap `chan_llm::LlmSession`. The `chan`
  binary never directly invokes a backend.
- **Pinned toolchain**: do not introduce code that requires a
  newer Rust than `rust-toolchain.toml` declares without bumping
  the pin in the same commit.

## Documentation

- **Design and architecture**: [`design.md`](design.md). Single
  load-bearing reference for the workspace layout, the chan-core
  contract, and the migration plan from `fiorix/chan`.
- **chan-core design**: `../chan-core/design.md`. Read before
  proposing chan-core changes.
- **Issue tracker**: GitHub repo `chan-writer/chan`.
