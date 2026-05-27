# chan-desktop design

This document is the source of truth for what chan-desktop is and is
not. It is intentionally light on Rust / Tauri specifics and heavy on
business logic. When the implementation drifts from this doc, fix one
of the two.

## 1. Purpose

chan-desktop is the native desktop shell for chan. For normal local
workspaces it embeds chan-server in the desktop process and serves the
same Svelte editor on a loopback HTTP port. It links `chan-workspace` and
`chan-server` directly and ships no `chan` binary; registry mutations
and feature toggles run in-process against the embedded `chan-workspace`
`Library`. The desktop app exists so that:

- a non-CLI user can install one signed bundle and open a folder
  through a familiar OS dialog instead of a terminal,
- multiple workspaces can be supervised at once, with one app window
  acting as the inventory and on/off control,
- local embedded workspaces and explicit remote attachments share the
  same editor window model.

Non-goals:

- chan-desktop is not a second editor. The editor is the web app
  served by chan-server. The desktop window manages workspaces and opens
  the editor in a Tauri webview.
- chan-desktop is not a `chan` CLI installer. It links `chan-workspace`
  and `chan-server` directly rather than shipping a `chan` binary, so
  installing the desktop does not put `chan` on your `$PATH`.
- chan-desktop is not a general web browser. Workspace windows are
  dedicated Tauri webviews pointed at local or attached chan URLs.

## 2. Mental model

One desktop process can host many running local workspaces:

```
                +-------------------+
   user -->     |  chan-desktop     |   launcher plus workspace
                |  (supervisor)     |   user session
                +---------+---------+
                          | embeds
                          v
                +-------------------+
                |  WorkspaceHost        |   many local workspaces
                |  (HTTP + WS)      |
                +---------+---------+
                          | http://127.0.0.1:PORT/?t=TOKEN
                          v
                    Tauri webview
```

There are three workspace attachment modes:

- **Local embedded**: a local registry entry opened by
  chan-desktop. The desktop mounts the workspace into its embedded
  `WorkspaceHost` and owns the runtime.
- **Remote outbound**: an already-running chan server that
  chan-desktop opens by URL. Example: the user runs
  `chan serve /tmp/foo`, then adds that token-bearing URL to the
  desktop's remote-workspace outbound config. The desktop owns only the
  window, not the server.
- **Remote inbound**: chan-desktop listens on a loopback tunnel
  endpoint and an external `chan serve` connects to it. Example:
  the desktop listens on `127.0.0.1:9999`, then the user runs
  `chan serve /tmp/foo --tunnel-url=http://127.0.0.1:9999`. The
  desktop owns the listener and per-workspace webview, not the remote
  server.

There is no fallback serve mode. If a user wants to run
`chan serve` directly, that is a remote attachment, even when the
server is on the same machine.

## 3. Workspace lifecycle

### 3.0 Source of truth

The `chan` registry at `~/.chan/config.toml` is the single source of
truth for the set of known workspaces and their display names.
chan-desktop treats that registry as the source of truth. Normal
user-driven mutations go through `chan add` / `chan remove` (and,
later, `chan rename`). The first-launch default-workspace path is the
only current exception: it calls `chan-workspace` directly to create and
register `Documents/Chan` before the launcher UI renders.

The desktop owns a small config of its own at the
platform-appropriate path. It holds desktop-only state such as
feature-toggle cache, tunnel preferences, and closed-window
restore data. Nothing about whether a local workspace is currently
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

### 3.1 Workspace row state

A "workspace" in chan-desktop maps 1:1 to a known workspace in the `chan`
registry. Visible state per workspace in the inventory:

| column  | meaning                                             |
|---------|-----------------------------------------------------|
| On/type | local On toggle, `tunnel` tag, or outbound URL tag  |
| Path    | local path, inbound label, or outbound URL label    |
| Actions | Open split, feature toggle, browser open, forget    |

Names are deliberately read-only in the desktop. Renaming a workspace is
done by running `chan rename` from a terminal; the watcher reflects
the new name on the next debounce tick. The alternative was a
write-through to `chan rename` from the desktop UI, which we
rejected for now to keep the registry-as-SoT contract one-way and
the data flow obvious.

### 3.2 First launch and Open workspace

