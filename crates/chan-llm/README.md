# chan-llm

LLM backends, embedded prompts, and the tool sandbox the chan
assistant uses to read and edit chan drives.

## Why this is its own crate

Cross-platform reuse. Two consumer shapes:

- `chan-server` (HTTP server in `chan-writer/chan`) wraps
  `LlmSession` in axum routes and forwards streaming events to the
  web frontend over WebSocket.
- Native shells (iOS / Android, future) link this crate via uniffi
  alongside `chan-core`. They construct `LlmSession` directly,
  implement `SessionListener` in Swift / Kotlin, and receive
  streaming deltas, tool calls, and tool results without a network
  hop.

Both consumers see the same prompts, the same tool schemas, and
the same edit-control rules. That is the point of the crate.

## Status

Pre-alpha. Initial commit is the public API contract:

- `LlmConfig` (load/save TOML at `<config>/chan/llm.toml`, mode
  0600 on Unix) with backend selection, per-backend model
  override, the `auto_apply_writes` flag, and on-disk key
  fallback.
- `keys`: env -> OS keychain -> file resolution; writes go only
  to the keychain.
- `tools`: four standard tools (`read_file`, `write_file`,
  `list_files`, `search_content`) wired through `chan-core::Drive`
  so the filesystem invariants apply automatically.
- `session`: `LlmSession` + `SessionListener` callback API.
- Backend stubs (Anthropic, Gemini, Ollama). Real ports follow
  in subsequent commits.

13 unit tests pass; pre-push hook installed.

## Build

```
git clone git@github.com:chan-writer/chan-core ../chan-core
cargo build
cargo test
```

## Contributing

Toolchain pinned in `rust-toolchain.toml` (1.95.0). Install rustup;
it picks up the pin automatically.

Install the pre-push hook once per clone:

```
./scripts/install-hooks
```

Same gate as the rest of the chan-writer org: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`, and a
no-default-features build under `RUSTFLAGS=-D warnings`.

### CI cross-repo auth

CI needs to clone `chan-writer/chan-core` (a private repo) to
resolve the `path = "../chan-core"` dep. One-time setup:

1. Create a fine-grained GitHub Personal Access Token at
   https://github.com/settings/personal-access-tokens with
   `Contents: Read` access on `chan-writer/chan-core` and
   `chan-writer/chan-llm` (one PAT covers both downstream
   repos so you don't have to manage three).
2. On each downstream repo's `Settings -> Secrets and
   variables -> Actions`, add a secret named
   `CHAN_REPO_TOKEN` with the PAT as its value.
   - `chan-writer/chan-llm`     needs read on chan-core.
   - `chan-writer/chan`         needs read on chan-core +
                                chan-llm.
   - `chan-writer/chan-core`    standalone; no secret needed.

Until the secret is set, CI's checkout-of-the-sibling step
fails with a 404. The `fmt` job still runs (no cross-repo
dep needed for rustfmt).
