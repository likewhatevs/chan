# chan-desktop design

This document is the source of truth for what chan-desktop is and is
not. It is intentionally light on Rust / Tauri specifics and heavy on
business logic. When the implementation drifts from this doc, fix one
of the two.

## 1. Purpose

chan-desktop is the native desktop shell for chan. For normal local
drives it embeds chan-server in the desktop process and serves the
same Svelte editor on a loopback HTTP port. The `chan` CLI remains
the compatibility path for registry mutations and feature toggles
until those surfaces have desktop-native APIs. The desktop app
exists so that:

- a non-CLI user can install one signed bundle and open a folder
  through a familiar OS dialog instead of a terminal,
- multiple drives can be supervised at once, with one app window
  acting as the inventory and on/off control,
- local embedded drives and explicit remote attachments share the
  same editor window model.

Non-goals:

- chan-desktop is not a second editor. The editor is the web app
  served by chan-server. The desktop window manages drives and opens
  the editor in a Tauri webview.
- chan-desktop is not a packaging tool for `chan`. It bundles a
  pinned `chan` binary and treats it as an internal dependency, not
  a user-facing CLI install path.
- chan-desktop is not a general web browser. Drive windows are
  dedicated Tauri webviews pointed at local or attached chan URLs.

## 2. Mental model

One desktop process can host many running local drives:

```
                +-------------------+
   user -->     |  chan-desktop     |   launcher plus drive
                |  (supervisor)     |   user session
                +---------+---------+
                          | embeds
                          v
                +-------------------+
                |  DriveHost        |   many local drives
                |  (HTTP + WS)      |
                +---------+---------+
                          | http://127.0.0.1:PORT/?t=TOKEN
                          v
                    Tauri webview
```

There are three drive attachment modes:

- **Local embedded**: a local registry entry opened by
  chan-desktop. The desktop mounts the drive into its embedded
  `DriveHost` and owns the runtime.
- **Remote outbound**: an already-running chan server that
  chan-desktop opens by URL. Example: the user runs
  `chan serve /tmp/foo`, then adds that token-bearing URL to the
  desktop's remote-drive outbound config. The desktop owns only the
  window, not the server.
- **Remote inbound**: chan-desktop listens on a loopback tunnel
  endpoint and an external `chan serve` connects to it. Example:
  the desktop listens on `127.0.0.1:9999`, then the user runs
  `chan serve /tmp/foo --tunnel-url=http://127.0.0.1:9999`. The
  desktop owns the listener and per-drive webview, not the remote
  server.

There is no fallback serve mode. If a user wants to run
`chan serve` directly, that is a remote attachment, even when the
server is on the same machine.

## 3. Drive lifecycle

### 3.0 Source of truth

The `chan` registry at `~/.chan/config.toml` is the single source of
truth for the set of known drives and their display names.
chan-desktop treats that registry as the source of truth. Normal
user-driven mutations go through `chan add` / `chan remove` (and,
later, `chan rename`). The first-launch default-drive path is the
only current exception: it calls `chan-drive` directly to create and
register `Documents/Chan` before the launcher UI renders.

The desktop owns a small config of its own at the
platform-appropriate path. It holds desktop-only state such as
feature-toggle cache, tunnel preferences, and closed-window
restore data. Nothing about whether a local drive is currently
*running* is persisted: the On column in the UI is derived live
from the in-memory map of active local runtimes, so a desktop
restart comes up with everything off and there is no chance of a
stale on=true sticking after a crash.

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

| column  | meaning                                             |
|---------|-----------------------------------------------------|
| On/type | local On toggle, `tunnel` tag, or outbound URL tag  |
| Path    | local path, inbound label, or outbound URL label    |
| Actions | Open split, feature toggle, browser open, forget    |

Names are deliberately read-only in the desktop. Renaming a drive is
done by running `chan rename` from a terminal; the watcher reflects
the new name on the next debounce tick. The alternative was a
write-through to `chan rename` from the desktop UI, which we
rejected for now to keep the registry-as-SoT contract one-way and
the data flow obvious.

### 3.2 First launch and Open drive

