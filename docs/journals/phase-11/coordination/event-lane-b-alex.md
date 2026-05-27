# Channel: @@LaneB -> @@Alex

Append-only. @@LaneB escalates human-decision blockers here; @@Alex
reads. Standing gate: the macOS CLI-to-desktop handoff design note goes
here and waits for @@Alex ratification before implementation. Never edit
prior entries.

(no entries yet)

## 2026-05-26 @@LaneB -> @@Alex: DESIGN NOTE -- macOS CLI-to-desktop handoff (GATE: needs ratification)

Scoped to macOS. Linux desktop launch is blocked behind the deferred
item; Windows deferred. Do NOT implement until you ratify the
recommendation below.

### Problem
Today `chan serve <path>` (cmd_serve) always owns its OWN loopback
server: it opens the drive under chan-drive's per-drive flock, binds
127.0.0.1, prints a one-shot bearer token, and opens the browser.
chan-desktop ALSO runs an embedded chan-server that mounts drives
in-process against the same `~/.chan` registry, behind its own
per-window webview. So when the desktop is already running and the user
types `chan serve ~/notes` in a terminal, two things contend for one
drive: the desktop holds (or will try to hold) the flock, and the CLI
hits `DriveLocked`/`DriveAlreadyOpen`. The desktop already surfaces this
verbatim ("This drive is open in another chan process. Quit it and try
again." -- embedded.rs map_open_error). That's correct but a dead-end
UX: the natural intent of `chan serve ~/notes` while the desktop is up
is "show me this drive in the app," not "fail."

### Three options

OPTION A -- attach to the running desktop-owned server.
The CLI discovers a running desktop, asks it for that drive's already
-mounted loopback URL + token, and just opens the browser (or prints the
URL) against the desktop's embedded server. No second drive open, no
second flock, no second listener.
+ Zero contention: one process owns the drive's writes (the single-user
  / single-machine invariant holds exactly).
+ Reuses the desktop's already-warm index/watch/server.
- The CLI's job becomes "find URL + token and open browser," which is a
  different shape from "I am the server." The token is the desktop
  embedded server's per-launch token; the CLI has to obtain it over the
  discovery channel (it must never land in argv / `ps`).
- If the user wanted a HEADLESS serve (scripting, a remote SSH session
  with no desktop display), attach is wrong.

OPTION B -- ask the desktop to OPEN the drive (spawn a window).
The CLI discovers a running desktop and sends it an "open this drive"
request; the desktop spawns/raises a drive WINDOW (its native UX) and
the CLI exits. No browser, no URL handed back.
+ Best native UX: the user gets the real desktop window, not a browser
  tab pointed at loopback.
+ Mirrors the existing `serve::start` window path; the desktop already
  knows how to mount + spawn a window for a registry drive.
- `chan serve` semantically becomes "tell the app to open this," which
  surprises anyone expecting a foreground server + printed URL. Needs a
  clearly-different exit behavior (CLI returns immediately).
- Still wrong for headless / browser-preferring users.

OPTION C -- keep owning its own server (status quo).
The CLI never talks to the desktop; on contention it prints the
quit-the-other-process message.
+ Simplest, no new IPC surface, no new attack surface.
+ Correct for headless/scripted/SSH and for "I explicitly want my own
  server."
- The desktop-running case stays a dead-end ("quit it and try again").

### Same-user discovery (the reuse pattern)
mcp_bridge.rs is the blueprint: it binds a per-pid Unix-domain socket at
`/tmp/chan-mcp-<pid>-<8hex>.sock`, unlinks stale sockets before bind, and
a `Drop` guard unlinks on teardown (so a `kill -9` leaves a stale file
that the next bind cleans up). For handoff, the desktop would publish a
WELL-KNOWN same-user UDS (not per-pid -- the CLI must FIND it without
knowing the pid), e.g. `$XDG_RUNTIME_DIR/chan-desktop.sock` or
`/tmp/chan-desktop-$UID.sock` (macOS sun_path is 104 bytes; keep it
short). Same-user is enforced by file ownership + 0600 perms + the
socket living in a per-user dir; cross-user attach is simply not
discoverable. The CLI:
1. tries to connect the well-known UDS;
2. if connect fails / refused / stale -> no desktop -> fall through.
The bearer token travels OVER this UDS (a trusted same-user channel),
never via argv/env, matching the local-first auth model.

### Mismatch / lifecycle representation
- OWNERSHIP: exactly one process owns a drive's writes. Attach/open
  hand the drive to the DESKTOP as owner; the CLI becomes a thin
  client/launcher. The flock stays the single authority -- if the
  desktop owns it, the CLI must NOT also open the drive.
- BEARER TOKEN: returned by the desktop over the UDS, appended to the
  URL the CLI opens (or used by the spawned window). Never logged,
  never in `ps`.
- LIFECYCLE: in attach/open, the CLI exits after handoff; the desktop's
  window/server lifecycle is unaffected by the CLI exiting. (Contrast
  with status quo where Ctrl-C on the CLI tears down the server.) This
  is a visible behavior change the note must call out to users.
- VERSION: the UDS handshake carries a protocol/semver field. On a
  desktop-vs-CLI version skew the CLI does NOT attach; it prints "desktop
  is version X, CLI is Y; cannot hand off" and falls back to standalone
  (or refuses, per the chosen policy). No silent cross-version IPC.
- CAPABILITY: the handshake advertises what the desktop can do (open a
  local drive? a tunneled one?). A request the desktop can't satisfy
  falls back to standalone rather than erroring.
- NO-DESKTOP FALLBACK: connect-refused / no socket / stale socket / bad
  handshake -> behave EXACTLY like today (own the server, print URL).
  This is the load-bearing default: `chan serve` must always work
  with the desktop absent.

### Standalone-forcing flags
- `--standalone` (or reuse the spirit of `--no-browser`): never attempt
  handoff; always own the server. Required for headless/SSH/scripted.
- Headless detection: if there's no GUI session / no display, skip
  handoff automatically (don't hand a drive to a desktop the user can't
  see). Treat `CHAN_NO_DESKTOP_HANDOFF=1` as an explicit opt-out for
  automation.
- `--tunnel-*` already forces a fundamentally different path (the CLI
  dials drive.chan.app); handoff is local-only and must be skipped when
  any tunnel flag is set.

### RECOMMENDATION
Adopt OPTION B (ask the desktop to OPEN a drive window) as the default
WHEN a same-user desktop is discovered AND we're in a GUI session AND no
standalone/tunnel flag is set; FALL BACK to OPTION C (own the server)
in every other case. Rationale: B gives the genuinely-native result a
desktop user expects from `chan serve ~/notes` (a real Chan window, not a
browser tab on loopback), reuses the desktop's existing
mount+spawn-window path, and keeps the single-owner invariant clean (the
desktop owns the drive; the CLI is a launcher that exits). The mandatory
C fallback preserves the headless / no-desktop / scripted contract
unchanged, so nothing regresses for non-GUI users. I'd skip A: handing
back a loopback URL + token to open in a browser is strictly worse UX
than B for the same discovery cost, and it muddies who-owns-the-token.

Phasing if you ratify B+C: ship the UDS discovery + handshake
(version/capability/ownership) and the "open window" request first; add
`--standalone` + headless auto-skip + the version-skew message in the
same slice (they're cheap and load-bearing for safety). Implementation
waits on your ratification.

QUESTION FOR YOU: ratify B-default-with-C-fallback? Or do you prefer C
forever (keep `chan serve` always-own-its-own-server, desktop-running is
a "quit it" message), or A (browser handoff)? One pick unblocks me.
