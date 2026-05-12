# chan

Notes app for plain markdown drives. `chan` is a single static binary
that bundles a CLI and a local HTTP server; the server serves a
Svelte WYSIWYG editor that the user edits notes in. Cross-file
`[[wiki-link]]` autocomplete, BM25 + embedding hybrid search, and an
optional assistant pane wired to Anthropic, Gemini, Ollama, or the
local `claude` / `gemini` CLIs.

Single-user, single-machine. Loopback HTTP by default; an opt-in
tunnel mode publishes the same drive at
`https://{user}.drive.chan.app/{drive}/*` for cross-device access.

## Layout

```
crates/
  chan           binary. CLI + dispatch.
  chan-server    HTTP + WebSocket surface; embeds the web bundle.
  fetch-models   build helper for the embedding-model bundle.

web/             Svelte frontend, embedded into the binary at build
                 time via rust-embed.
```

`chan` and `chan-server` depend on three crates from the sibling
`chan-writer/chan-core` repo: `chan-drive` (filesystem, search,
graph), `chan-llm` (LLM backends, prompts, tool sandbox, keys), and
`chan-tunnel-client` (tunnel transport). The workspace assumes a
sibling-checkout layout at `~/dev/github.com/chan-writer/{chan,chan-core}`.

See [`design.md`](design.md) for the architecture and how the
frontend embeds into the binary.

## Build

```bash
git clone git@github.com:chan-writer/chan-core ../chan-core

make                # frontend bundle + debug binary
make build-release  # frontend + embedded model bundle + release binary
make install        # copy target/release/chan to PREFIX/bin
make dev            # run `chan serve /tmp/chan-dev --no-token`
```

`make help` lists every target. Manual `cargo` / `npm` calls still
work; the Makefile is just a shortcut.

`make install` defaults `PREFIX=$XDG_BIN_HOME` (or `$HOME/.local`),
so the binary lands in `~/.local/bin/chan` without sudo. Override
for a system-wide install: `make install PREFIX=/usr/local`.

In debug builds, rust-embed reads files from `web/dist/` on each
request, so a re-run of `make web` (or just `npm run build`)
updates the served bundle without a `cargo build`. In release
builds, the bundle is baked into the binary at compile time;
`build.rs` re-links chan-server whenever any file under `web/dist/`
changes.

The release build also bundles a pre-fetched embedding model
(BGE-small, ~80 MB). `make build-release` runs `make models` first,
which pre-populates `crates/chan-server/resources/models.tar.zst`;
the seeder at first launch zstd-decodes + untars it into the
per-machine cache. Plain `cargo build` ships an empty stub: at
runtime the seeder downloads from HuggingFace as a fallback.

`HTTPS_PROXY` / `HTTP_PROXY` are honored everywhere chan reaches
out (model fetch, self-upgrade probe, LLM backends).

## Run

```bash
chan add ~/Notes              # register the drive
chan serve ~/Notes            # bind 127.0.0.1:8787 and open browser
```

The first launch prints the URL on stderr and opens the user's
default browser. The URL carries a per-launch bearer token; the
same token also accepts an `Authorization: Bearer ...` header.
The token is persisted at `<state>/tokens/token` so a `cargo build
&& chan serve` cycle does not invalidate the browser's cached
sessionStorage token.

Useful flags:

- `-4` / `-6`: force IPv4 / IPv6 loopback (default 127.0.0.1).
- `--host`, `--port`: bind elsewhere. No TLS; loud warning when
  binding off-loopback.
- `--prefix /seg`: mount under a URL prefix so a reverse proxy can
  multiplex many `chan serve` instances under one host.
- `--timeout 30s` / `5m` / `1h`: graceful shutdown after an idle
  window with no HTTP / WebSocket activity. Designed for systemd
  socket-activation.
- `--no-token`: skip the bearer-token gate (loopback bind only).

`chan upgrade` self-replaces the running binary against
`https://chan.app/dl/...`, with SHA-256 verification. Set
`CHAN_UPDATE_CHECK=0` to silence the once-per-day banner.

Other subcommands: `chan list`, `chan remove`, `chan rename`, `chan
index`, `chan search`. `chan --help` documents every flag.

## Publish via tunnel

Instead of binding a local port, `chan serve` can publish a drive
at `https://{user}.drive.chan.app/{drive}/*` over an outbound
tunnel. No inbound ports, no router config.

```
export CHAN_TUNNEL_TOKEN=chan_pat_...    # from id.chan.app/tokens
chan serve ~/Notes
```

`chan` dials `drive.chan.app/v1/tunnel`, runs a Hello/HelloAck
handshake that names the drive, and serves every inbound request
through the same axum router the local listener uses. The flag form
`--tunnel-token <TOKEN>` works too but exposes the token in `ps`;
prefer the env var. Override the endpoint with `--tunnel-url`,
publish under a different name with `--tunnel-drive <name>`. The
drive name must be lowercase `[a-z0-9-]`, 1-32 chars.

By default `{user}.drive.chan.app/{drive}/` returns a 404 to anyone
without a fresh handoff from id.chan.app's dashboard; only the
drive owner can open the drive from there. `--tunnel-public` makes
the URL world-readable (no auth gate at the gateway).

## Contributing

The Rust toolchain is pinned in `rust-toolchain.toml` (1.95). Install
[rustup](https://rustup.rs/); it picks up the pin automatically the
first time you run `cargo` here.

Install the pre-push hook once per clone:

```
./scripts/install-hooks
```

The hook runs `cargo fmt --check`, `cargo clippy --all-targets --
-D warnings`, `cargo test --all-targets`, and `cargo build
--no-default-features` with `RUSTFLAGS=-D warnings` before every
push, mirroring CI. A passing local push therefore will not fail in
GitHub Actions.

### CI cross-repo auth

CI needs to clone the chan-core sibling repo (private) to resolve
the `chan-drive`, `chan-llm`, and `chan-tunnel-client` path deps.

One-time setup:

1. Create a fine-grained GitHub Personal Access Token at
   https://github.com/settings/personal-access-tokens with
   `Contents: Read` access on `chan-writer/chan-core`.
2. On `chan-writer/chan`'s `Settings -> Secrets and variables ->
   Actions`, add a secret named `chan_ci` with the PAT as its
   value.

Until the secret is set, CI's sibling-checkout step fails. The
`fmt` job runs without it (no cross-repo dep needed for rustfmt).
