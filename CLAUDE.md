# CLAUDE.md

Contribution guidelines for Claude Code (claude.ai/code) when
working on the `chan-core` workspace.

## What this workspace is

`chan-writer/chan-core` is a Cargo workspace that holds the cross-
platform Rust core of chan, a local-first markdown editor. Five
crates: `chan-drive` (sandboxed FS + tantivy search + sqlite graph),
`chan-tunnel-{proto,client,server}` (h2/yamux tunnel transport),
and `chan-llm` (LLM backends, prompts, tool sandbox). Same crates
back the `chan` CLI today and Swift / Kotlin shells via uniffi
once those land.

Each crate's `README.md` is the canonical design reference for that
crate. This file is the only set of contributor guidelines and
applies to every crate.

## Build & Test

```bash
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The Rust toolchain is pinned in `rust-toolchain.toml`. `cargo`
auto-installs the pinned version through rustup on first use, so
contributor and CI clippy lint sets stay locked together. The pre-
push hook (`./scripts/install-hooks` to install) runs the same gate
as CI under the pinned compiler, with `RUSTFLAGS=-D warnings` plus
the `--no-default-features` build, so a passing local push will
not fail in the cloud.

Bumping Rust = edit `rust-toolchain.toml` + fix new clippy findings
in the same commit. Don't drift between local and CI.

## Writing Rules

- **No em dashes** in comments or documentation. They hurt
  readability in terminals and divert the reader's train of
  thought. Use commas, semicolons, parentheses, or separate
  sentences.
- **Tables**: pure ASCII, target 80 columns, no Unicode box-
  drawing. When a table would exceed 80 columns, keep a short
  summary table and expand details below with bullet points.
- **Factual**: no marketing language ("just", "easy", "blazing").
  When reporting test results or benchmarks, include analysis of
  what the numbers mean and whether they meet expectations.
- **Comments**: explain WHY, not WHAT. The code shows what; the
  comment explains the reasoning, the trade-off, or the
  constraint.

## Workspace Principles

These rules cut across every crate. Per-crate design (on-disk
layout, locking, public API shape) lives in each crate's README.

### FFI-shaped public APIs

Every public type has to survive a uniffi boundary later. No
lifetimes on public types; owned `String` / `PathBuf` only;
`Arc`-able handles; one umbrella error enum per crate with
primitive payloads (no `reqwest::Error` or `chan_drive::ChanError`
re-exported, flatten via `Display`). Streaming is callback-based
through trait objects, never `impl Stream` or
`tokio::sync::mpsc::Receiver`.

### Sync API surface, async only behind it

No public `async fn` in any crate's headline API. `chan-drive` is
sync end-to-end; `chan-llm` and the tunnel crates own their tokio
runtime internally and surface results through callbacks. Internal
helpers may use threads or async; they must not leak into the
public surface.

### Atomic writes, always

Anything chan-drive-managed (registry, sessions, graph DB control
records, blob storage) goes through `chan_drive::fs_ops::atomic_write`.
Never `std::fs::write` directly to the target. A crash mid-write
must produce zero state for the writer plus an intact previous
version.

### Filesystem operations route through chan-drive

`chan-llm` tools (`read_file`, `write_file`, `list_files`,
`search_content`) call into `chan_drive::Drive` so the path
sandbox, special-file refusal, atomic writes, and editable-text
gate apply automatically. A backend cannot invent a tool that
bypasses these gates. The tunnel crates never read or write
drive contents; they forward HTTP only.

### Tests use isolated config dirs

`Library::open_at(path)` takes an explicit config path so tests
never touch the developer's real `~/.chan`. Always use this in
tests; never call `Library::open()` from a test. chan-llm tests
use the `Collector` listener pattern (`Vec<Event>`) over a
`Library::open_at` drive.

## Contributor Patterns

Per-crate rules that come up often when editing this code. For
the full design rationale, read the crate README.

### chan-drive

- **Strict path resolution**: every Drive entry point uses
  `fs_ops::resolve_safe_strict` (lexical sandbox plus a
  canonical-form check on the deepest existing ancestor). New
  entry points must do the same. The strict variant catches mid-
  path symlinks pointing outside the drive.
- **lstat, never stat, on user paths**: `read` / `write` / `stat`
  / `remove` use `fs_ops::ensure_regular_file` or
  `std::fs::symlink_metadata` so a symlink target can't mask the
  link. New ops touching user content must apply the same gate.
- **Editable-text gate**: `is_editable_text(rel)` is the single
  predicate for "the editor can safely round-trip this file
  through a UTF-8 buffer." `read_text` / `write_text` enforce it;
  binary callers use `read` / `write_bytes`.
- **Watcher drops `.chan/` and `.git/`**: chan-drive never writes
  inside the user's drive directory, so any `.chan/` activity is
  foreign noise. Filter chain in `watch::dispatch` short-circuits
  both subtrees.
- **Send + Sync on public types**: anything in `Arc<Drive>` must
  be `Send + Sync`. `rusqlite::Connection` is `Send` but not
  `Sync`, so `GraphView` wraps it in a `Mutex`.

### chan-llm

- **Backends never touch chan-drive directly**: a backend builds
  wire-format requests and parses streaming responses. Anything
  filesystem goes through the tool sandbox.
- **`auto_apply_writes` is the user's contract**: when false,
  `write_file` returns `Pending`. Never silently flip it to true
  and never write to disk in the false branch.
- **Keys: env -> keychain -> file**: writes go to the OS keychain
  only. The file fallback (`LlmConfig.keys`) is read-only from
  chan-llm's perspective; a user-managed TOML stays user-managed.

### chan-tunnel-{proto,client,server}

- **Proto stays pure**: `chan-tunnel-proto` has no I/O, no async,
  and no dependency on tokio / hyper at runtime. Adding either
  is a sign the type belongs in the client or server crate.
- **Hello first, yamux after**: the duplex carries a length-
  prefixed JSON Hello / HelloAck pair, then yamux. Don't sneak
  protocol changes into the post-handshake stream; bump
  `ProtocolVersion` and negotiate inside `Hello`.
- **Drive name validation is shared**: use
  `chan_tunnel_proto::is_valid_drive_name` /
  `sanitize_drive_name` on both sides. Don't reimplement.

## Documentation

- **Workspace overview**: [`README.md`](README.md)
- **Crate design references**:
  - [`crates/chan-drive/README.md`](crates/chan-drive/README.md):
    on-disk layout, locking model, public API surface, schema
    versioning.
  - [`crates/chan-llm/README.md`](crates/chan-llm/README.md):
    backend list, prompts, tool sandbox, FFI plan.
  - [`crates/chan-tunnel-proto/README.md`](crates/chan-tunnel-proto/README.md):
    wire types, framing, control frames.
  - [`crates/chan-tunnel-client/README.md`](crates/chan-tunnel-client/README.md):
    dial + handshake, embedding into `chan serve`.
  - [`crates/chan-tunnel-server/README.md`](crates/chan-tunnel-server/README.md):
    terminator library, Validator, Registry.
- **Issue tracker**: GitHub repo `chan-writer/chan-core`.
