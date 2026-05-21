# webtest-b-3 — `-b-22` orphan-sidecar reap + drive-lock-takeover UX walkthrough

Owner: @@WebtestB
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Walk @@FullStackB's `fullstack-b-22` work on the live
binary: process-group sidecar reap (prevention half) +
drive-lock-takeover UX (recovery half). Confirm both pieces
hold under the exact failure mode that surfaced the bug:
chan-desktop killed ungracefully → orphan `chan serve`
children → next chan-desktop launch can't bind the same
drive.

## Background

`-b-22` shipped 2026-05-21 at HEAD `3987e73`
("chan-desktop: process-group sidecar reap + drive-lock-
takeover UX (fullstack-b-22)"). Closes the orphan-sidecar
bug filed
[`../phase-8-bugs.md`](../phase-8-bugs.md) §"chan-desktop
leaves bundled `chan serve` sidecars orphaned after parent
dies".

Two implementation pieces per the bug spec:

1. **Prevention** — chan-desktop spawns `chan serve`
   children in a separate process group (`setpgid`-shape)
   AND keeps a `Vec<Child>` walked in a Drop /
   `on_window_event(CloseRequested)` handler. Defense in
   depth: SIGTERM-the-group on graceful exit; Drop catches
   the case where chan-desktop itself crashes ungracefully.
2. **Recovery** — on bind failure, chan-desktop identifies
   the holder (via known sidecar metadata file under TMPDIR
   with PID, OR `lsof -i :<port>` shape), confirms it's
   itself an orphan `chan serve` for the SAME drive path
   (not an unrelated process), auto-kills it (SIGTERM with
   deadline → SIGKILL), proceeds with the bind, surfaces a
   transient toast ("Reclaimed drive from orphaned process
   from previous session").

Read the @@FullStackB task tail for the canonical
implementation note + verification matrix:
[`../fullstack-b/fullstack-b-22.md`](../fullstack-b/fullstack-b-22.md).

## Coverage slice (lane B)

This is a chan-desktop runtime walkthrough — entirely
inside the **standing chan-desktop runtime permission**
that survives recycle per
[`../../../agents/bootstrap.md`](../../../agents/bootstrap.md)
§"Standing permissions". Throwaway-drive shape only.

**Do NOT fire the canonical fresh-Mac Gatekeeper perm ask
for this task** — per @@Alex 2026-05-21 "ahhh hold on, i
will only test the chan.app at the very very end", the
fresh-Mac walk is deferred indefinitely. See
[`../alex/event-webtest-b-alex.md`](../alex/event-webtest-b-alex.md)
"DEFERRED" entry. This task is throwaway-drive shape; no
fresh-Mac axis.

### Tightened-scope rules (still apply)

1. **Never touch `/Applications/Chan.app`.** Use a built
   chan-desktop binary or `npm run tauri dev` against a
   throwaway drive only.
2. **PID capture, not triage.** Capture chan-desktop's
   spawn PID; SIGTERM that PID and its sidecar children.
   No `pkill -f chan-desktop` shape.
3. **No `xattr -w com.apple.quarantine` on system paths.**

## Acceptance criteria

### Empirical repro of the original bug (against pre-`-b-22` shape)

Optional — only do this if you want a "before" anchor for
the verdict. The `-b-22` shape is already in HEAD; you can
skip the regression baseline and walk straight to the
"after" verification.

If you DO want the before-anchor: check out `3987e73~1`
(parent of `-b-22`), build chan-desktop, kill it
ungracefully, confirm orphan sidecars survive, confirm the
next launch fails to bind. Then return to HEAD.

### Prevention half — graceful exit reaps sidecars

1. Build chan-desktop at HEAD (`3987e73` or later);
   `make build` or `npm run tauri dev` per your standing
   perm.
2. Launch chan-desktop pointed at a throwaway drive
   (e.g. `/tmp/chan-test-phase8-wb-b22/`). Register via
   chan-desktop's launcher; open the drive.
3. Capture the chan-desktop PID + the spawned `chan serve`
   PID(s). `ps aux | grep -E '(chan-desktop|chan serve)'`.
4. **Graceful exit path**: send SIGTERM to chan-desktop's
   PID (NOT a window-close — actually
   `kill <chan-desktop-pid>`), OR close the window via the
   Cmd+Q / window-close affordance.
5. After ~1 second, `ps aux | grep 'chan serve'` should
   show zero rows for the throwaway drive's port. The
   sidecar got reaped.
6. Relaunch chan-desktop; click the throwaway drive; it
   binds cleanly with no takeover dialog (because no
   orphan exists).

### Prevention half — ungraceful exit reaps sidecars (defense-in-depth)

1. Repeat the prevention-half setup steps 1-3.
2. **Ungraceful exit path**: `kill -9 <chan-desktop-pid>`
   (skips Drop semantics in the parent). The Vec<Child>
   Drop handler won't fire; only the process-group
   semantic catches this.
3. After ~1 second: depending on the implementation
   shape, the sidecars may or may not be reaped. If they
   survive `kill -9`, the recovery half (takeover dialog)
   catches them on the next launch.

Document which outcome you see. Both are valid per the
defense-in-depth framing; the recovery half is the
load-bearing backstop.

### Recovery half — lock-takeover dialog

1. **Force the orphan-survival path**:
   * `kill -9` chan-desktop and confirm the sidecar
     survives (`ps aux | grep 'chan serve'` shows it),
     OR
   * Manually `chan serve /tmp/chan-test-phase8-wb-b22/`
     in a separate terminal to simulate the "stranded
     sidecar" condition.
2. Relaunch chan-desktop; click the drive.
3. **Expected**: takeover dialog surfaces, identifying
   the holder PID + drive path. User confirms; the orphan
   sidecar is SIGTERM'd (with deadline → SIGKILL fallback);
   chan-desktop proceeds with the bind; toast appears
   ("Reclaimed drive from orphaned process from previous
   session" or similar).
4. Capture screenshots: dialog state, toast state,
   post-takeover drive open.

### Negative case — non-chan PID

1. Bind a random non-chan process to the port chan would
   use for the throwaway drive (e.g. `python3 -m
   http.server <port>` against the configured port; or
   `nc -l <port>`).
2. Launch chan-desktop; click the drive.
3. **Expected**: chan-desktop refuses the takeover (does
   NOT kill arbitrary PIDs). Surfaces an error with the
   offending PID + advice. Should NOT auto-SIGTERM
   anything that isn't a confirmed-orphan `chan serve`
   for the same drive path.
4. Tear down the placeholder process.

This is the safety check from the bug spec: "If the
holder turns out to be NOT a chan sidecar, refuse +
surface the error with the offending PID + advice."

### Walkthrough audit trail

Append a fresh dated heading to
[`webtest-b-1.md`](webtest-b-1.md):
`## 2026-05-21 — fullstack-b-22 walkthrough (orphan
sidecar reap + lock takeover)`. Capture:

* Each of the four acceptance subsections (prevention
  graceful, prevention ungraceful, recovery dialog,
  negative case) with HOLD / FAIL / PARTIAL verdict.
* Screenshots / log snippets at each step.
* Any side observations for the bug list (e.g. dialog
  copy that could be clearer, edge cases the
  implementation doesn't catch).
* Tear-down evidence (chan-desktop killed, throwaway
  drive `rm -rf`'d, `chan remove <path>` registry
  cleanup).

## How to start

1. `git status` to confirm clean tree; `git log
   --oneline -10` to verify `3987e73` is in HEAD.
2. Build chan-desktop: `cd desktop && make build` (or
   `npm run tauri dev` if you prefer hot-reload during
   the walk).
3. Set up the throwaway drive
   `/tmp/chan-test-phase8-wb-b22/` per the standard
   test-server-workflow shape. Seed with `mkdir -p
   /tmp/chan-test-phase8-wb-b22/{subdir,notes}` + a
   few sample markdown files OR copy a known fixture.
   Register via `./target/debug/chan add` OR via
   chan-desktop's launcher; your call.
4. Walk the four acceptance subsections in order.
5. Append the verdict to `webtest-b-1.md`; fire a poke
   to @@Architect via
   `event-webtest-b-architect.md` when done.
6. Tear down per the standing test-server-workflow rule
   (kill chan-desktop, `rm -rf` throwaway drive, `chan
   remove <path>`).

## Coordination

* @@WebtestB lane (reactive).
* Standing chan-desktop runtime perm covers everything
  in this task (throwaway-drive shape, no
  `/Applications/Chan.app` touch).
* Tightened-scope rules from 2026-05-21 still apply
  (PID capture; no system-path xattr).
* If you find a regression-class issue: file a bug-list
  entry + flag for @@Architect routing (likely a
  fullstack-b-N follow-up).

## Numbering

Highest committed `webtest-b-N` is `-2` (the v0.11.2
walkthrough verdict, committed in `3262e61` /
adjacent pre-recycle commits); this is `-3`.

## Out of scope

* Canonical fresh-Mac Gatekeeper walk — deferred per
  @@Alex's "i will only test the chan.app at the very
  very end". Don't fire the perm ask.
* `chan-v0.11.2` DMG verification — that was `-b-2`
  (closed). This is the lane-B walk for HEAD-only
  work that hasn't shipped yet on a tag.
* @@FullStackB's `-b-23` web-marketing static site —
  routed to @@WebtestA via `webtest-a-3`.
* `-a-43` Hybrid back-side refactor walk — routed to
  @@WebtestA via `webtest-a-3`.
