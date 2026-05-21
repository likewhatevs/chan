# fullstack-a-38: Notification surface polish

Owner: @@FullStackA
Date: 2026-05-21

## Goal

Two related notification-surface bugs combined into one task
since both touch BubbleOverlay / status-bar UI + the
transient-vs-persistent notification taxonomy:

1. Pre-flight bubble shows a spinner glyph + `0:00` label
   that never ticks.
2. "Copied path" status-bar notification persists too long
   (doesn't auto-dismiss).

## Background

Bug entries:

* [`../phase-8-bugs.md`](../phase-8-bugs.md) — "Pre-flight
  bubble spinner stuck at `0:00`" (filed 2026-05-20).
* [`../phase-8-bugs.md`](../phase-8-bugs.md) — "'Copied path'
  status-bar notification persists too long (doesn't
  auto-dismiss)" (filed 2026-05-21).

Both touch the SPA's notification rendering — different
surfaces but the same UX-taxonomy concern: notifications
that have no timing concept shouldn't pretend to (spinner)
and transient-action notifications should auto-dismiss
(Copied path).

## Authorization

**Authorization: yes**, covers SPA notification surfaces
(BubbleOverlay.svelte + status-bar notification component
+ any state-mod required for the taxonomy split).

## Acceptance criteria

Two pieces:

### A — Pre-flight bubble spinner gating

* Pre-flight events whose JSON payload carries NO timing
  data (no `eta` / `started_at` / equivalent) render
  WITHOUT the spinner + label entirely. Bubble shows
  topic + note + standing options chrome; no timer.
* Pre-flight events WITH timing data render the spinner +
  the timer tick correctly. (Optional verification — chan
  doesn't have a current emitter that sets timing data;
  the gate just stops the false-positive display.)
* Implementer picks: option 1 = suppress spinner when no
  timing (recommended); option 2 = show elapsed-since-emit
  using the watcher's read of the event file's mtime
  (deferred to follow-up if it earns itself).

### B — Status-bar transient notification auto-dismiss

* "Copied path" notification (and any sibling transient
  actions: "Saved", "Build complete", etc.) auto-dismiss
  after ~3 s (4-5 s acceptable).
* Audit the status-bar notification taxonomy: TRANSIENT
  (auto-dismiss after timeout) vs PERSISTENT (stay until
  user dismisses, e.g. watcher-event counts, error
  notifications). The split should be EXPLICIT in the data
  model + render path, not implicit in timeout-or-not
  behavior. Either a `kind: "transient" | "persistent"`
  discriminator or two separate stores.
* Pre-push gate: clean.

## How to start

1. Grep for the "Copied path" string in the SPA source.
   Find the emission call site + the status-bar notification
   datastructure.
2. Grep for the pre-flight spinner / `0:00` label rendering
   in `BubbleOverlay.svelte` or adjacent. Identify the
   timing-data source.
3. Decide the taxonomy shape: discriminator on the
   notification type OR separate stores. Implement.
4. Apply the auto-dismiss timer for transient
   notifications. ~3 s default; constant should be a named
   value in a single place for easy adjustment.
5. Suppress the spinner when no timing data is present.
6. Test: trigger Copy Path action → notification appears →
   auto-dismisses ~3 s later. Drop a pre-flight event into
   a watcher dir without timing data → bubble renders
   without spinner. Drop a pre-flight event WITH timing
   data → spinner + label appear (if any pre-flight emitter
   in chan-server populates timing; otherwise this branch
   is dead code, fine).
7. Append commit-readiness.

## Coordination

* Independent of other v0.11.2 tasks (no shared file with
  the critical bugs).
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Parallelisable with `-a-39 / -a-40 / -a-41 / -b-18 /
  -b-19`.

## Open questions

(populated as you investigate)
