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
platform-appropriate path. It holds the global `dev_mode` flag and,
per drive (keyed by canonical path), the last port that drive's
`chan serve` bound to. Nothing about whether a drive is currently
*running* is persisted: the On column in the UI is derived live
from the in-memory map of supervised serves, so a desktop restart
comes up with everything off and there is no chance of a stale
on=true sticking after chan died unexpectedly.

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
4. On success the desktop immediately spawns `chan serve` for the
   new drive (see section 3.3). The registry watcher fires, the UI
   re-fetches, the new row appears with **On = on**, and the URL
   column populates as soon as chan prints its ready banner.

The auto-start is specific to "Open drive" from the desktop UI:
the user's intent there is "make this drive usable now". Adding a
drive from a terminal (`chan add`) only registers it; the desktop
shows the new row with On = off, the same as for any pre-existing
registry entry. Registering without serving is still a valid state
in the model; we just don't make the desktop pick it.

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

We pass `--no-token` to every `chan serve` the desktop spawns. The
serve binds to 127.0.0.1, the desktop user already trusts every
process on their own machine, and the rotating bearer token only
bought us URL churn that broke browser tabs on every restart.
Combined with port reuse this means a tab kept open across a
toggle-Off / toggle-On lands on a still-routable URL and chan's
WebSocket reconnect succeeds.

This applies only to desktop-spawned serves. A terminal-initiated
`chan serve` is outside our control and keeps chan's default
token-issuing behaviour.

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

## 10. Tunneled drives

chan-desktop embeds `chan-tunnel-server` from chan-core so a remote
`chan serve` can register a drive over an SSH tunnel and show up in
Drive Manager alongside locally-supervised drives. The remote drive
opens in a regular drive webview window pointed at a loopback URL on
the laptop; the request body rides yamux substreams back through the
SSH tunnel to the remote `chan serve`.

### 10.1 Topology

```
laptop (chan-desktop)               remote host
─────────────────────               ───────────
tunnel listener   127.0.0.1:7777  <── ssh -R 7777:localhost:7777
   │
   ├─ Arc<Registry>: (label, drive) → TunnelHandle
   │
   └─ per-tenant axum listener  127.0.0.1:<port>
        GET /<drive>/...  →  PrependPathLayer  →  public_router
                              (sees /<label>/<drive>/...)
```

The user opens an SSH session like:

```
ssh -R 7777:localhost:7777 remote-host
# on remote
chan serve PATH \
  --tunnel-url=http://127.0.0.1:7777 \
  --tunnel-token=<label> \
  --tunnel-drive=<drive>
```

`<label>` is opaque to the protocol; chan-desktop returns it
verbatim as the validated username and renders it in Drive Manager.
A natural convention is `<hostname>-<osuser>` (e.g. `alex-laptop`),
which keeps multiple machines distinguishable. Charset is
`chan_tunnel_proto::is_valid_username`: ASCII alphanumeric plus
`-` and `_`, ≤64, first char alphanumeric.

### 10.2 Security boundary

Both the tunnel listener and every per-tenant listener bind
`127.0.0.1` only. There is no config knob to change the bind host.

- The tunnel listener speaks h2c (cleartext). The SSH `-R` forward
  is what makes that safe; sshd's default `GatewayPorts no` keeps
  the remote's `:7777` reachable only by processes on the remote
  host itself. Operators who flip `GatewayPorts yes` expose
  `:7777` to the remote's network and should know what they're
  doing.
- The bearer token is the tenant label; there is no shared secret
  and no mapping table. Any local process on the laptop that can
  open `127.0.0.1:7777` can register a drive under any label.
  This matches the OS process-trust boundary of a single-user
  desktop app: every local process can already read your files.
  Raising the bar (a config-file secret) is deferred to when this
  surface gets shared between users.
- Per-tenant listeners use separate origins (`127.0.0.1:<portA>`
  vs `127.0.0.1:<portB>`), so the browser's same-origin policy
  delivers cross-tenant isolation analogous to wildcard subdomain
  isolation in production. JS served from one tenant cannot fetch
  from another.
