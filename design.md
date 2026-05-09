# chan-desktop design

This document is the source of truth for what chan-desktop is and is
not. It is intentionally light on Rust / Tauri specifics and heavy on
business logic. When the implementation drifts from this doc, fix one
of the two.

## 1. Purpose

chan-desktop is a thin desktop shell around the `chan` CLI. The CLI
already does the actual work: it indexes a markdown folder ("drive")
and serves a Svelte web editor on a local HTTP port. The desktop app
exists so that:

- a non-CLI user can install one signed bundle and open a folder
  through a familiar OS dialog instead of a terminal,
- multiple drives can be supervised at once, with one app window
  acting as the inventory and on/off control,
- the `chan` binary lifecycle (process, port, URL, token) is hidden
  behind buttons.

Non-goals:

- chan-desktop is not a second editor. The editor is the web app
  served by `chan serve`. The desktop window only manages drives.
- chan-desktop is not a packaging tool for `chan`. It bundles a
  pinned `chan` binary and treats it as an internal dependency, not
  a user-facing CLI install path.
- chan-desktop is not a web browser. The drive itself is opened in
  the user's default browser, not in the Tauri webview.

## 2. Mental model

Two processes per running drive:

```
                +-------------------+
   user -->     |  chan-desktop     |   single Tauri window per
                |  (supervisor)     |   user session
                +---------+---------+
                          | spawns / signals
                          v
                +-------------------+
                |  chan serve PATH  |   one per "on" drive
                |  (HTTP + WS)      |
                +---------+---------+
                          | http://127.0.0.1:PORT/?t=TOKEN
                          v
                  default OS browser
```

chan-desktop's window never embeds the editor itself. The Launch
button on each row opens the captured URL in the system browser via
the Tauri opener plugin.

## 3. Drive lifecycle

A "drive" in chan-desktop maps 1:1 to a known drive in the `chan`
registry (`~/.chan/config.toml`). Visible state per drive in the
inventory:

| column | meaning                                          |
|--------|--------------------------------------------------|
| On     | toggle: is `chan serve` running for this drive?  |
| Path   | absolute path the user picked                    |
| Name   | display name (subject to chan-core constraints)  |
| URL    | the URL printed by `chan serve` once it is up    |
| Close  | unregister: stop + `chan remove`                 |

### 3.1 Open drive

Triggered by the "Open drive" button (and automatically on first
launch when the config has no drives).

1. Tauri opens a native folder picker.
2. The selected path is canonicalised and validated (see
   section 4).
3. chan-desktop runs `chan add <path>` to register the drive in the
   `chan` registry. Stdout is parsed for the assigned name; on
   success the drive is appended to the desktop config.
4. The new row appears with **On = off**. The user toggles On to
   start serving.

`add` is a separate step from `serve`: registering without serving is
a valid state, and matches what the CLI does.

### 3.2 Toggle On (serve)

Toggling On spawns `chan serve <path> --port 0 --no-token=false` (or
equivalent flags determined later) as a child process owned by
chan-desktop. The supervisor:

- streams stderr line-by-line until it sees the "chan is ready:"
  banner and the URL on the following line,
- captures `http://127.0.0.1:PORT/?t=TOKEN` and stores it in the
  in-memory drive state (not persisted; tokens rotate per serve),
- exposes that URL on the row so the Launch button can open it,
- reaps the child on toggle-off, on Close, and on app exit.

Failure modes the supervisor must handle explicitly:

- non-zero exit before the banner: surface stderr to the user and
  flip the toggle back to Off,
- port already in use / permission denied: same,
- supervisor crash: child must not outlive the desktop process. On
  Unix, set up a process group and SIGTERM on drop. On Windows, use
  a job object.

### 3.3 Toggle Off (stop)

Sends SIGTERM (Unix) or the equivalent on Windows. `chan serve`
handles graceful shutdown with a 10s grace period. After that, SIGKILL.
The URL is cleared.

### 3.4 Close drive (remove)

Stops the serve (if running), then runs `chan remove <path>`. The
filesystem is untouched. The row is removed from the desktop config.

"Close" deliberately leaves the user's markdown folder alone. There
is no "delete drive" action in the desktop UI.

## 4. Validation

The desktop app does not invent its own validation. It defers to
chan-core / chan helpers, both to avoid drift and so that anything
the desktop app accepts is also accepted by every other chan
surface.

- **Drive name**: must satisfy `chan_tunnel_proto::is_valid_drive_name`
  (lowercase `[a-z0-9-]`, 1-32 chars, no leading/trailing hyphen).
  Rename input is sanitised through `sanitize_drive_name` before the
  user can hit Save.
- **Path**: canonicalised via `std::fs::canonicalize` before being
  passed to `chan add` / `chan serve`. Reject paths that fail to
  canonicalise (broken symlink, permission), are not directories, or
  contain components that the host shell would interpret (we always
  pass argv as a slice, never as a single shell-quoted string, so
  the main concern is canonicalisation, not quoting).
- **Relative path arguments inside a drive** (used later, not by the
  current UI) reuse `chan_drive::fs_ops::validate_rel`.

These dependencies will be pulled in as path deps once the desktop
crate starts linking chan-core directly. While we are still
shelling out exclusively, sharing happens by re-implementing the
same character set tests in the frontend for early feedback and
re-checking on the Rust side before any subprocess call.