On a fresh desktop launch with empty chan metadata, chan-desktop
creates the platform default workspace at `Documents/Chan`, seeds the
embedded `docs/manual/` tree into it, registers it through
`chan-workspace`, and opens it through the embedded local server.

When an existing registry has workspaces but no default workspace, the
launcher prompts once per process to choose an existing registered
workspace or create `Documents/Chan`. Choosing an existing workspace only
sets `default_drive_root`; it does not start, stop, move, or delete
anything. Creating `Documents/Chan` registers and opens that new
workspace.

When the registered default `Documents/Chan` path is missing, the
launcher requires an explicit factory-reset confirmation before it
clears chan metadata on this machine. The reset keeps user note
folders outside chan metadata untouched, recreates `Documents/Chan`,
seeds the manual, registers it, and opens it through the embedded
local server.

The "Open workspace" button still registers a user-chosen folder.

1. Tauri opens a native folder picker.
2. The selected path is canonicalised and validated (see
   section 4).
3. chan-desktop registers the path through `chan-workspace` in-process.
   On failure the error is surfaced as an inline banner.
4. On success the desktop immediately starts the local runtime for
   the new workspace (see section 3.3). The registry watcher fires, the
   UI re-fetches, the new row appears with **On = on**, and the URL
   column populates from the embedded handle.

The auto-start is specific to "Open workspace" from the desktop UI:
the user's intent there is "make this workspace usable now". Adding a
workspace from a terminal (`chan add`) only registers it; the desktop
shows the new row with On = off, the same as for any pre-existing
registry entry. Registering without serving is still a valid state
in the model; we just don't make the desktop pick it.

### 3.3 Toggle On (serve)

Toggling On opens the workspace through the embedded chan-server
`WorkspaceHost`. The desktop owns one loopback listener and mounts each
workspace under a distinct path prefix. Each mounted workspace gets
isolated AppState, watcher, indexer, terminal registry, MCP bridge,
control socket, and token state.

Embedded local serving keeps chan-server's bearer token gate
enabled. The desktop webview receives the token-bearing URL and
the SPA stores the token in sessionStorage.

The local runtime:

- stores the URL in `AppState.serves` in memory only,
- emits a `serves-changed` Tauri event so the row re-renders with
  the URL field populated and the Launch button enabled,
- opens one Tauri workspace webview automatically, with additional
  Launch clicks opening more windows for the same runtime,
- closes all workspace windows when the local runtime is toggled off.

### 3.4 Toggle Off (stop)

Toggle Off closes the mounted workspace in WorkspaceHost and tears down its
workspace windows. App exit calls the same stop path for every active
local runtime.

### 3.5 Close workspace (remove)

Stops the serve (if running), then unregisters the workspace through
`chan-workspace` in-process. The filesystem is untouched. The watcher
fires and the row disappears from the UI.

"Close" deliberately leaves the user's markdown folder alone. There
is no "delete workspace" action in the desktop UI.

### 3.6 External changes

Anything that mutates `~/.chan/config.toml` shows up in the UI:

- `chan add` / `chan remove` / `chan rename` from a terminal,
- a second chan-desktop process opened against the same home
  directory (rare, but defined),
- the user editing the TOML by hand.

For external `chan serve` (somebody runs `chan serve ~/notes` from a
terminal, bypassing the desktop), the registry only records that
the workspace exists; it does not record that a serve is running. The
desktop's local On toggle will not flip to on, and no URL will
appear. A user who wants that server in the desktop adds it through
the remote outbound config using the server's URL.

### 3.7 File Browser export drag-out

The File Browser keeps its browser drag payloads so web use and
in-app tree moves keep working. In chan-desktop, the same drag start
also calls the `start_file_browser_drag_out` Tauri command.

The command treats the server as the content boundary. It fetches
the token-bearing `/api/files/<path>?download=1` URL that the web
client already uses for right-click Download and browser drag-out,
then streams the response into a staged file under the OS temp
directory. Files use the basename reported by the server download
header, with the frontend fallback name as a backup. Directories
stage the server's `.tar` archive so the exported tree shape is
preserved inside the archive.

On macOS the staged file is passed to AppKit as a native file drag.
Tauri filesystem code does not read the workspace root. Failed or
cancelled drags remove the staging directory immediately. Accepted
drags use bounded cleanup, and later drag starts sweep stale staging
directories.

## 4. Validation