- The audited `chan_tunnel_server::public_router` is used
  unchanged. A thin path-rewrite layer prepends `/<label>` to every
  incoming request before routing, so the upstream's
  `/:user/:drive/*rest` match still picks the registered handle.
  The prepended segment is captured at listener bring-up from the
  desktop's tenant string; it is never derived from any request
  byte.

### 10.3 Lifecycle

- Explicit start. Boot does not bind anything; the user clicks
  "Listen…" in the Drive Manager header to open a panel that
  accepts an optional port (`0` / blank = OS-assigned). Clicking
  Start invokes `tunnel_start`, which binds `127.0.0.1:<port>`
  and spawns both the tunnel accept loop and the supervisor.
  The actual bound port plus a `ssh -R` snippet and a sample
  `chan serve` command appear in the same panel.
- Persistence is limited to the user's preferred port (saved in
  the sidecar config so the input is pre-filled on next launch).
  The listening state itself is NOT persisted: every desktop
  start comes up off.
- A supervisor task polls the registry every 500 ms and reconciles
  the per-tenant listener set: spin up a fresh `127.0.0.1:0` axum
  listener when a new label first appears, tear it down when its
  last drive deregisters. Polling is fine for the tiny set
  involved; promote to a notify channel if this ever shows up in
  a profile. On every newly-observed `(label, drive)` the
  supervisor emits `tunneled-drive-ready { label, drive, url }`,
  which the frontend uses to auto-launch the editor for the
  freshly-registered drive in the system browser.
- Eviction is upstream's last-writer-wins: two `chan serve`
  instances registering `(label, drive)` collapse to the most
  recent. The previous yamux connection is closed; the client
  reconnects with backoff per `chan-tunnel-client`. UI shows the
  current `peer_addr` and `connected_at` so collisions are
  visible.
- Stop tears down the tunnel listener, the supervisor, and every
  per-tenant listener via a cascading cancel token. The registry
  empties as yamux connections close. On app exit the same
  shutdown runs unconditionally so children don't outlive the
  desktop process.

### 10.4 UI

Header strip: a "Listen…" button toggles an inline tunnel panel
above the drives table. While idle the panel offers a port input
and Start button; while listening it shows the bound port, a
copy-on-click `ssh -R` snippet, a copy-on-click `chan serve`
snippet, and a Stop button.

A tunneled drive row in Drive Manager has:

- A `tunnel` tag in the On column (no toggle; the remote owns
  the lifecycle).
- Label in the Path column (no real path).
- Drive name from the Hello frame.
- URL = `http://127.0.0.1:<port>/<drive>/`, Launch button opens
  it in the default browser.
- No Close button; closing a tunneled drive means shutting down
  `chan serve` on the remote.

Newly-registered drives auto-open in the system browser via the
`tunneled-drive-ready` event so the user doesn't have to click
Launch for the first registration.

### 10.5 Deferred

- Per-tenant listener port persistence across Stop/Start cycles
  within one session, and across desktop restarts. Open browser
  tabs on a tunneled drive lose their target on Stop.
- Pre-shared secret in the token to raise the bar above
  "any local process can register". Add when shared / multi-user.
- Notify-channel from registry (replaces 500 ms polling).
- Surfacing Hello.public on the UI explicitly (currently in the
  row tooltip only).
- In-app Tauri WebviewWindow for tunneled drives (currently the
  auto-launch and Launch button open the system browser, while
  locally-supervised drives get an in-app webview).

## 11. Open questions

- (resolved) `chan serve` token: desktop-spawned serves pass
  `--no-token`. See section 3.3.
- Multiple desktop windows vs one window: current design is one
  window with a drives table. Adding per-drive child windows is a
  later concern.
- Graceful stop: see section 3.4. Worth doing once we hit the first
  user-visible cost.
- Cross-platform child reaping on hard crash: see section 3.4.
  Probably solved by chan growing a parent-death watchdog rather
  than chan-desktop tracking PIDs in a sidecar file.
