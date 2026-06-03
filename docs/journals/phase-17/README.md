# Phase 17

Opened after v0.24.0 (the phase-16 close). Phase-17 gathers @@Alex's new
requirements plus a few items left out of the phase-16 cut. The launcher
redesign that finished just before this phase is archived one level up in
docs/journals/round-16/ (its 3 smoke follow-ups are folded into round-1).

## round-1/

@@Alex's round-1 draft: bugs found during manual testing, enhancements,
documentation rewrites, the chan gateway writeup, and the website
screenshot plan.

- draft.md          the round-1 spec (titled "Phase 17 - round 1")
- image*.png        screenshots referenced inline by draft.md

## round-2/

@@Alex's second report (moved here from ./alex-report-2). Three items:
open-source attribution on the about page, a list paste-link indent bug, and
per-terminal surveys. Triaged onto the existing lanes; dispatched as each
finishes round-1.

- draft.md          the round-2 spec
- image*.png        screenshots referenced inline by draft.md
- plan.md           lane triage + sequencing

## team/

The team-work bus for the next session, ready to launch.

- config.toml       4-lane team (@@LaneA lead + @@LaneB/C/D workers)
- bootstrap.md      team process + the round-1 execution plan (lane
                    assignments, file boundaries, sequencing, acceptance)
- tasks/ journals/ followups/   empty; filled live during the round

Launch the round with:

    cs terminal team load docs/journals/phase-17/team

Use `load` (NOT `new`): `new` regenerates bootstrap.md from config.toml and
would wipe the hand-authored plan. `load` reads the config, spawns the four
agents, and pokes each to read this bootstrap.md.
