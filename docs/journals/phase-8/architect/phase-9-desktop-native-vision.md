# Phase 9 vision — desktop-native cross-platform architect

Author: @@Architect (current phase-8 incarnation), captured
from @@Alex's 2026-05-21 framing.
Status: **forward-look**. Phase 8 is still closing
(v0.11.2 cut in flight; Round-2 wave-2 + Round-3
pending). Phase 9 opens after Round 3 lands. This doc
captures the directional intent so the eventual phase-9
@@Architect inherits the vision intact.

## Headline

Phase 9 bootstraps **@@Desktect** (Desktop + Architect
portmanteau; handle locked by @@Alex 2026-05-21), a new
architect specialized on desktop-native cross-platform —
macOS, Linux, Windows, each with native keybindings +
native integration with the chan binary. This is parallel
to (not replacing) the current @@Architect; phase 9 may
have BOTH architects active depending on how scope
partitions.

## Architectural boundary

The seam between desktop-native and chan is at the
**drive level, at the network layer.** Not at the
filesystem layer (chan-drive owns that), not at the
process layer (today's fork-chan-serve model is one
implementation, not the boundary), but at the
protocol that transports drive operations.

`chan-tunnel-proto` (h2/yamux frames over the network)
is the existing seam; it generalizes to three drive-
connection modes:

