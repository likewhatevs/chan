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

### 3.0 Source of truth

The `chan` registry at `~/.chan/config.toml` is the single source of
truth for the set of known drives and their display names.
chan-desktop is a read-only consumer of the drive set; mutation
happens exclusively through `chan add` / `chan remove` (and, later,
`chan rename`).

The desktop owns a small sidecar config of its own at the
platform-appropriate path, holding only state that has no place in
chan: today that is just the `dev_mode` flag and a per-drive
on-toggle, both keyed by canonical drive path. Per-drive serve URLs
are kept in process memory only because chan rotates the bearer
token on every `chan serve`.

A filesystem watcher (`notify` + 150ms debounce) runs over
`~/.chan/` for the lifetime of the process and emits a
`registry-changed` Tauri event whenever the registry file moves.
The frontend reacts by re-fetching `list_drives` and re-rendering.
Concrete consequence: if the user runs `chan add ~/notes` from a
terminal, the row appears in the desktop window without any
explicit refresh.

### 3.1 Drive row state

A "drive" in chan-desktop maps 1:1 to a known drive in the `chan`
registry. Visible state per drive in the inventory:

| column | meaning                                                  |
|--------|----------------------------------------------------------|
| On     | toggle: is `chan serve` running for this drive?          |
| Path   | canonical absolute path from the chan registry           |
| Name   | display name from the chan registry; read-only in the UI |
| URL    | the URL printed by `chan serve` once it is up            |
| Close  | unregister: stop + `chan remove`                         |

Names are deliberately read-only in the desktop. Renaming a drive is
done by running `chan rename` from a terminal; the watcher reflects
the new name on the next debounce tick. The alternative was a
write-through to `chan rename` from the desktop UI, which we
rejected for now to keep the registry-as-SoT contract one-way and
the data flow obvious.

### 3.2 Open drive

Triggered by the "Open drive" button, and automatically on first
launch when the chan registry is empty.

1. Tauri opens a native folder picker.
2. The selected path is canonicalised and validated (see
   section 4).
3. chan-desktop runs `chan add <path>`. On non-zero exit, the stderr
   is surfaced as an inline error banner.
4. On success there is nothing more to do here: the registry file
   changed, the watcher fires, the UI re-fetches, the new row
   appears with **On = off**. The user toggles On to start serving.

`add` is a separate step from `serve`: registering without serving is
a valid state, and matches what the CLI does.

### 3.3 Toggle On (serve)

Toggling On spawns `chan serve <path> --host 127.0.0.1 --port N` as
a child process owned by chan-desktop. Port `N` is allocated as
follows:

1. If the drive's sidecar has a `last_port` from an earlier serve,
   we try to bind `127.0.0.1:last_port` first. On success we drop
   the probe socket and reuse that port. The point is to keep any
   browser tabs the user already has open on a URL that is still
   routable across a stop-then-start cycle.
2. Otherwise (or if the preferred port is taken), we bind
   `127.0.0.1:0` and let the OS assign one, then close the probe.

Either way, we persist the chosen port back into the sidecar
before spawning chan. There is a TOCTOU window between close and
chan's bind which we accept: a foreign process grabbing the port
in that window surfaces as chan exiting non-zero, which the
reader thread already handles by flipping the toggle back to off.

Note: the URL chan prints includes a per-serve bearer token
(`?t=...`), so even with a stable port the full URL changes on
restart and a stale browser tab will hit a token-mismatch on
reconnect. Closing the token gap is a separate decision (e.g.
`--no-token` for the desktop's loopback case, or chan persisting
the token across serves) tracked in section 10.

The supervisor:

- pipes stderr and tails it line by line on a dedicated thread
  per running drive,
- watches for chan's `chan is ready:` banner and captures the URL
  printed on the following line,
- stores the URL in `AppState.serves` (in-memory only; the bearer
  token rotates on every `chan serve` so a saved URL would decay
  to garbage between launches),
- emits a `serves-changed` Tauri event so the row re-renders with
  the URL field populated and the Launch button enabled,
- when the reader hits EOF (chan exited, intentionally or not),
  reaps the child, flips the sidecar `on` flag back to false, and
  emits another `serves-changed`.

When dev mode is on, the supervisor also forwards every captured
line to the frontend as a `chan-log` event with the drive's
canonical path and the line. The console window subscribes to that
stream. See section 9 (Settings).

### 3.4 Toggle Off (stop)

Calls `Child::kill`, which is SIGKILL on Unix and `TerminateProcess`
on Windows. chan does not get a chance to flush logs or unbind its
listening socket cleanly; the OS reclaims the port within seconds.

