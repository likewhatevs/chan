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
