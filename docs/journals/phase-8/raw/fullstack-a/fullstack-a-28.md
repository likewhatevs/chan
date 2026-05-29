# fullstack-a-28: BubbleOverlay regression cluster — filter generalization + explicit dismiss + refresh diff-merge

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Three coupled fixes to the rich-prompt bubble overlay, exposed
by @@Alex's broadcast smoke test 2026-05-20. All three land in
one commit since they sit on the same component and share the
"dismissal contract" mental model.

1. **Filter generalization** — `BubbleOverlay.visibleEvents`
   today filters source events of `type === "survey"` whose
   `id` has a sibling `*-reply-<id>.md`. Pre-flight + any poke
   that surfaced a standing-options reply (via
   `normalizeStandingOptions` "C — Check my comments first")
   land replies too, but the source bubble keeps showing.
   Change the predicate to filter ANY id with a sibling reply,
   regardless of source `type`.

2. **Explicit dismiss affordance for every bubble type** — add
   a close button to all bubbles (today only the survey gets
   dismissal-by-reply; poke + pre-flight with no reply path
   have no escape hatch). Clicking persists the dismissed id
   in a session-scoped store; the bubble stays gone across
   subsequent watcher polls. Reply-based dismissal stays as
   the preferred dismissal for surveys; explicit close becomes
   the universal escape hatch.

3. **Refresh hygiene: diff-merge over atomic-replace** —
   profile the per-poll flicker @@Alex reproduced on non-
   survey bubbles. Hypothesis: the refresh path replaces the
   visible-events array atomically (clear → re-populate) on
   each poll, producing a brief "empty" frame. Switch to a
   diff-merge that only adds ids not previously seen, only
   removes ids whose source files disappeared from the
   listing. May eliminate the flicker for all types
   regardless of dismissal contract.

If (1) + (2) alone resolve the flicker (i.e., it was actually
"bubble unmounts + remounts because the dismissal didn't
stick"), drop (3) from scope and document why in the task
tail.

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md) "Poke +
pre-flight bubbles flicker; survey bubble does not; non-survey
replies don't dismiss source bubble".

Smoke-test fixtures live at
`docs/journals/phase-8/rich-prompt/events/` — the surviving
`event-arch-survey-1.md` (source) +
`event-reply-arch-survey-1.md` +
`event-reply-arch-preflight-1.md` (replies) document the
exact JSON shape the SPA emits. The poke + pre-flight source
files were deleted from disk after @@Alex confirmed the
flicker; reproducing requires dropping a fresh `event-` or
`pre-flight-` file with `type` in `{poke, pre-flight}`.

Related code:
* `web/src/state/watcherEvents.ts` — parser + reply writer +
  the current survey-only filter (look for the `survey` /
  `survey-reply` pair handling).
* `web/src/components/BubbleOverlay.svelte` (or wherever
  `visibleEvents` is derived) — the filter consumer.
* `web/src/state/tabs.svelte.ts` — SerTab field for the
  dismissed-id set (suggest `dbi?: string[]` for "dismissed
  bubble ids"; conditional spread per the `fullstack-a-24`
  pattern so the empty case doesn't bloat the serialised
  form).

## Acceptance criteria

* Survey reply still dismisses the survey bubble (unchanged
  contract).
* Pre-flight or poke with a sibling reply file (any reply
  type) gets filtered from the visible list — same predicate.
* Every bubble (survey, poke, pre-flight) has a visible
  close affordance (icon button or `×`). Click adds the id
  to the dismissed set; the bubble disappears immediately +
  stays gone across subsequent watcher polls.
* Dismissed ids persist for the lifetime of the rich-prompt
  session (SerTab). Clearing the watcher dir / detaching the
  watcher implicitly resets visibility (since source files
  vanish from the listing).
* No flicker on any bubble type with two consecutive watcher
  poll cycles' worth of source file present.
* `vitest` green; new pinned tests cover: (a) survey reply
  dismissal still works, (b) pre-flight reply dismissal now
  works, (c) explicit close persists across a simulated
  watcher refresh.

## How to start

1. Reproduce the bugs locally:
   * Spin up a test server against any throwaway drive.
   * Point the rich-prompt watcher at a directory.
   * Drop a `poke` event file (see bug entry for shape).
   * Observe flicker + no dismiss.
2. Read the relevant code (links above) — confirm the
   survey-only filter; find the refresh path; find the
   reply writer call site.
3. Decide ordering: (1) filter generalization is one-line;
   (2) explicit dismiss is the SerTab plumbing; (3) is the
   open profile.
4. Cross-lane: pairs with `fullstack-b-13`'s survey-reply
   echo consumer. The bubble overlay's reply-write path
   today emits `poke<Enter>` to the PTY; -b-13 changes that
   to honour the per-prompt shell/agent mode toggle. Your
   work doesn't need to change the echo path itself, only
   the dismissal/filter side. Coordinate at task-cut if any
   of the bubble-overlay code touches the echo trigger.

## Coordination

* Pairs with [`fullstack-b-13`](../fullstack-b/fullstack-b-13.md)
  (shell/agent submit-mode toggle) — same consumer surface
  (rich prompt) but different code path (-b-13 owns the
  PTY-write side; this task owns the BubbleOverlay
  rendering + dismissal side).
* Hard gate before the broader rich-prompt session-evolution
  work in
  [`../architect/rich-prompt-session-evolution.md`](../architect/rich-prompt-session-evolution.md);
  the history backlog + cwd preflight + team conductor
  bands all sit on top of the bubble overlay layer.
* @@WebtestA verifies on lane-A once landed; the smoke-test
  fixtures under `docs/journals/phase-8/rich-prompt/events/`
  are the repro set.

