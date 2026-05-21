# fullstack-a-37: "File moved or deleted" false-positive (CRITICAL)

Owner: @@FullStackA
Date: 2026-05-21

## Goal

Fix the editor falsely flipping to a "File moved or deleted"
panel while the file is STILL on disk + at the recorded path.
Multi-occurrence interruption during active writing.

## Background

Bug entry:
[`../phase-8-bugs.md`](../phase-8-bugs.md) — "**CRITICAL UX**:
Editor falsely flips to 'File moved or deleted' while file
is still on disk (repeated; interrupts writing)" (filed
2026-05-20).

Severity: **CRITICAL**. @@Alex's framing: "this is a serious
bug which has been interrupting my writing and breaking
concentration; we dont want users to have this kind of
experience." Third+ occurrence per @@Alex's dogfooding.

Root cause hypothesis space (narrow during repro):

* (a) chan-drive atomic-write race (temp + rename window
  caught by watcher).
* (b) `self_writes.rs` suppression miss (path canonicalisation
  / pathbuf vs string mismatch).
* (c) Sibling-write directory-scope leak (other agents'
  same-dir writes confuse the editor's "is my file there?"
  check).
* (d) FB watcher scope leak (per `-b-6` infrastructure
  shared with editor existence check).
* (e) Mtime / stat cache going stale (clock skew, smaller
  file confusing the check).

## Authorization

**Authorization: yes**, covers SPA editor + file-tab + the
"moved or deleted" panel UI. If root cause lands in
chan-server `self_writes.rs` or chan-drive's atomic-write
boundary, coordinate with @@FullStackB / @@Systacean for the
backend fix; the panel-side polish (broken Re-open button +
Find-suggest-inline UX) lives entirely in this task.

## Acceptance criteria

Three pieces:

1. **Stop the false detection**: under normal writing
   conditions (file on disk, atomic writes by chan-drive /
   external editor, sibling-file activity in the same dir),
   the panel does NOT surface. Add a recovery check: when
   the panel is about to fire, `stat` the recorded path with
   a 100-200 ms debounce; if the file is back, dismiss
   without UI flash.
2. **Fix the broken Re-open button**: currently the Re-open
   button routes to FB with nothing selected. Should restore
   the SAME file in place — re-read content from chan-drive,
   reset cursor / scroll state. This must work both when the
   panel surfaced falsely AND when the file genuinely moved.
3. **Find-suggest-reopen inline UX** (per @@Alex's suggestion):
   when the panel surfaces, run a backend search by basename
   (and optionally content-fingerprint of the cached file
   contents) across the drive. If a unique match is found at
   a different path, present inline: "File seems to have
   moved to `path/elsewhere/file.md` — Reopen there?" with
   a one-click reopen. Eliminates the "click Find → search →
   manually open" round trip.

* Pre-push gate: clean.
* Test pins: unit tests for the debounced stat-recheck +
  the Find-suggest-match logic (mock the search API).

## How to start

1. Wait for `-a-36` + `-b-17` to land so chan-desktop
   DevTools is unblocked. Then repro the bug with DevTools
   open + filesystem watcher logs (server-side) running.
2. Grep the SPA for the "File moved or deleted" string +
   the panel component. Trace the trigger path: what event
   causes the panel to surface? Likely a watcher event
   handler that interprets some FS event as a
   move-or-delete.
3. Reproduce: open `docs/journals/phase-8/alex/hybrid-revisited.md`
   (the file @@Alex hit it on) + write into it repeatedly,
   maybe with other agents writing sibling files. Observe
   when the panel surfaces + which event triggered it.
4. Narrow root cause to one of the (a)-(e) hypotheses. Use
   chan-server logs + DevTools network panel to see the
   watcher event sequence.
5. Implement the three-piece fix.
6. Test against the repro scenario.
7. Append commit-readiness + a brief root-cause writeup so
   the audit trail has the actual cause documented.

## Coordination

* **Depends on `-a-36` + `-b-17`** for the DevTools
  unblock that enables effective investigation. Could
  start scaffolding before they land (the Re-open button
  fix + Find-suggest UX are SPA-independent of the
  watcher root cause).
* **May touch chan-server / chan-drive** if root cause
  lands there. Coordinate via permission event to the
  appropriate lane (@@FullStackB for chan-server
  self_writes; @@Systacean for chan-drive watcher).
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Priority 2 in the wave's critical path (CRITICAL UX
  after the DEV META-BLOCKER unlocks).

## Open questions

(populated as you investigate)
