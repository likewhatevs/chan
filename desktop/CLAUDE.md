# chan-desktop notes

## Bundled chan sidecar

chan-desktop ships with a copy of the `chan` binary inside the
packaged app so the user does not need a separate `cargo install` /
`brew install` to run drives. The bundling is configured via Tauri's
`externalBin` mechanism in `src-tauri/tauri.conf.json`
(`bundle.externalBin = ["binaries/chan"]`), and `desktop/Makefile`'s
`chan-bin` recipe stages `target/release/chan` to
`src-tauri/binaries/chan-<target-triple>` before every build.

### Bundle layout

Tauri strips the `-<target-triple>` suffix at bundle time and places
the sidecar next to chan-desktop's own binary:

| Build              | Sidecar path                                 |
|--------------------|----------------------------------------------|
| `cargo tauri dev`  | `target/debug/chan`                          |
| Packaged macOS     | `Chan.app/Contents/MacOS/chan`               |
| Packaged Linux     | sibling of `chan-desktop` in the install dir |
| Packaged Windows   | sibling of `chan-desktop.exe`, name `chan.exe` |

`Contents/MacOS/chan` is the recommended location on macOS because
Tauri's code-signing step automatically covers everything under
`Contents/MacOS/`. No custom `codesign --deep` invocation is needed:
both binaries get a Developer ID signature from the same identity in
one pass, and `ci-7`'s notarization roundtrip covers both for free.

### Resolution helpers

`crates/chan-desktop/src/serve.rs` exposes three helpers used by
every spawn site:

* `resolve_chan_binary() -> Result<PathBuf, String>` — the
  PATH-first picker. Walks `PATH` for a `chan` (or `chan.exe`)
  binary; if found, probes its `--version` and accepts only an
  EXACT semver match against chan-desktop's own
  `env!("CARGO_PKG_VERSION")`. Any failure (no chan on PATH,
  spawn error, --version error, version mismatch) falls through
  to `bundled_chan_path()`. Result: `add_drive`, `remove_drive`,
  `set_drive_on`, and the boot-time preflight all delegate to
  this helper so a power user who runs `cargo install --path
  crates/chan` against the matching version gets to use their
  own build through chan-desktop without rebuilding chan-desktop.
* `bundled_chan_path() -> Result<PathBuf, String>` — pure path
  math over `current_exe()`. Returns the expected sidecar path
  without checking that the file exists. Cheap, infallible at
  runtime in practice (only fails if `current_exe()` itself
  fails).
* `probe_chan_version(bin: &Path) -> Result<(), String>` — runs
  `<bin> --version` and asserts an EXACT semver match against
  `env!("CARGO_PKG_VERSION")`. Used by both the boot-time
  preflight (validating whatever `resolve_chan_binary()` picks)
  AND `resolve_chan_binary()` itself (validating the PATH
  candidate before accepting it). Exact match (not a `>=` floor)
  is the locked Round-2 decision-3 contract: bundled chan is
  built from the same workspace checkout as chan-desktop, and
  any PATH chan that doesn't match the same version is rejected
  in favour of the bundled fallback.

The boot-time preflight in `main.rs::compute_bin_status` runs the
resolver + existence check + version check exactly once, stores
the verdict in `AppState::bin_status`, and every IPC that spawns
chan gates on `require_bin()` before calling
`resolve_chan_binary()`.

#### Resolution algorithm (user-facing behaviour)

| State                                          | Picked binary            |
|------------------------------------------------|--------------------------|
| `chan vN` on PATH (N = chan-desktop's version) | PATH chan                |
| `chan vM` on PATH, M ≠ N                       | bundled (PATH rejected)  |
| `chan` on PATH errors on `--version`           | bundled                  |
| no `chan` on PATH                              | bundled                  |
| both unavailable                               | error → `BinStatus::missing` |

Fresh installs Just Work: chan-desktop ships the matching `chan`
inside the app bundle. Power users who want chan-desktop to drive
their own chan build install it to PATH at the matching version
(`cargo install --path crates/chan` from the same checkout), and
chan-desktop picks it up automatically on next launch. A
mismatched PATH install (older or newer than chan-desktop) is
ignored cleanly — the app keeps working via the bundled binary.

### Architecture handling

`desktop/Makefile`'s `chan-bin` recipe currently builds for the
host's target triple only (`$(shell rustc -vV | sed -n 's/host: //p')`).
A macOS universal2 fat binary (`aarch64-apple-darwin` +
`x86_64-apple-darwin` merged via `lipo -create`) is the next step
for distributing a single DMG that runs natively on both Apple
Silicon and Intel Macs; that work is owned by `ci-7` in the
GitHub Actions release workflow rather than in this Makefile (CI
already runs per-arch matrix builds and is the natural place to
`lipo`-merge before bundling).

## Auto-upgrade signing (tauri-plugin-updater)

The desktop app verifies update bundles with a minisign signature.
Pubkey is embedded in `src-tauri/tauri.conf.json` under
`plugins.updater.pubkey`. Matching private key lives outside the
repo at `~/.tauri/chan-desktop.key`.

### Current key is a DEV key

Generated with `cargo tauri signer generate --ci ...` on
2026-05-11. No password. Unencrypted on disk. Anyone with read
access to the dev box's `~/.tauri/` can sign a "valid" update.

### Rotate before any public release

1. On a secure machine, run:
   `cargo tauri signer generate -w <newkey>`
   Set a strong password when prompted.
2. Replace `plugins.updater.pubkey` in `tauri.conf.json` with the
   contents of `<newkey>.pub` and ship a "bridge" release still
   signed with the OLD key (so existing installs accept it). The
   bridge release embeds the NEW pubkey in the binary.
3. Every release after the bridge is signed with the NEW key.
4. Old installs that never picked up the bridge release will fail
   to verify NEW-key-signed bundles and stall on their last good
   version until the user manually reinstalls. Plan the bridge
   window for how long you're willing to support that tail.
5. The signing identity used at build time is selected via
   `TAURI_SIGNING_PRIVATE_KEY` (key contents) or
   `TAURI_SIGNING_PRIVATE_KEY_PATH` (file path), with optional
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. CI should pull these from
   a secrets store, never from the repo.

### Manifest endpoint

Client probes:
`https://chan.app/dl/desktop/{{target}}/{{current_version}}/latest.json`

Server-side publishing of that manifest is owned by chan-prod-setup.