## 2026-05-20 — implementation note + ready for review

Three-area landing in one commit covering the three task
goals plus the cross-lane seam-mapping with `fullstack-b-13`.

### Cross-lane seam-mapping (start-of-task)

@@Alex flagged at session bootstrap: "if you are working on
fullstack-a-13 or fullstack-b-28, make sure to coordinate
well before you edit the same file." -a-13 was already
committed (`887d19c`); -b-28 does not exist. Closest live
peer is `fullstack-b-13` (Shell/Agent submit-mode toggle).

Grepped the "poke" emitter before editing anything:
* SPA `web/src/components/TerminalTab.svelte:765-769`
  CONSUMES `poke<Enter>` from the PTY output stream as a
  watcher-refresh trigger.
* Server `crates/chan-server/src/terminal_sessions.rs:502`
  EMITS `b"poke\n"` to the PTY after a reply lands.

The "poke<Enter> vs poke<Cmd+Enter>" mismatch flagged in
the bug list lives in the SERVER's `send_input` call,
which is @@FullStackB's territory in -b-13. My -a-28
touches none of that path. Shared SerTab fields are
non-overlapping (my `dbi`, their `rpsm`). Clean split.

### Filter generalization (goal 1)

The `BubbleOverlay.visibleEvents` predicate from
`fullstack-a-5` already filtered any non-reply source
event whose `id` had a sibling `survey-reply` — the
comment above said "surveys" but the code was already
type-agnostic. Refreshed the comment to match reality
and added two test pins to lock the predicate against
silent regressions on pre-flight + poke source types.

(The bug-list note "Root cause confirmed: filter only
matches `type === 'survey'`" was a misread of the code
comment; the actual predicate was already general. The
visible bug @@Alex saw — pre-flight bubble not
dismissing — was the per-poll `Loading...` flicker
hiding the post-reply filter outcome. Fixed in goal 3
below.)

### Explicit dismiss affordance (goal 2)

New `X` icon button on every bubble's
`.bubble-head-actions` row (after refresh, before any
future header chrome). aria-label "Dismiss bubble".
Click → `dismissExplicit(event.id)`:

* Appends id to `watcher.dismissedIds` (new field on
  `TerminalWatcherState`).
* Immediately filters the event out of `watcher.events`
  so the bubble drops on the next reactive cycle.

`visibleEvents` honours the per-tab `dismissedIds` set
in addition to the existing reply-based filter. Reply-
based dismissal stays the preferred path for
surveys + pre-flight standing options; explicit close
is the universal escape hatch (poke + any bubble the
user wants gone without replying).

Persisted on `SerTab.dbi: string[]` with conditional
spread (empty case keeps the persisted shape short;
shareable URL hash excludes the field via the existing
`opts.terminalSessions` gate).

### Diff-merge / flicker (goal 3)

Profiled the per-poll flicker. The atomic
`tab.watcher.events = events` reassignment in
`TerminalTab.svelte:754` is fine — Svelte 5's
`#each (event.id)` keyed iteration preserves DOM
identity. The actual flicker source is the template's
`{#if watcher.loading} Loading... {:else}` branch: every
poll flips `watcher.loading` true then false, which
swaps the bubble list OUT for a `Loading...` placeholder
and back IN. For ~50ms on every poll cycle, the bubble
column visibly empties.

Surveys did NOT flicker for @@Alex because the survey
path took the `dismissEvent(id, 600)` fast path and the
bubble was gone before the next poll's Loading swap fired.
Poke + pre-flight stayed on screen across polls, so the
swap was visible.

One-line tightening: only render the Loading placeholder
when `visibleEvents.length === 0`. Subsequent polls keep
the bubble list visible during the watcher refresh
roundtrip. Documented inline with the rationale.

Skipped the full diff-merge restructure in
`TerminalTab.svelte` — not needed once the Loading-swap
is gated, and the atomic reassignment keeps the data
path simpler than a manual splice. If a future surface
exposes per-event identity churn (e.g. animated entry /
exit), revisit.

### Files touched

* `web/src/state/tabs.svelte.ts`
  * `TerminalWatcherState`: new `dismissedIds?: string[]`.
  * `SerTab`: new `dbi?: string[]` with conditional spread
    on serialize; deserialize at both restore sites + clone.
* `web/src/components/BubbleOverlay.svelte`
  * Filter honours `dismissedIds`; comment refreshed.
  * New `dismissExplicit(id)` + `X` button per bubble.
  * Loading placeholder gated on `visibleEvents.length === 0`.
* `web/src/components/BubbleOverlay.test.ts` (+3 tests)
  * Pre-flight + survey-reply siblings → filtered.
  * Poke + survey-reply siblings → filtered.
  * Dismiss button populates `dismissedIds` + drops event;
    companion mount asserts the predicate at first render.
* `web/src/state/tabs.test.ts` (+2 tests)
  * SerTab.dbi round-trip with non-empty `dismissedIds`.
  * Empty `dismissedIds` omits `dbi` from the persisted shape.

### Gate

* `vitest`: 512/512 (+5 from baseline 507).
* `svelte-check`: 0 errors / 0 warnings / 3974 files.
* `npm run build`: clean.
* Rust gate: no Rust changes; not run.

### Suggested commit subject

`BubbleOverlay: explicit dismiss + dismissedIds persistence + Loading flicker fix (fullstack-a-28)`

### Cross-lane handoff for fullstack-b-13

If @@FullStackB ends up extending the
`writeTerminalEventReply` request shape with a
`submit_mode` field, my `BubbleOverlay.commit()` is the
sole SPA call site for that API surface — easy single
threading. SerTab additions land in distinct fields
(`dbi` vs `rpsm`) with no collision risk on commit
ordering.
