# chan-desktop

Desktop edition of the [chan](https://chan.app) markdown editor.

chan-desktop is a Tauri shell that embeds `chan-server` for normal
local workspaces. Non-CLI users get a familiar app icon, a window for
managing workspaces, and a one-click "open workspace" flow without a separate
per-workspace `chan serve` process.

The web editor itself is the same Svelte app shipped in the standalone
`chan` binary. Desktop opens it in Tauri webview windows backed by the
embedded server or by explicit remote attachments.

## Download

Notarized macOS builds and Linux builds (`.AppImage`, `.deb`, `.rpm`)
are published on https://chan.app. Windows builds are not published yet.
End users should grab the build from there rather than building from
source.

## Build and run

Requirements: stable Rust (pinned in `rust-toolchain.toml`) and npm for
the embedded web build. The Makefile builds the embedded web bundle
(`web/dist`) and installs `tauri-cli` v2 under `../target/tauri-cli` if
`cargo-tauri` is not already on `$PATH`.

```bash
make dev      # cargo tauri dev (launch the desktop app in dev mode)
make build    # cargo tauri build (release bundle for the host platform)
make check    # cargo check
make clean    # cargo clean
```

The desktop app stores its config at:

- macOS: `~/Library/Application Support/Chan Desktop/config.json`
- Linux: `~/.config/chan-desktop/config.json`
- Windows: `%APPDATA%/Chan Desktop/config.json`

## Workspace modes

- Local embedded: desktop owns the local workspace runtime through its
  embedded `chan-server` host.
- Remote outbound: desktop opens an already-running `chan serve` URL
  pasted by the user. The remote server owns its own lifecycle.
- Remote inbound: desktop listens on a loopback tunnel endpoint and an
  external `chan serve` connects to it.

There is no local sidecar fallback mode. Running `chan serve` directly
is still supported, but desktop treats it as a remote attachment.

## Layout

```
src/             vanilla HTML / CSS / JS frontend (no bundler)
src-tauri/       Rust crate. main.rs wires Tauri commands;
                 config.rs owns the on-disk config store.
Makefile         shortcuts; manual cargo / cargo tauri calls also work
design.md        what this app actually does, end to end
```

## License

Apache 2.0. See [LICENSE](./LICENSE).