On a fresh desktop launch with empty chan metadata, chan-desktop
creates the platform default drive at `Documents/Chan`, seeds the
embedded `docs/manual/` tree into it, registers it through
`chan-drive`, and opens it through the embedded local server.

When an existing registry has drives but no default drive, the
launcher prompts once per process to choose an existing registered
drive or create `Documents/Chan`. Choosing an existing drive only
sets `default_drive_root`; it does not start, stop, move, or delete
anything. Creating `Documents/Chan` registers and opens that new
drive.

When the registered default `Documents/Chan` path is missing, the
launcher requires an explicit factory-reset confirmation before it
clears chan metadata on this machine. The reset keeps user note
folders outside chan metadata untouched, recreates `Documents/Chan`,
seeds the manual, registers it, and opens it through the embedded
local server.

The "Open drive" button still registers a user-chosen folder.

1. Tauri opens a native folder picker.
2. The selected path is canonicalised and validated (see
   section 4).
3. chan-desktop runs `chan add <path>`. On non-zero exit, the stderr
   is surfaced as an inline error banner.
4. On success the desktop immediately starts the local runtime for
   the new drive (see section 3.3). The registry watcher fires, the
   UI re-fetches, the new row appears with **On = on**, and the URL
   column populates from the embedded handle.

The auto-start is specific to "Open drive" from the desktop UI:
the user's intent there is "make this drive usable now". Adding a
drive from a terminal (`chan add`) only registers it; the desktop
shows the new row with On = off, the same as for any pre-existing
registry entry. Registering without serving is still a valid state
in the model; we just don't make the desktop pick it.

### 3.3 Toggle On (serve)

Toggling On opens the drive through the embedded chan-server
`DriveHost`. The desktop owns one loopback listener and mounts each
drive under a distinct path prefix. Each mounted drive gets
isolated AppState, watcher, indexer, terminal registry, MCP bridge,
control socket, and token state.

Embedded local serving keeps chan-server's bearer token gate
enabled. The desktop webview receives the token-bearing URL and
the SPA stores the token in sessionStorage.

The local runtime:

- stores the URL in `AppState.serves` in memory only,
- emits a `serves-changed` Tauri event so the row re-renders with
  the URL field populated and the Launch button enabled,
- opens one Tauri drive webview automatically, with additional
  Launch clicks opening more windows for the same runtime,
- closes all drive windows when the local runtime is toggled off.

### 3.4 Toggle Off (stop)

Toggle Off closes the mounted drive in DriveHost and tears down its
drive windows. App exit calls the same stop path for every active
local runtime.

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
the drive exists; it does not record that a serve is running. The
desktop's local On toggle will not flip to on, and no URL will
appear. A user who wants that server in the desktop adds it through
the remote outbound config using the server's URL.

## 4. Validation

The desktop app avoids inventing durable validation rules. It
defers to chan-drive or the `chan` CLI where those surfaces already
own a contract, both to avoid drift and so that anything the desktop
app accepts is also accepted by every other chan surface.

- **Drive name**: not validated by the desktop at all. Names are
  read-only in the UI, so the only writer is `chan rename`, which
  enforces `chan_tunnel_proto::is_valid_drive_name` itself. If a
  pre-existing registry entry has a name that no longer validates,
  the desktop displays it as-is rather than rewriting it.
- **Path**: canonicalised via `std::fs::canonicalize` before being
  passed to `chan add` / `chan remove` / embedded open. We always
  invoke chan with argv as a slice
  (`Command::new("chan").args([...])`), never as a single
  shell-quoted string, so quoting is a non-issue; the remaining
  concern is just that we hand chan the same path the user sees in
  the UI. When canonicalisation fails (broken symlink, asleep
  network mount), we fall back to the literal path.
- **Relative path arguments inside a drive** (used later, not by the
  current UI) reuse `chan_drive::fs_ops::validate_rel`.

Desktop now links chan-drive and chan-server for embedded local
serving. Registry mutations and feature flips still go through the
`chan` CLI until their desktop-native replacements land.

## 5. The chan binary

chan-desktop links `chan-drive` and `chan-server` for embedded local
serving. It still calls out to a pinned `chan` executable for CLI
surfaces that remain owned by the binary: registry mutations,
feature toggles, update checks, and the hidden MCP proxy command.

