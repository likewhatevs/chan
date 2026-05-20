# chan-desktop drive-onboarding workflow redesign (note, not dispatched)

Author: @@Architect
Date: 2026-05-20

Status: **note, parked for later**. Captures @@Alex's
2026-05-20 sketch of a redesigned chan-desktop launcher
flow that adds first-class remote-drive support (both
outbound + inbound) alongside the existing local-drive
flow. Not Round-2-patch territory; sits in the
round-3-or-later queue.

## Source ask (paraphrased from chat 2026-05-20)

Replace today's launcher buttons ("Open drive" + "Attach")
with a single `[new]` entry point that branches by drive
type:

* **Local** → prompt for a directory (pick existing or
  create new). Same as today's local-drive add.
* **Remote outbound** → prompt for a remote URL (the
  remote chan instance to connect OUT to). Configure
  retry loop + backoff. Shows in the drives list with
  `host:port` instead of a filesystem path.
* **Remote inbound** → prompt for a host + port to
  listen on (default `127.0.0.1` / `::1` + chan-serve
  default port). Equivalent to running
  `chan serve --tunnel-url` locally and accepting
  inbound connections from a remote chan-tunnel
  gateway. Also added to the drives list.

Cross-cutting changes to the drive list UI:

* **New "windows open" column** — count across all
  three drive types. Surfaces how many windows are
  currently mounted against each drive.
* **"Open" → "Boot"** — rename the action button. For
  local drives "Boot" still means "start chan serve +
  open a window". For remote-outbound it means
  "connect out + open a window". For remote-inbound
  it means "start the listener + open a window".
* **Gear icon next to Boot** — opens a config dialog
  that is the **same** as the creation dialog,
  parameterized by drive type (path for local, URL for
  outbound, host+port for inbound).
