# chan-desktop

Desktop edition of the [chan](https://chan.app) markdown editor.

A small Tauri shell that supervises the `chan` binary so non-CLI users
get a familiar app icon, a window for managing drives, and a one-click
"open drive" flow that hides the underlying `chan add` / `chan serve`
plumbing.

The web editor itself is the same one shipped in `chan` proper. This
repo only owns the desktop shell.

## Download

Notarized macOS builds (and, later, Linux and Windows builds) are
published on https://chan.app. End users should grab the build from
there rather than building from source.

## Build and run

Requirements: stable Rust (pinned in `rust-toolchain.toml`) and npm for
the embedded web build. The Makefile builds `chan` and installs
`tauri-cli` v2 under `../target/tauri-cli` if `cargo-tauri` is not already
on `$PATH`.

```bash
make run      # cargo tauri dev
make build    # cargo tauri build (release bundle for the host platform)
make check    # cargo check
make clean    # cargo clean
```

The desktop app stores its config at:

- macOS: `~/Library/Application Support/Chan Desktop/config.json`
- Linux: `~/.config/chan-desktop/config.json`
- Windows: `%APPDATA%/Chan Desktop/config.json`

"Forget all drives" in Settings deletes that file.

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