The desktop app avoids inventing durable validation rules. It
defers to chan-workspace or the `chan` CLI where those surfaces already
own a contract, both to avoid drift and so that anything the desktop
app accepts is also accepted by every other chan surface.

- **Workspace name**: not validated by the desktop at all. Names are
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
- **Relative path arguments inside a workspace** (used later, not by the
  current UI) reuse `chan_workspace::fs_ops::validate_rel`.

Desktop links chan-workspace and chan-server for embedded local serving.
Registry mutations and feature flips run in-process against the same
embedded `chan-workspace` `Library`, not through a separate `chan` CLI.

## 5. Self-contained runtime

chan-desktop is self-contained. It links `chan-workspace` and
`chan-server` directly and embeds the web bundle (`web/dist`) via
rust-embed at build time. No `chan` binary is shipped in the app
bundle, and none is required at runtime.

Local workspaces open through the embedded chan-server `WorkspaceHost`, which
owns a single `chan_workspace::Library`. Every registry mutation (add,
remove, default-workspace reconciliation) and feature toggle (semantic
search, reports) runs in-process against that `Library`, or against
the live `Arc<Workspace>` the host already holds for a mounted workspace.
Routing through one shared registry is what keeps a freshly-added
workspace openable immediately: a subprocess `chan add` would mutate only
the on-disk registry and leave the host's in-memory snapshot stale.

The single codesigned and notarised artifact is the chan-desktop
`.app` itself; there is no second binary to sign. External
`chan serve` processes are still supported, but as explicit remote
attachments over a separate transport (see section 10), not as a
local serving dependency.

## 6. Power users and the CLI tool

Non-goal: chan-desktop installation should be "drag Chan.app to
/Applications". No installer, no scripts.

chan-desktop ships no `chan` binary, so installing it does not put
the CLI on your `$PATH`. Power users who want `chan serve` or
shell-first workflows install the standalone `chan` separately (the
`chan.app/install.sh` installer or a release tarball). The desktop
app and the standalone CLI are independent installs that share the
same `~/.chan` registry, so a workspace added by one shows up in the
other.

Windows: deferred with the rest of Windows desktop support (see
section 7). When Windows desktop support returns, the installer can
offer to add the CLI to PATH with an opt-out checkbox.

## 7. Distribution

The download entry point is https://chan.app/install, which links to
release assets hosted on GitHub Releases. The desktop release workflow
(`.github/workflows/release-desktop.yml`) produces:

- macOS arm64: notarised DMG containing `Chan.app`. Drag to
  /Applications.
- Linux x86_64: `.deb` and `.AppImage`.

Both are built in CI: the Linux artifacts on an Ubuntu runner
(unsigned), the macOS DMG on a macOS runner where the Developer ID
identity is imported from secrets and the bundle is signed and
notarised.

Windows desktop builds are deferred. The bundler config still carries
a Windows target, but no Windows artifact is built or published yet
and the Authenticode signing lane is not open; Windows returns as a
distribution channel when that lane lands.

Cargo install (`cargo install chan-desktop`) builds the self-contained
desktop from source, for contributors and packagers rather than end
users. The README points end users at chan.app.

## 8. Self-upgrade

chan-desktop updates itself through `tauri-plugin-updater`. The plugin
is wired in `src-tauri/src/main.rs` and gated by the `updater:*`
capabilities; it can check for, download, and install a newer signed
bundle.

- Update bundles are verified with a minisign signature. The
  production public key is embedded in `src-tauri/tauri.conf.json`
  under `plugins.updater.pubkey`; the matching private key lives
  outside the repo in the release owner's secret store.
- The client probes the manifest endpoint
  `https://chan.app/dl/desktop/{{target}}/{{current_version}}/latest.json`.
  Server-side publishing of that manifest is owned by chan-prod-setup.
- Because the desktop ships no separate `chan` binary, there is no
  second executable to upgrade and no in-bundle update banner to
  suppress.

Key rotation and the bridge-release procedure are documented in
`desktop/CLAUDE.md` ("Auto-upgrade signing") and the
[`updater-bridge.md`](updater-bridge.md) runbook.

## 9. Settings and developer controls

chan owns the Settings surface per workspace. The desktop menu item
dispatches `app.settings.toggle` into the focused workspace webview; it
is a no-op when focus is not inside a workspace window.

