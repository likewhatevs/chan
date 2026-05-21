# fullstack-a-47 — Drop front/back independent theme (Task E)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 lands in HEAD)

## Goal

Simplify the per-Hybrid theme override (from `-b-5`)
from front/back independent values to a SINGLE per-
Hybrid value. The hamburger theme toggle from `-a-27`
flips the single value; both sides of the Hybrid share
it.

## Background

Locked design:
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited" — Task E.

`-b-5` (phase-7) introduced per-Hybrid theme that
defaulted to inheriting from the global theme but
could be overridden per Hybrid. The override was
front/back independent (each side of the flip had its
own theme value). Under the new Hybrid back-side
semantics (back = config surface, not a tab collection),
the front/back theme split is no longer load-bearing:
the config surface should always render with the same
theme as the front surface for visual coherence.

## Acceptance criteria

* Per-Hybrid theme stored as a single value, not a
  front/back pair.
* Hamburger theme toggle (`-a-27`) flips the single
  value.
* Both sides of every Hybrid render with the same
  theme.
* Migration: existing user preferences with separate
  front/back theme values collapse cleanly to a single
  value (pick the front-side value as the canonical
  one).
* Tests cover the migration shape + the flip-toggle
  behaviour.
* Pre-push gate green.

## How to start

1. Audit the per-Hybrid theme storage shape in the
   Preferences / persistence layer.
2. Collapse the front/back pair to a single value.
3. Update the hamburger theme toggle binding.
4. Add a one-time migration on load (pick front-side
   value if both exist).
5. Wire tests.

## Coordination

* SPA-only.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereq

Depends on
[`fullstack-a-43`](fullstack-a-43.md) landing in HEAD.

## Numbering

This is `-a-47`. See `-a-45` for the broader wave
numbering note.

## 2026-05-21 — ready for review

Two-file change. SPA state-only; no Rust touched.

### Architecture

`HybridSide` collapsed to an empty marker type
(`{}`). Its only field was `theme?`; under `-a-47`
that override moved out of the per-side slot into
the single per-Hybrid value at `pane.theme`. The
marker type stays so `pane.back !== undefined`
still discriminates "this pane is a Hybrid (has
been flipped at least once)" from "never flipped"
for menu-gating in `Pane.svelte`.

`flipHybrid` simplified: no more theme swap, no
more `inverseTheme` helper. Lazy-init still
materialises `pane.back = {}` on first flip so the
discriminator works. The function reduces to
`back ??= {}; showingBack = !showingBack;
requestPaneFlip()`.

`Pane.svelte` is unchanged — the existing
`pane.back !== undefined` gate on the hamburger
Theme / Flip entries still works because
`pane.back` is still set (empty object) on first
flip.

`togglePaneTheme` in `Pane.svelte` is unchanged —
it already flipped `pane.theme` directly (the
single per-Hybrid value); no front/back ceremony.

### Serialization

* **`hb` field removed from emit.** New session blobs
  / URL hashes never carry a back-side theme override.
* **`bm` (back-materialised marker) added.** A
  flipped pane with no per-side theme still serializes
  its Hybrid-ness via `bm: 1`. Without this, a Hybrid
  with no theme override and `showingBack === false`
  would round-trip into a non-Hybrid pane (back
  marker lost) — and the next flip would behave like
  the first one ever, which is wrong.
* **Legacy `hb` accepted on rehydrate, then dropped.**
  Per the task spec migration: "pick the front-side
  value as the canonical one." `ht` (front) wins;
  `hb` (back) is ignored. The PRESENCE of `hb` on
  the wire (even when dropped) implies the pane
  was a Hybrid, so the back marker materialises
  (same as `bm`). `bt` (legacy back-side tabs)
  ALSO still implies the pane was a Hybrid; both
  legacy signals materialise the back marker.

### Migration coverage

* User with theme on front only: `ht` survives;
  `pane.theme` set; no change in behaviour.
* User with theme on back only (pre-`-a-47`
  `showingBack=true` and changed theme there):
  the old serializer would have put the visible
  theme into `ht` (the "front" slot from the
  swap-flip semantics). Worst case is "user
  customized theme only while showingBack" —
  uncommon enough to be acceptable per the task
  spec.
* User with different themes on each side: front
  wins. The back-side preference is lost. Task
  body called this out explicitly ("pick the
  front-side value as the canonical one").

### Tests

Three existing tests in
`describe("Hybrid flip (...)")` rewritten to
match the new contract:

* "first flip materialises back marker;
  pane.theme is preserved" — was "...lazy-
  initializes back with inverted theme...".
* "flipping back round-trips showingBack;
  pane.theme is single + stable" — was "...
  round-trips showingBack + theme...".
* "serialize / restore round-trips theme +
  showingBack + back marker" — was "...per-
  side themes + showingBack..." now asserts
  `bm: 1` emitted and `hb` NOT emitted.

One NEW test:

* "legacy `hb` payload is accepted on rehydrate
  and dropped" — pins the migration shape:
  `ht` wins, `hb` ignored, back marker
  materialised.

The flip-bus-bump test + no-op-on-bad-id test
unchanged.

### Gate

* vitest **622 / 622** (+1 net: +1 new legacy
  test; 3 existing rewritten in place).
* svelte-check 0 errors / 0 warnings across
  3989 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions flagged

* **`bm` (back-materialised) marker added** to
  the wire format. Tiny field (1 char id + the
  value `1`). Required so a Hybrid pane with no
  per-side theme can round-trip. Alternative
  considered: drop the back-discriminator
  entirely and treat every pane as potentially
  Hybrid (so the hamburger Theme/Flip entries
  always show). Cleaner but a UX papercut
  (un-flipped panes shouldn't advertise back-
  side ops). Flag if `bm` is the wrong shape.
* **Front-side wins on legacy migration**.
  Matches the task spec. Alternative: pick the
  visible-side theme (use `sb` to disambiguate).
  Slightly more user-aware but more
  implementation. Going with task-spec default.
* **`inverseTheme` helper deleted** — no longer
  called now that flipHybrid doesn't swap.
  Clean removal; not exported.

### Suggested commit subject

```
Drop front/back independent theme; single per-Hybrid value (fullstack-a-47)
```

Single commit. State shape + flip impl + ser/
deser + test updates are all part of the same
collapse.

### Files for `git add` (per-path discipline)

* `web/src/state/tabs.svelte.ts`
* `web/src/state/tabs.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-47.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
  (commit-readiness poke)

Push held — multi-agent tree commit discipline.
Standing by for clearance.