* **Forget drive** — a destructive affordance with an
  explanatory blurb naming exactly what it deletes
  ("deletes chan's local metadata for this drive — the
  drive content itself is not touched"). Mirrors the
  semantic distinction between `chan remove` (registry
  unregister) and `rm -rf` (content delete).

## Mapping to current code + crates

Today's surfaces:

* `chan-desktop` launcher window: per-window state keyed
  by `w=<window-label>` URL parameter. Window creation
  in `desktop/src-tauri/src/serve.rs`.
* `chan list` / `chan add` / `chan remove`: drive
  registry CLI (in `crates/chan/src/main.rs`).
* `chan-tunnel-{proto, client, server}`: the existing
  h2/yamux drive-tunnel infrastructure. `chan-server`
  pulls `chan-tunnel-client`; the standalone tunnel
  server lives next door for `drive.chan.app`.
* `chan serve --tunnel-token` / `CHAN_TUNNEL_TOKEN`:
  outbound tunnel mode per CLAUDE.md. Replaces the
  local loopback listener with a dial to
  `drive.chan.app/v1/tunnel`. Drive then published at
  `{user}.drive.chan.app/{drive}/*`.

So the **outbound** pattern is already plumbed at the
chan-server level — the redesign exposes it through the
chan-desktop launcher. The user-visible flow is "add a
remote-outbound drive" which under the hood spawns
`chan serve --tunnel-token <...>` locally and mounts a
webview against `{user}.drive.chan.app/<drive>` (or
similar), then proxies through.

The **inbound** pattern is the symmetric case: chan-
desktop runs `chan serve` locally with the standard
loopback listener (default `127.0.0.1:<port>`) but
configurable for `::1` + any host — and the drive entry
records the binding so the launcher can boot / shut
down the listener on demand. This is "remote inbound"
from the perspective of OTHER chan instances connecting
in via a tunnel server. Local user's webview connects
to the same loopback like a normal local drive.

(Naming check needed at task-cut: "remote inbound" vs
"shared local" vs "exposed drive." @@Alex's framing
maps to network-direction; the implementer may pick
clearer copy.)

Configuration details to nail down at task-cut:

* **Retry loop + backoff** for outbound: live-reconnect
  shape (jittered exponential? max-retries cap? user-
  surfaced state when reconnect fails?). The
  `chan-tunnel-client` likely has primitive support;
  audit + extend if needed.
* **Default port** for inbound: match whatever
  `chan serve --port` defaults to today (audit
  `crates/chan/src/main.rs::cmd_serve`). Surface in the
  config dialog as the default + let the user override.
* **Host binding** for inbound: support both `127.0.0.1`
  and `::1` (IPv4 + IPv6 loopback) explicitly per
  @@Alex's note. Any-interface (`0.0.0.0` / `::`)
  binding is a separate question — probably gated
  behind a "this exposes the drive to your local
  network" warning, not the default.
* **Tunnel credentials** for outbound: how does the
  remote URL get an auth token? Per CLAUDE.md the
  per-launch bearer token is the local-loopback gate;
  for remote outbound the trust boundary is the
  gateway in front of `drive.chan.app`. Either the
  tunnel-token flow handles this (chan-tunnel-client
  carries the credential) or the drive config stores
  it; audit at task-cut.

## Drives-list UI shape

```
+-------------------------+----------+--------+---------+-----------------+--------+
| Drive                   | Type     | State  | Windows | Address         | Action |
+-------------------------+----------+--------+---------+-----------------+--------+
| chan                    | local    | booted | 2       | ~/dev/.../chan  | Boot ⚙ |
| my-team's-drive         | outbound | off    | 0       | host.example:7  | Boot ⚙ |
| my-laptop-shared        | inbound  | booted | 1       | ::1:8787        | Boot ⚙ |
+-------------------------+----------+--------+---------+-----------------+--------+
```

Rough sketch; final visual decisions for the implementer.
Key columns: drive name, type, on/off state, window
count, address (path-or-host:port), action cluster
(Boot button + gear icon for config + forget option in
overflow / right-click menu).

## Forget-drive semantics

* Local drives: `chan remove <path>` unregisters from
  the registry. Drive content on disk is untouched.
  Confirmation modal copy: "This removes chan's
  bookkeeping for the drive at `<path>`. The drive's
  files stay on disk; you can re-add it later with
  the same path."
* Outbound drives: removes the drive entry + cached
  credentials. No effect on the remote chan instance.
  Copy: "This removes chan's record of the remote
  drive at `<host:port>`. The remote chan instance
  continues running on the host side. To stop the
  remote, ask the host to stop `chan serve` there."
* Inbound drives: removes the drive entry + the
  configured listener binding. Local content (the
  drive's actual file tree) is untouched. Copy:
  "This removes chan's bookkeeping for the inbound
  drive at `<host:port>`. The drive's files stay on
  disk; you can re-expose them later with the same
  path."

## Coupling with existing + planned work

* **`fullstack-b-1`** (window-config LRU): the new
  drives-list windows-open column reads the same
  bookkeeping that LRU restore uses. Reuse.
* **`fullstack-b-14`** (drive-path window title): the
  title for outbound drives becomes `host:port`; for
  inbound drives the listener binding. Generalize the
  `drive_title(key)` helper to handle the three drive
  types or split into per-type helpers.
* **Round-2 item 2 (pre-flight + BOOT)**: the per-
  drive pre-flight + BOOT process from
  `next-phase-backlog.md` item 2 + the
  `round-2-plan.md` Pre-flight feature toggles
  composition. This redesign is the LAYER ABOVE
  pre-flight — when the user clicks Boot, the
  pre-flight runs. The remediation card from item 2
  + the pre-flight feature toggles surface in the
  same window.
* **Round-3 Track 5** (per-agent submit-chord
  encoding map): unrelated but composes — remote
  drives may host agents the user runs against;
  agent-detection happens per-terminal regardless
  of drive type.

## Sequencing recommendation

This redesign is medium-large. Decomposes naturally:

| Step | Scope                                                                       | Owner(s) |
|------|-----------------------------------------------------------------------------|----------|
| 1    | Local-drive flow surface (replace today's "Open drive" + "Attach" with `[new] → [local]` branch + create-new affordance) | @@FullStackB |
| 2    | Drives-list table redesign: windows-open column + Boot rename + gear icon + forget action | @@FullStackB |
| 3    | Forget-drive UX + per-type confirmation copy                                 | @@FullStackB |
| 4    | Remote outbound flow: `[new] → [remote] → [outbound]` + retry/backoff config + drives-list integration | @@FullStackB + @@Systacean (tunnel-client wiring) |
| 5    | Remote inbound flow: `[new] → [remote] → [inbound]` + host+port config + drives-list integration | @@FullStackB + @@Systacean (chan-serve listener config) |
| 6    | Config dialog (same shape as creation, per-type parameterized) — refactor so step 1's create-dialog is reusable for step 4 + 5 | @@FullStackB |
| 7    | Quit-confirm gate when drives are booted (intercept OS quit, confirm + shutdown all booted drives + exit). See "Quit-confirm gate" section below. | @@FullStackB |

Steps 1-3 land before 4-6 since they don't depend on
the tunnel-client + listener config work. 4 + 5 land
in either order (independent paths). 6 is a refactor
that consolidates the dialog used in 1 / 4 / 5.

## Quit-confirm gate when drives are booted (added 2026-05-20)

@@Alex 2026-05-20: "if we click to close chan desktop when
there's at least 1 drive booted, we should ask the user to
close confirm (we close all windows and turn off the drive)
so we shutdown".

### Behaviour

* Trigger: user hits the OS quit affordance (Cmd+Q on
  macOS, the X on the launcher window, application-menu
  Quit) while at least one drive is in the "booted"
  state (per the drives-list column from the main
  redesign).
* Block the quit, show a confirmation dialog:
  > "Quit chan? One booted drive will be shut down."
  > (or "N booted drives will be shut down" if > 1)
  > [Cancel] [Quit + shutdown]
* If 0 drives are booted, quit immediately (no
  confirmation needed — nothing to shut down).
* On confirm:
  * Close every drive window in turn.
  * Stop each booted drive's underlying mechanism:
    * Local drive → kill the `chan serve` child process.
    * Remote outbound → close the `chan-tunnel-client`
      connection.
    * Remote inbound → stop the local listener.
  * Once all booted drives are off, the chan-desktop
    process itself exits cleanly.

### Why this matters

Today (and on the post-redesign launcher) closing
chan-desktop while drives are booted is a silent
operation — the underlying `chan serve` processes may
stay alive as background daemons, the user has no
signal that their drives are still running, and the
windows-open counter from the redesign would lie
across a quit-relaunch cycle. The confirmation +
explicit shutdown contract eliminates the
"chan-desktop closed but `chan serve` still running"
zombie state.

### Coupling with the redesign steps

Lands as **step 7** in the decomposition table (post
forget-drive UX, after the drives-list table redesign
+ Boot rename). Builds on steps 2 (windows-open
column tracking) + 4 / 5 (boot/shutdown semantics for
each drive type).

### Open questions

* **Quit-without-shutdown escape hatch?** Some users
  may want to leave drives running (e.g. expose an
  inbound drive overnight while the launcher is closed).
  Options: never offer (always shut down on quit), add
  a third button "Quit, leave drives running" (with a
  warning), or a Settings toggle "drives stay running
  when chan-desktop quits" (default off). Implementer
  picks at task-cut; the conservative default is
  "always shut down on quit."
* **Crash / hard-kill case**: if chan-desktop dies
  without going through the quit-confirm gate, the
  child `chan serve` processes are orphaned. Probably
  a Round-3 hardening item — the launcher could
  inherit-and-reattach orphaned `chan serve` processes
  on next launch by matching against the drives-list
  config. Out of scope for this redesign.

## Open questions for @@Alex when this revives

1. **Naming**: "Remote outbound" / "Remote inbound" vs
   alternatives ("Connect to remote drive" / "Share
   local drive"). The network-direction framing is
   accurate but jargon-heavy.
2. **Drive-name display for remote drives**: where does
   the human-readable name come from? User-typed at
   create time? Derived from `host:port`? Pulled from
   the remote chan's drive metadata?
3. **Any-interface (`0.0.0.0` / `::`) inbound binding**:
   gated behind a warning or excluded entirely?
4. **Outbound auth credentials storage**: in the
   chan-desktop config sidecar (encrypted? plaintext?)
   or in the OS keychain (Tauri secure-storage plugin)?
5. **"Boot" button copy for inbound drives that have
   no current viewer**: "Boot" still reads naturally
   because it starts the listener; just confirm.

## Status

* Not dispatched. No task files cut.
* @@Alex 2026-05-20: "it's fine, i can spin up separate
  team to work on the desktop part." This redesign is
  **separate-team work** — not folded into the current
  six-agent roster's queue. A future spawn of a
  chan-desktop-focused team picks up the
  decomposition above. The current @@FullStackB lane
  continues with the in-tree chan-desktop touches
  (window-config LRU, opener IPC, title format,
  Source Code Pro bundle, etc.) but does NOT carry
  this redesign.
* @@Alex 2026-05-20 (post-v0.11.1): "yes I will spin
  up a separate architect under the ./desktop dir and
  work from there without conflicting with the rest,
  will work on the details in the next session."
  **Separate architect role + working directory
  (`./desktop`)** confirmed. The chan-desktop team
  operates from `./desktop` with its own architect
  (distinct identity from `@@Architect`); coordination
  with the main six-agent roster is via the canonical
  phase journal + cross-references in this artifact,
  not via shared event channels (to avoid the
  multi-agent-tree commit-discipline overhead). Details
  for the separate-team bootstrap + scope boundary
  land in the next session.
* Cross-reference added to
  [`round-3-plan.md`](round-3-plan.md) so any future
  Round-3 fan-out surfaces it for the separate
  desktop team's bootstrap.