| Mode                       | Who runs what                                                                                                |
|----------------------------|--------------------------------------------------------------------------------------------------------------|
| **Local drive**            | Desktop-native forks `chan serve` (or, if bundled, runs it in-process); same machine, loopback transport.    |
| **Attached drive — outbound** | User provides a URL of a `chan serve` running somewhere; desktop-native is the CLIENT dialing it (today's tunnel-client side). |
| **Attached drive — inbound**  | Desktop-native LISTENS; remote `chan serve --tunnel-url <desktop-native-url>` connects back to it. Reverse-tunnel; desktop-native is the SERVER. |

The "drive at network layer" framing means desktop-native
NEVER needs to be the filesystem authority. The remote
chan-serve (or local-bundled chan-server) owns the drive;
desktop-native is always a network consumer with
identical UX regardless of where the drive engine lives.

### Why this boundary

Cross-version compatibility is the load-bearing benefit.
A user can run a newer chan in a remote environment and
attach it to an older desktop-native; or vice-versa. The
desktop-native shell IS the user's stable interface;
chan versions can roll independently behind it.

This is the same shape as LSP (editor stable; language
servers version independently) or `docker` CLI vs
daemon. Protocol stability becomes a first-class promise.

## Open question — ship the chan binary, or not

@@Alex is genuinely considering whether desktop-native
ships as a **single binary with the drive engine
embedded**, eliminating the separate `chan` binary
download for desktop users.

### The desktop-only user journey @@Alex described

```
User: never used chan; opens Mac terminal regularly only
      for git etc.
↓
Cmd+Space → "Chan" → first launch ever
↓
Desktop-native detects "no chan metadata anywhere"
(fresh-install state)
↓
Auto-creates default drive named "Chan" containing the
chan user manual as starter content
↓
Drive opens; user sees the manual; starts using it
↓
[Over time] User makes the Chan drive their second brain
— notes, ideas, daily writing. Also creates additional
drives for git repos (local or remote), each shaped to
their content.
↓
[If user ever deletes the default "Chan" drive] → ALL
chan metadata gets wiped (registry, indexes, sessions,
config, embedded model cache, everything)
↓
Next chan-desktop launch detects fresh-install state
again → recreates the default "Chan" drive with manual
```

This is "the app IS the experience" — Cmd+Space → Chan
→ immediately useful → grows with the user. Single
binary is the natural shape for this onboarding.

### Tradeoffs

| Axis                                       | Single binary (drive engine embedded)                                              | Separate chan binary (current model)                                                       |
|--------------------------------------------|------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------|
| **Install friction**                       | Lower — one download, full experience                                              | Higher — desktop + chan binary discovery (PATH-first + bundled fallback per `-b-15/-16`)   |
| **Binary size**                            | ~30-50 MB DMG (estimate; +drive engine + indexer)                                  | ~16 MB DMG today (chan binary bundled as sidecar; same size class)                         |
| **Orphan-sidecar bug surface**             | Gone — no separate process to orphan                                               | Present — see `phase-8-bugs.md` "chan-desktop leaves bundled chan serve sidecars orphaned" |
| **Architectural boundary**                 | Boundary moves from process to module; `chan-drive` library is consumed in-process | Process-level boundary at `chan serve` fork seam                                           |
| **Headless / CLI / CI users**              | Separate `chan` binary still ships for them                                        | Same binary serves both                                                                    |
| **Update cadence**                         | Desktop + chan bump together                                                       | Desktop + chan can roll independently                                                      |
| **Lock-step library risk**                 | Single source of truth (one `chan-drive` consumer in the bundled path)             | Two consumers; protocol version skew possible if not careful                               |

### Architect's preliminary read

The workspace already separates `chan-drive` as a
library; both `chan` binary and `chan-server` consume it.
Adding "chan-desktop embeds `chan-server`" is a SMALL
architectural shift — not a rewrite. The boundary
question is only "do we ship the separate `chan` binary
for desktop users by default?" not "does the architecture
support embedding?" — the architecture supports both.

Recommend phase 9 ships **embedded by default for
desktop-native** + keeps the separate `chan` binary
available for the curl-bash install path. Desktop users
get one-binary install; CLI / headless / server users
keep the option. Same `chan-drive` library across both
consumers, so no library skew.

The orphan-sidecar bug (filed today) is the canary —
shipping fork-style means owning a lifecycle problem
that embed-style doesn't have. That's a tail wind for
embed-by-default.

## Bidirectional discovery — chan binary prefers running desktop-native

@@Alex's invariant: if the user has both desktop-native
AND a separate `chan` binary installed, and runs `chan
serve <path>` from the terminal:

```
chan serve <path>
↓
chan binary detects: is a desktop-native instance running?
↓
[YES] Hand the drive to desktop-native as an
      attached-inbound drive. User sees it appear in their
      open chan-desktop window. NO local browser launched.
↓
[NO]  Fall back to today's behavior: local browser open
      against loopback chan-server. (Or whatever the
      eventual no-desktop-native default becomes — could
      stay browser-launch or could be CLI-only TUI.)
```

This is the `code <file>` shape — VS Code's CLI prefers
attaching to a running window over spawning a new one.
Or `git daemon`'s "is there a daemon already?" check.

### What this needs

* **Discovery mechanism**: how does the `chan` binary
  know desktop-native is running? Candidates:
  1. Well-known Unix-domain socket under `$TMPDIR/chan-
     desktop.sock` (per-user); desktop-native creates on
     launch, removes on graceful exit.
  2. Well-known TCP port on loopback (race-y if multiple
     desktop-native instances).
  3. Bonjour / mDNS (overkill for same-machine; useful
     for cross-machine attached cases).
  4. Per-user config registry file under
     `~/.config/chan/desktop-native.pid` (PID file with
     IPC endpoint info).
  Recommend (1) — Unix-domain socket is the cleanest
  same-machine signal + matches the chan-server MCP
  socket pattern already in `mcp_bridge.rs`.
* **Handshake**: chan binary dials the socket, sends a
  "claim drive at <path>" frame, gets back an OK + the
  open-drive URL or a denied response. If denied (e.g.
  drive is incompatible), fall back to browser-launch.
* **Security**: same-machine + same-user only; the
  Unix-domain socket's filesystem permissions enforce
  this. No cross-user surprise drive grabs.
* **Graceful behavior on no-desktop-native**: socket
  missing → no probe wait beyond a timeout (~50ms); fall
  back cleanly.

## Default "Chan" drive lifecycle

@@Alex's intent: the desktop-native owns a default drive
named `Chan` (capital C) that's the "second brain" entry
point for non-power users.

### State machine

```
[ALWAYS] On desktop-native launch:
  ↓
  Read chan metadata (registry + sessions + indexes + ...)
  ↓
  [FRESH STATE — no metadata]
    → create default drive "Chan" under canonical path
      (e.g. ~/Documents/Chan or ~/Chan; OS-platform
      decision; macOS = ~/Documents/Chan)
    → seed with chan manual content (the manual lands
      under backlog item 6 as docs/manual/; bundled
      version embedded in the binary)
    → open it automatically
  ↓
  [NORMAL STATE — registry has the Chan drive]
    → resume; open it automatically (UX detail: also
      open whatever drives were open when user last
      closed? Recall the chan-desktop session state)
  ↓
  [USER-DELETED-THE-CHAN-DRIVE STATE]
    → Wipe all chan metadata everywhere (registry,
      indexes, sessions, config, embedded model cache,
      everything)
    → On next launch → FRESH STATE path → recreate the
      Chan drive
    → This is the "factory reset" affordance, accessed
      by the user simply deleting their default drive
```

The "delete default drive → wipe everything → recreate"
loop is elegant — it makes the default drive's existence
LOAD-BEARING for chan's identity. Users have a single
clear "uninstall the app's memory" gesture: delete the
Chan drive. The desktop-native binary stays in
/Applications; everything else regenerates.

### Open design questions for phase 9

1. **Default drive location** — macOS `~/Documents/Chan`
   vs `~/Chan` vs `~/Library/Application
   Support/Chan/drive`. macOS convention favors
   Documents for user-facing content; Application
   Support for app metadata. The Chan drive is user-
   facing (their second brain), so Documents fits. Linux
   = `~/Documents/Chan` (XDG `$XDG_DOCUMENTS_DIR`);
   Windows = `Documents\Chan`.
2. **Manual seeding shape** — embed the full
   `docs/manual/` tree (from backlog item 6) at build
   time and write it into the new drive on creation? Or
   ship a smaller seed (welcome.md + one or two
   pointers)? Recommend full embed — the manual is the
   onboarding surface; partial seed degrades it.
3. **Wipe-on-delete trigger** — how do we detect "user
   deleted the Chan drive" reliably across platforms?
   Filesystem-watch on the parent dir? On-launch check
   that the registered drive path exists? Recommend the
   latter (simpler, no background watch); the check runs
   on every launch and is cheap.
4. **Migration from current model** — existing phase-8
   users have multiple drives, no special "Chan" drive
   designation. Migration path: on first phase-9
   desktop-native launch with existing chan metadata,
   detect the "no Chan drive registered" state and offer
   to designate an existing drive as the default OR
   create a fresh Chan drive alongside. Don't surprise-
   delete anything.

## Cross-platform native bindings + integration

The phase-9 architect owns parity across:

| Platform | Native bindings                                                              | OS integration                                              |
|----------|------------------------------------------------------------------------------|-------------------------------------------------------------|
| macOS    | Cmd-based chords (today's baseline); native menu bar; Cmd+, Cmd+W etc.       | Spotlight, Quick Look, Services, file-type associations, URL scheme `chan://` |
| Linux    | Ctrl-based chords; native menu via GTK/AppIndicator; xdg-open integration    | XDG MIME types, .desktop file, application menu, freedesktop conventions |
| Windows  | Ctrl-based chords; native ribbon / menu; file associations via registry      | Start menu, taskbar, MSI installer, URL scheme registration |

The keybinding remap (Cmd ↔ Ctrl ↔ platform-canonical)
needs to be **stable across chan versions** — same shell
talks to older/newer attached chan. This is the
load-bearing protocol-stability promise.

## Implications for phase-8 close

None — phase 8's in-flight work (v0.11.2 cut, Round-2
wave-2 fan-out, Round-3 public flip) doesn't conflict.
The current chan-desktop / chan-serve fork model
continues; it's the foundation phase 9 builds on.

Two carryovers that touch phase-9 surface:

* **`fullstack-b-15` / `-16`** (PATH-first probe +
  bundled fallback for `chan` binary): if phase 9 chooses
  embed-by-default, this code path simplifies — the
  embedded chan-server is the only resolver target. The
  PATH-discovery falls back to "external chan binary
  attached as inbound drive" via the bidirectional
  discovery mechanism. Not a rewrite; a re-purposing.
* **Orphan-sidecar bug** (filed today): phase 9 might
  obsolete this bug entirely by removing the fork.
  Don't sink heavy investigation budget into the
  takeover-UX piece if phase 9 lands within a quarter;
  build the minimum fix that ships v0.12.x cleanly. The
  takeover-UX work survives only for the
  separate-chan-binary use case.

## What @@Alex needs to decide before phase 9 opens

A small surface; all sit at the "lock the directional
choice" level, not implementation detail:

1. **Embed or separate** — single binary with drive
   engine vs. ship-separate-chan model. Recommend
   embedded by default + separate available for CLI
   users. Locking this drives everything else.
2. **Default drive location per platform** — proposed
   above (`~/Documents/Chan` on macOS / Linux; Windows
   equivalent). Quick confirm.
3. **Manual seeding shape** — full embed of
   `docs/manual/` vs. partial seed. Recommend full.
4. **Wipe-on-delete UX** — confirm the
   "delete-the-drive = factory reset" semantic. It's
   load-bearing for the entire UX; getting it wrong
   means users lose data unexpectedly.
5. **Phase-9 architect's name / handle** — **LOCKED
   2026-05-21**: `@@Desktect` (Desktop + Architect
   portmanteau). Separate identity from the current
   @@Architect; phase 9 runs both lanes if scope warrants.

These can survey at fan-out time, not now. Phase 8 is
still closing.

## Memory linkage

* [[project_dispatch_is_automation_blueprint]] — phase 9
  inherits the same dispatch shape; the new architect
  follows the same event-channel + task-file pattern.
* [[project_chan_code_variant]] — the chan-code spinoff
  idea is orthogonal to this; both can coexist
  (different verticals).
* [[project_media_browser]] — relevant to FB scope; the
  Hybrid FB back from Task F (Round-2 wave-2) is a phase-
  8 piece that desktop-native will inherit unchanged.

## Next steps

* Phase 8 closes per existing plan (v0.11.2 cut → Round-2
  wave-2 → Round-3 public flip → end of phase 8).
* Phase 9 opens with @@Alex's decisions on the 5
  questions above; the new desktop-native architect
  bootstraps from this doc.
* This doc lives under `phase-8/architect/` per
  convention (artifacts live in the phase where they
  were ARTICULATED, not where they execute). Phase 9's
  planning docs will reference it.
