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
