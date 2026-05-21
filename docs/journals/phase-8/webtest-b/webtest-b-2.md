# webtest-b-2 — v0.11.2 cut walkthrough lane B

Owner: @@WebtestB
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Walk the v0.11.2 cut binary on lane B. Confirm the
Round-1 + v0.11.1 + v0.11.2 mini-wave fixes hold on the
shipped binary; surface any regressions for v0.11.3 /
Round-2 wave-2.

## Background

`chan-v0.11.2` tag shipped 2026-05-21:

* Version-bump commit: `60901c1`.
* `release-desktop.yml` run 26221281508 completed green
  in 19m45s.
* Signed + notarized DMG live at
  https://github.com/fiorix/chan/releases/tag/chan-v0.11.2
  as `Chan_0.11.2_x64.dmg` (16.4 MB).

## Coverage slice (lane B)

Per the lane-A/B split established in `-1`:

* Native window-config persistence (chan-desktop side).
* Terminal cluster (scrollback, default TERM, Cmd+T).
* Watcher dialog cluster + bubble overlay.
* Indexing-chart pan/zoom.
* CLI scriptability (`chan list --json`, `chan index
  status`, etc.).
* New: chan-desktop signed bundle + first-launch.

## Acceptance criteria

* Walk each lane-B surface; confirm fixes hold.
* Surface regressions; file as Round-2 wave-2 or v0.11.3
  candidates per regression severity.
* Append per-surface verdict + screenshots to
  [`webtest-b-1.md`](webtest-b-1.md) tail with a fresh
  dated heading `## 2026-05-21 — v0.11.2 cut walkthrough
  lane B`.

### Canonical fresh-Mac Gatekeeper walk

The chan-v0.11.2 DMG is the first signed+notarized
chan-desktop bundle to ship publicly. A canonical
fresh-Mac walk on a Mac that has never seen the signing
identity is high-value verification.

**Fire a permission event FIRST** per the tightened
scope rules in
[`../alex/event-architect-webtest-b.md`](../alex/event-architect-webtest-b.md)
"Scope clarification..." + the pause-and-warn rule. Body
shape:

> Gatekeeper-clean walkthrough for chan-v0.11.2 DMG
> requires either (a) pausing the current chan-desktop
> session + closing Chan.app + resuming via iTerm with
> the tightened scope rules, OR (b) running on @@Alex's
> secondary Mac. Which?

WAIT for @@Alex's call before proceeding. The (a) path
requires @@Alex to consciously close their working
Chan.app — that's a destructive action the agent CANNOT
make unilaterally.

If @@Alex defers / declines: walk the binary in lane-B
throwaway-drive shape only, capture the keychain-
independent signals (`spctl`, `stapler`, `codesign`,
`syspolicyd` log), document the partial.

### Boundaries that always apply (from the tightening)

1. **Never touch `/Applications/Chan.app`.** Custom
   install destinations only (`/tmp/chan-ci10-verify/...`
   or @@Alex's secondary Mac).
2. **Process ownership by capture, not triage.** Capture
   the launched PID at spawn; only SIGTERM that PID.
   No `pkill -f chan-desktop`. No "high elapsed time so
   it must not be mine" inference.
3. **No `xattr -w com.apple.quarantine` on system
   paths.** Real fresh-Mac verification can't be
   simulated locally; honest options are secondary Mac,
   fresh VM, or documented partial.

## How to start

1. Standing perm covers throwaway-drive runtime; the
   tightened scope applies to DMG/Gatekeeper
   verification only.
2. If proceeding with the fresh-Mac walk: fire the
   permission ask first.
3. Spin a lane-B test server against any throwaway drive
   (e.g. `/tmp/chan-test-phase8-wb-r2/`).
4. Walk + capture.

## Coordination

* @@WebtestB lane. Standing perm covers
  throwaway-drive shape.
* Tightened DMG/Gatekeeper scope applies.
* On bootstrap, ack the tightened scope by appending a
  one-line confirmation to
  [`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md)
  before starting any DMG-verification work.

## Numbering

Highest committed `webtest-b-N` is `-1` (lane-omnibus).
This is `-2`.
