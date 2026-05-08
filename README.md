# chan

Notes app with embedded web editor. The `chan` binary is a CLI plus
an HTTP server that serves a Svelte WYSIWYG editor for plain markdown
drives, with cross-file `[[wiki-link]]` autocomplete and BM25
content search.

## Layout

```
crates/
  chan         the binary. Subcommands (add, list, remove, rename,
               serve, index, search). Embeds the web frontend at
               build time.
  chan-server  HTTP + WebSocket surface. Wraps chan-drive in axum
               routes; uses chan-llm for assistant routes.
web/           Svelte frontend, embedded into the binary at build
               time. Wires in a later commit.
```

The chan-core sibling repo (a Cargo workspace) provides
`chan-drive` (filesystem, search, graph, drive registry),
`chan-llm` (LLM backends, embedded prompts, tool sandbox, key
resolution), and `chan-tunnel-client` (h2/yamux drive tunnel),
all as path deps. Native shells (iOS / Android) link
`chan-drive` and `chan-llm` via uniffi without dragging in
chan-server's HTTP stack.

The workspace assumes the sibling-checkout layout
`~/dev/github.com/chan-writer/{chan,chan-core}`.

## Status

Pre-alpha. `chan add`, `chan list`, `chan remove`, `chan rename`,
`chan index`, `chan search`, and `chan serve` work end-to-end.
LLM, attachments, sessions, and assistant chat history land via
chan-llm and chan-server config files. Tunnel publishing
(`--tunnel-token`) works against a running chan-tunnel daemon.

## Build

```bash
git clone git@github.com:chan-writer/chan-core ../chan-core

make            # frontend bundle + release binary
make install    # copy target/release/chan to /usr/local/bin
make dev        # run `chan serve /tmp/chan-dev --no-token`
```

`make install PREFIX=$HOME/.local` to install per-user. `make help`
lists every target. Manual cargo / npm calls still work; the
Makefile is just a shortcut.

In debug builds, rust-embed reads files from `web/dist/` on each
request, so a re-run of `make web` (or just `npm run build`)
updates the served bundle without a `cargo build`. In release
builds, the bundle is baked into the binary at compile time;
`build.rs` re-links chan-server whenever any file under
`web/dist/` changes.

## Publish via tunnel

Instead of binding a local port, `chan serve` can publish a drive
at `https://drive.chan.app/{user}/{drive}/*` over an outbound
tunnel. No inbound ports, no router config.

```
export CHAN_TUNNEL_TOKEN=chan_pat_...    # from id.chan.app/tokens
chan serve ~/Notes
```

`chan` dials `tunnel.chan.app`, runs a Hello/HelloAck handshake
that names the drive, and serves every inbound request through
the same axum router the local-mode listener uses. The flag form
`--tunnel-token <TOKEN>` works too but exposes the token in `ps`;
prefer the env var. Override the endpoint with `--tunnel-url`,
publish under a different name with `--tunnel-drive <name>`. The
drive name must be lowercase `[a-z0-9-]`, 1-32 chars.

The public URL is currently world-readable. OAuth gating at the
gateway is tracked separately; for now treat the tunneled URL as
public.

## Contributing

The Rust toolchain is pinned in `rust-toolchain.toml` (1.95.0).
Install [rustup](https://rustup.rs/); it picks up the pin
automatically the first time you run `cargo` here.

Install the pre-push hook once per clone:

```
./scripts/install-hooks
```

The hook runs `cargo fmt --check`, `cargo clippy -- -D warnings`,
`cargo test --all-targets`, and `cargo build --no-default-features`
with `RUSTFLAGS=-D warnings` before every push, mirroring CI. A
passing local push therefore will not fail in GitHub Actions.

### CI cross-repo auth

CI needs to clone the chan-core sibling repo (private) to
resolve the `chan-drive`, `chan-llm`, and `chan-tunnel-client`
path deps.

One-time setup:

1. Create a fine-grained GitHub Personal Access Token at
   https://github.com/settings/personal-access-tokens with
   `Contents: Read` access on `chan-writer/chan-core`.
2. On `chan-writer/chan`'s `Settings -> Secrets and
   variables -> Actions`, add a secret named
   `chan_ci` with the PAT as its value.

Until the secret is set, CI's sibling-checkout step fails. The
`fmt` job runs without it (no cross-repo dep needed for
rustfmt).