This is deliberately the prototype version. The intended endgame is
SIGTERM with a 10s grace period (what chan's own signal handler
already implements), then SIGKILL on timeout. That needs either
`libc::kill` on Unix and the win32 equivalent on Windows, or the
`nix` / `windows-sys` crates. We will pull one in when graceful
shutdown actually matters; until then, the cost is "chan's last
log line is missing" and "the listening socket sits in TIME_WAIT
for ~10s after stop".

App exit triggers a SIGKILL of every running serve via the Tauri
`RunEvent::Exit` hook, so children do not outlive the desktop
process under normal exit. Hard crashes can still orphan children;
a chan-side parent-death watchdog or a per-child PID file would
close that gap.

### 3.5 Close drive (remove)

Stops the serve (if running), then runs `chan remove <path>`. The
filesystem is untouched. The watcher fires and the row disappears
from the UI.

"Close" deliberately leaves the user's markdown folder alone. There
is no "delete drive" action in the desktop UI.

### 3.6 External changes

Anything that mutates `~/.chan/config.toml` shows up in the UI:

- `chan add` / `chan remove` / `chan rename` from a terminal,
- a second chan-desktop process opened against the same home
  directory (rare, but defined),
- the user editing the TOML by hand.

For external `chan serve` (somebody runs `chan serve ~/notes` from a
terminal, bypassing the desktop), the registry only records that
the drive *exists*; it does not record that a serve is running. So
the desktop's On toggle will not flip to on, and no URL will appear.
Detecting external serves would need either a process scan
(cross-platform PITA) or a chan-side change to write a per-drive
status file. We accept this gap rather than pay for it now.

## 4. Validation

The desktop app does not invent its own validation. It defers to
chan-core / chan helpers, both to avoid drift and so that anything
the desktop app accepts is also accepted by every other chan
surface.

- **Drive name**: not validated by the desktop at all. Names are
  read-only in the UI, so the only writer is `chan rename`, which
  enforces `chan_tunnel_proto::is_valid_drive_name` itself. If a
  pre-existing registry entry has a name that no longer validates,
  the desktop displays it as-is rather than rewriting it.
- **Path**: canonicalised via `std::fs::canonicalize` before being
  passed to `chan add` / `chan remove` / `chan serve`. We always
  invoke chan with argv as a slice (`Command::new("chan").args([...])`),
  never as a single shell-quoted string, so quoting is a non-issue;
  the remaining concern is just that we hand chan the same path the
  user sees in the UI. When canonicalisation fails (broken symlink,
  asleep network mount), we fall back to the literal path.
- **Relative path arguments inside a drive** (used later, not by the
  current UI) reuse `chan_drive::fs_ops::validate_rel`.

We will pull chan-core in as a path dep when the desktop starts
linking it directly (e.g. for the bundled-chan model in section 5.2).
Until then, validation responsibility lives entirely on the chan
side and the desktop just forwards user input.

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

## 9. Settings and developer mode

The Settings window is intentionally empty apart from one toggle:
**Developer mode**. When on:

- every `chan serve` chan-desktop spawns gets `-vv` (debug-level
  tracing) appended to its argv,
- the **console window** is shown. The console window is a third
  Tauri window (label `console`, hidden by default) that subscribes
  to the `chan-log` event stream and appends every captured line,
  prefixed with the drive's basename. Auto-scroll pins to the bottom
  unless the user has scrolled up. There is a Clear button.
- when dev mode is toggled off, the console window is hidden but
  not destroyed, and the supervisor stops emitting `chan-log`
  events. Already-running serves continue with whatever verbosity
  they were started with; the user has to toggle Off / On to pick
  up a verbosity change.

This is deliberately not browser DevTools. Browser-level debugging
the desktop UI itself is a maintainer concern, not an end-user one,
and is reachable through the standard Tauri / WebKit Inspector when
needed for development.

Future settings additions are deferred until they have concrete
demand: tunnel publishing (`--tunnel-token`, `--public`) probably
belongs in a per-drive "Share" panel rather than a global setting.

## 10. Open questions

- `chan serve` token: today we pass no `--no-token` flag, so chan
  rotates the bearer token on every serve. Combined with port
  reuse (section 3.3) the path stays stable but the query string
  changes, so a browser tab kept open across a stop-then-start
  fails its reconnect. Options: pass `--no-token` for the desktop
  loopback case (cheap, slight loss of in-machine isolation), or
  ask chan to persist the token (chan-side change, no security
  cost). Pick one once it actually bites.
- Multiple desktop windows vs one window: current design is one
  window with a drives table. Adding per-drive child windows is a
  later concern.
- Graceful stop: see section 3.4. Worth doing once we hit the first
  user-visible cost.
- Cross-platform child reaping on hard crash: see section 3.4.
  Probably solved by chan growing a parent-death watchdog rather
  than chan-desktop tracking PIDs in a sidecar file.
