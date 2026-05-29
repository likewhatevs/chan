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

## 2026-05-21 — ready for review

### A — Pre-flight bubble spinner gating

Picked option 1 per the spec recommendation: suppress the
spinner + label entirely when no timing data is present.
Root-caused in `BubbleOverlay.svelte::elapsedLabel`: it
derives `startMs` from either `event.topic` (numeric string)
OR a 10+ digit timestamp inside `event.id`. When neither
yields a positive number the label falls through to `0:00`
and the existing `preFlightTimedOut` check returns false
(0 < 300 s), so the spinner branch fires forever. The
architect-fired pre-flight events @@Alex saw
(`event-arch-preflight-2.md` etc.) carry only
`topic`/`note`/`from`/`to`/`id`/`type` — no timing data —
so they hit the bug verbatim.

Fix:

* New `preFlightStartMs(event): number | null` helper.
  Returns the resolved start time OR null when none was
  embedded. Centralises the topic/id derivation in one
  place.
* New `hasPreFlightTiming(event): boolean` predicate
  wrapping the above.
* `elapsedLabel` now reads through the helper (no
  behavioural change for events WITH timing).
* `preFlightTimedOut` short-circuits to false when no
  timing data is present (the "Spawn idle" branch can't
  fire without a clock to measure against).
* The `{#if event.type === "pre-flight"}` block in the
  bubble template now also checks `hasPreFlightTiming(event)`
  so the whole `.preflight-status` div (spinner + label OR
  Spawn-idle text) hides when there's no clock.

Events WITH timing data (any future emitter that packs
`started_at` epoch into `topic` or embeds a timestamp in
`id`) still get the spinner + tick chrome unchanged.

### B — Status-bar transient notification auto-dismiss

Audit verdict on the status taxonomy:

| Surface                              | Verdict     |
|--------------------------------------|-------------|
| `notify(msg)` (10+ call sites)       | TRANSIENT — short action feedback |
| "Copied path" / "copy failed" (FileTree) | TRANSIENT — action confirmation/error |
| `opened X` / `selected X` (window_command) | TRANSIENT — navigation confirmation |
| "Moving…" (in-flight rename)         | PERSISTENT — cleared on op end |
| `restore failed:` / `bootstrap failed:` | PERSISTENT — load-time errors |
| Per-action error tails (`rename failed:` etc.) | left as-is for follow-up; obvious TRANSIENT candidates but lower-priority migration |

Fix:

* Extended `ui` state with `statusKind: "transient" |
  "persistent" | null`. Explicit in the data model per
  the spec.
* New `setTransientStatus(msg, ms = 3000)` exported helper
  in `store.svelte.ts`. Writes both fields, schedules a
  self-cancelling timer; re-entry cancels the prior timer
  so the latest message wins. The timer's clear check is
  identity-guarded: `if (ui.status === msg && statusKind ===
  "transient")` — a persistent write that lands during
  the window is NOT clobbered by the late timer fire.
* `setNotifyHandler` rewired to route ALL `notify(msg)`
  calls through `setTransientStatus`. Every `notify()`
  caller gets auto-dismiss without per-callsite change.
* `FileTree.svelte::copyPath` migrated: `ui.status = "Copied
  path"` → `notify("Copied path")`. Pairs with the error
  branch (`notify("copy failed: ...")`).
* `store.svelte.ts::handleWindowCommand` migrated:
  `ui.status = "opened ..."` / `"selected ..."` →
  `setTransientStatus(...)`.

Direct `ui.status = ...` writes default to persistent
(`statusKind` stays whatever it was — typically null on
first write). This is the conservative path: existing 30+
direct writers keep their existing semantics, only the
opt-in callers flip to transient.

### Test pins

* **NEW `web/src/state/transientStatus.test.ts`** — 5 pins:
  default-3s clear, latest-wins on re-entry,
  persistent-mid-flight isn't clobbered, custom ms arg,
  `notify()` routes through transient.
* **BubbleOverlay.test.ts** — 2 new pins: pre-flight
  without timing renders no `.preflight-status` div + no
  `0:00` text; pre-flight with timing (topic = epoch ms)
  renders the `.preflight-status` div with a M:SS label.

### Files touched

| File                                              | Change                                                                  |
|---------------------------------------------------|-------------------------------------------------------------------------|
| `web/src/state/store.svelte.ts`                   | `ui.statusKind` field + `setTransientStatus` helper + notify handler rewired; window_command status writes migrated |
| `web/src/state/notify.svelte.ts`                  | (no change — handler signature unchanged; routing flipped in store)     |
| `web/src/components/BubbleOverlay.svelte`         | `preFlightStartMs` / `hasPreFlightTiming` helpers; spinner block gated  |
| `web/src/components/FileTree.svelte`              | Copy-path / copy-failed migrated to `notify()`; `ui` import dropped     |
| `web/src/state/transientStatus.test.ts`           | NEW — 5 pins on the auto-dismiss + notify routing                       |
| `web/src/components/BubbleOverlay.test.ts`        | 2 new pins on the spinner gating                                        |

### Suggested commit subject

```
Notification surface: pre-flight spinner gating + transient status auto-dismiss (fullstack-a-38)
```

Single commit. The two pieces share the
"transient-vs-persistent taxonomy" framing and would split
awkwardly: the spinner-gate piece references the same idea
(notifications without timing data shouldn't pretend to
have it; transient notifications shouldn't pretend to be
persistent).

### Gate

* vitest **575 / 575** (+7 net: 5 in
  `transientStatus.test.ts`, 2 in `BubbleOverlay.test.ts`).
* svelte-check 0 errors / 0 warnings across 3982 files.
* npm build clean.

### Composition

* `-a-37` already in the working tree — its `FileMissingState`
  change is in `tabs.svelte.ts`; unrelated to this task.
* `-a-36` ditto, in `api/desktop.ts` + `Pane.svelte`.
* Round-2 follow-up candidate: migrate the remaining
  one-shot action error writes (rename / create / delete /
  duplicate failed) to `notify()`. Out of scope for v0.11.2
  per spec discipline; flagging here for any future polish
  pass.

Picking up `-a-39` (FB tab state polish: expand-persistence
across tab switch + spawn-new chord always creates new tab)
next. Independent of -38; can land in any order.