Maintainer controls stay native:

- Cmd+R / Ctrl+R reloads the focused workspace webview.
- Cmd+Opt+I / Ctrl+Alt+I opens webview DevTools.
- Cmd+Shift+N opens another launcher window.

Future global settings additions are deferred until they have
concrete demand. Tunnel publishing belongs in the workspace attachment
surface rather than a generic app settings page.

## 10. Remote workspaces

Remote workspaces are explicit attachments. They are not a fallback for
failed embedded local serving.

### 10.1 Outbound URL attach

Outbound attach means the server already exists and chan-desktop
opens it by URL. Example:

```
chan serve /tmp/foo
```

The user copies the printed URL, including the bearer token, into a
remote-workspace outbound config in chan-desktop. The desktop opens that
URL in a workspace webview and does not try to start, stop, reclaim, or
inspect the server process. This works whether the URL points at
another machine or at `127.0.0.1` on the same machine.

### 10.2 Inbound tunnel attach

chan-desktop embeds the `chan-tunnel-server` workspace crate so a
remote `chan serve` can register a workspace over an SSH tunnel and show up in
Workspace Manager alongside embedded local workspaces. The remote workspace
opens in a regular workspace webview window pointed at a loopback URL on
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
   +-- Arc<Registry>: (label, workspace) -> TunnelHandle
   |
   +-- per-tenant axum listener  127.0.0.1:<port>
        GET /<workspace>/...  ->  PrependPathLayer  ->  public_router
                              (sees /<label>/<workspace>/...)
```

The user opens an SSH session like:

```
ssh -R 7777:localhost:7777 remote-host
# on remote
chan serve PATH \
  --tunnel-url=http://127.0.0.1:7777 \
  --tunnel-token=<label> \
  --tunnel-drive=<workspace>
```

`<label>` is opaque to the protocol; chan-desktop returns it
verbatim as the validated username and renders it in Workspace Manager.
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
  open `127.0.0.1:7777` can register a workspace under any label.
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
  `/:user/:workspace/*rest` match still picks the registered handle.
  The prepended segment is captured at listener bring-up from the
  desktop's tenant string; it is never derived from any request
  byte.

### 10.5 Lifecycle

- Explicit start. Boot does not bind anything; the user clicks
  "Listen..." in the Workspace Manager header to open a panel that
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
  last workspace deregisters. Polling is fine for the tiny set
  involved; promote to a notify channel if this ever shows up in
  a profile. On every newly-observed `(label, workspace)` the
  supervisor emits `tunneled-workspace-ready { label, workspace, url }`,
  which the frontend uses to auto-launch the editor for the
  freshly-registered workspace in the system browser.
- Eviction is upstream's last-writer-wins: two `chan serve`
  instances registering `(label, workspace)` collapse to the most
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
above the workspaces table. While idle the panel offers a port input
and Start button; while listening it shows the bound port, a
copy-on-click `ssh -R` snippet, a copy-on-click `chan serve`
snippet, and a Stop button.

A tunneled workspace row in Workspace Manager has:

- A `tunnel` tag in the On column (no toggle; the remote owns
  the lifecycle).
- Label in the Path column (no real path).
- Workspace name from the Hello frame.
- URL = `http://127.0.0.1:<port>/<workspace>/`, Launch button opens
  it in the default browser.
- No Close button; closing a tunneled workspace means shutting down
  `chan serve` on the remote.

Newly-registered workspaces auto-open in the system browser via the
`tunneled-workspace-ready` event so the user doesn't have to click
Launch for the first registration.

### 10.7 Deferred

- Per-tenant listener port persistence across Stop/Start cycles
  within one session, and across desktop restarts. Open browser
  tabs on a tunneled workspace lose their target on Stop.
- Pre-shared secret in the token to raise the bar above
  "any local process can register". Add when shared / multi-user.
- Notify-channel from registry (replaces 500 ms polling).
- Surfacing Hello.public on the UI explicitly (currently in the
  row tooltip only).
- In-app Tauri WebviewWindow for tunneled workspaces (currently the
  auto-launch and Launch button open the system browser, while
  embedded local workspaces get an in-app webview).

## 11. Open questions

- Multiple desktop windows vs one window: current design is one
  window with a workspaces table. Adding per-workspace child windows is a
  later concern.