The release `.app` / `.exe` / Linux bundle ships a pinned `chan`
binary inside the bundle. Runtime resolution is PATH-first only when
the PATH binary's version matches chan-desktop exactly; otherwise it
uses the bundled binary. This is a deliberate choice:

- Pro: deterministic. One desktop release pairs with exactly one
  `chan` build. The test matrix is "did this DMG work" rather than
  "what version of `chan` does the user have".
- Pro: signed and notarised together; macOS Gatekeeper sees one
  bundle.
- Con: every chan release requires a new desktop release, even if
  the desktop UI did not change. Acceptable while both are
  pre-1.0 and ship together.

The bundled `chan` binary lives at:

- macOS:   `ChanDesktop.app/Contents/Resources/bin/chan`
- Linux:   alongside the desktop binary (`/usr/lib/chan-desktop/chan`
           when packaged, or next to the binary in a tarball)
- Windows: `chan.exe` in the install directory

The binary is not a local serving fallback. Running `chan serve`
directly is an explicit remote attachment from the desktop's point
of view.

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

## 9. Settings and developer controls

chan owns the Settings surface per drive. The desktop menu item
dispatches `app.settings.toggle` into the focused drive webview; it
is a no-op when focus is not inside a drive window.

Maintainer controls stay native:

- Cmd+R / Ctrl+R reloads the focused drive webview.
- Cmd+Opt+I / Ctrl+Alt+I opens webview DevTools.
- Cmd+Shift+N opens another launcher window.

Future global settings additions are deferred until they have
concrete demand. Tunnel publishing belongs in the drive attachment
surface rather than a generic app settings page.

## 10. Remote drives

Remote drives are explicit attachments. They are not a fallback for
failed embedded local serving.

### 10.1 Outbound URL attach

Outbound attach means the server already exists and chan-desktop
opens it by URL. Example:

```
chan serve /tmp/foo
```

The user copies the printed URL, including the bearer token, into a
remote-drive outbound config in chan-desktop. The desktop opens that
URL in a drive webview and does not try to start, stop, reclaim, or
inspect the server process. This works whether the URL points at
another machine or at `127.0.0.1` on the same machine.

### 10.2 Inbound tunnel attach

chan-desktop embeds `chan-tunnel-server` from chan-core so a remote
`chan serve` can register a drive over an SSH tunnel and show up in
Drive Manager alongside embedded local drives. The remote drive
opens in a regular drive webview window pointed at a loopback URL on
the laptop; the request body rides yamux substreams back through the
SSH tunnel to the remote `chan serve`.

The same shape also works on one machine for local testing:

```
# in chan-desktop: listen on 127.0.0.1:9999
chan serve /tmp/foo --tunnel-url=http://127.0.0.1:9999
```

### 10.3 Topology

```
laptop (chan-desktop)               remote host
---------------------               -----------
tunnel listener   127.0.0.1:7777  <- ssh -R 7777:localhost:7777
   |
   +-- Arc<Registry>: (label, drive) -> TunnelHandle
   |
   +-- per-tenant axum listener  127.0.0.1:<port>
        GET /<drive>/...  ->  PrependPathLayer  ->  public_router
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
`-` and `_`, <=64, first char alphanumeric.

### 10.4 Security boundary

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

### 10.5 Lifecycle

- Explicit start. Boot does not bind anything; the user clicks
  "Listen..." in the Drive Manager header to open a panel that
  accepts an optional port (`0` / blank = OS-assigned). Clicking
  Start invokes `tunnel_start`, which binds `127.0.0.1:<port>`
  and spawns both the tunnel accept loop and the supervisor.
  The actual bound port plus a `ssh -R` snippet and a sample
  `chan serve` command appear in the same panel.
- Persistence is limited to the user's preferred port (saved in
  desktop config so the input is pre-filled on next launch).
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
  shutdown runs unconditionally.

### 10.6 UI

Header strip: a "Listen..." button toggles an inline tunnel panel
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

### 10.7 Deferred

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
  embedded local drives get an in-app webview).

## 11. Open questions

- Multiple desktop windows vs one window: current design is one
  window with a drives table. Adding per-drive child windows is a
  later concern.