## 5. The chan binary

chan-desktop does not implement any drive logic. It calls out to a
`chan` executable. There are two phases:

### 5.1 Prototype (now)

The supervisor spawns whatever `chan` is on `$PATH`. Developers and
early testers are expected to have a working `chan` install. There is
no version pinning. This is fine for iteration but unacceptable for
end-user releases.

### 5.2 Production (target)

The release `.app` / `.exe` / Linux bundle ships a pinned `chan`
binary inside the bundle. The supervisor resolves it relative to the
running executable, never `$PATH`. This is a deliberate choice:

- Pro: deterministic. One desktop release pairs with exactly one
  `chan` build. The test matrix is "did this DMG work" rather than
  "what version of `chan` does the user have".
- Pro: signed and notarised together; macOS Gatekeeper sees one
  bundle.
- Con: every chan release requires a new desktop release, even if
  the desktop UI did not change. Acceptable while both are
  pre-1.0 and ship together.

The `chan` binary lives at:

- macOS:   `ChanDesktop.app/Contents/Resources/bin/chan`
- Linux:   alongside the desktop binary (`/usr/lib/chan-desktop/chan`
           when packaged, or next to the binary in a tarball)
- Windows: `chan.exe` in the install directory

Resolution order at runtime: bundled binary first, fall back to
`$PATH` only in dev builds (`#[cfg(debug_assertions)]`).

## 6. Power users and the CLI tool

Non-goal: chan-desktop installation should be "drag ChanDesktop.app
to /Applications". No installer, no scripts.

That conflicts with power users who want `chan` on their `$PATH`. The
current thinking, in order of preference:

1. **Settings button: "Install command line tool"**. Symlinks the
   bundled `chan` to `/usr/local/bin/chan` (or `~/.local/bin/chan`
   if the user lacks write access). Off by default. Removable. This
   is what Xcode does. It keeps the default install dead simple and
   makes the CLI an explicit, reversible choice.
2. **First-run prompt** offering the same. Probably too much for
   regular users who do not know what a CLI is. Skip unless data
   from real users says otherwise.

Linux: distros generally expect packaging to put binaries on `$PATH`
for them, so the dpkg / rpm bundle should `Conflicts:` / `Replaces:`
the standalone `chan` package and install both. That keeps the test
matrix concern from section 5 intact.

Windows: add the install directory to PATH during MSI install, with
an opt-out checkbox. Same reasoning as macOS.

## 7. Distribution

Primary channel is https://chan.app. The release flow on the maintainer's
machine produces:

- macOS arm64: notarised DMG containing `ChanDesktop.app`. Drag to
  /Applications.
- Linux amd64 / arm64: `.deb` and `.AppImage`.
- Windows amd64 / arm64: signed MSI.

Cargo install (`cargo install chan-desktop`) is a supported but
secondary path: it builds without the bundled `chan`, so it falls
back to `$PATH` lookup like the prototype. This is for contributors
and packagers, not end users. The README points end users at
chan.app.

Cross-bundling from a single host is painful (webkit2gtk on Linux,
MSVC on Windows, codesigning on macOS). The Linux and Windows
artifacts will be built in CI; only the macOS DMG is built locally
on the maintainer's machine where the signing identity lives.

## 8. Self-upgrade

Deferred. `chan` already implements self-upgrade
(`crates/chan/src/update.rs`): fetch
`https://chan.app/dl/latest/VERSION`, download an archive matching
the current target triple, verify SHA-256, atomically replace the
running executable. chan-desktop wants the same.

The plan:

1. Generalise the upgrade module out of `chan` into chan-core (new
   crate, e.g. `chan-self-update`), parameterised over the target
   binary name and the download URL prefix. The current code is
   already mostly generic; the chan.app URLs are the main
   chan-specific thing.
2. chan-desktop links that crate and uses it to upgrade itself. On
   macOS, "atomic replace" inside a signed bundle is more involved
   than swapping a single binary; the desktop case may need to
   download and stage a new `.app` and prompt the user to relaunch.
   That is the part that needs design work.
3. Because chan is bundled with chan-desktop (section 5.2), the
   bundled `chan` upgrades when the desktop upgrades. The bundled
   `chan` therefore does not need its own auto-update banner.
   Disable the banner when run from inside the bundle (env var or
   parent-process check).

This whole section becomes urgent once the first end-user DMG ships.

## 9. Open questions

- `chan serve` flags: do we always pass `--no-token` and rely on
  loopback, or do we keep the token and inject it into the launched
  URL? Tokens make accidental cross-app exposure harder; cost is
  the URL changes every serve. Lean: keep the token.
- Port assignment: pass `--port 0` and parse the printed URL, or
  allocate a random port ourselves and pass it in? The CLI does not
  currently support `--port 0`; if/when it does, switch.
- Multiple desktop windows vs one window: current design is one
  window with a drives table. Adding per-drive child windows is a
  later concern.
- Settings: which subset of `chan serve` flags do we expose to the
  user, and which do we hardcode? Tunnel publishing
  (`--tunnel-token`, `--public`) probably belongs in a per-drive
  "Share" panel rather than the global settings.
