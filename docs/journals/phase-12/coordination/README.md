# Phase 12 coordination protocol

Append-only directional channels for the phase-12 agents. Mirrors phase-8/10/11
so @@Alex watches one place without relaying copy/paste.

## Where these files live

Per-lane git worktrees (`../chan-lane-{a,b,c}`) are for SOURCE CODE only. All
phase-12 coordination docs (lane plans, per-agent journals, these channels)
live and are edited in the MAIN checkout:

`/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-12/`

Read + append there by ABSOLUTE PATH, not in your worktree copy. Code commits
happen on your lane branch and merge to `main` via @@Architect; channel +
journal writes happen in the main checkout.

## Roster

| Handle      | Role                                                        |
|-------------|-------------------------------------------------------------|
| @@Architect | Orchestrator. Plan, dispatch, serialize merges, re-gate.    |
| @@LaneA     | Graph + File Browser carryover. May spawn 2-3 subagents.    |
| @@LaneB     | Scoping architect: drive -> workspace terminology/codemod.  |
| @@LaneC     | @@Alex ad-hoc: frontend/cosmetics, keyboard shortcuts.      |
| @@Alex      | Human owner. Drives @@LaneC; rules on scope/infra calls.    |

@@Alex will add more lanes later (a release/build lane is the likely next).

## Channel convention

One file per direction, `event-<from>-<to>.md`. APPEND to the file whose
`<to>` is the recipient; never edit another agent's entries.

| File                       | Direction                              |
|----------------------------|----------------------------------------|
| event-architect-lane-a.md  | @@Architect -> @@LaneA                 |
| event-architect-lane-b.md  | @@Architect -> @@LaneB                 |
| event-architect-lane-c.md  | @@Architect -> @@LaneC                 |
| event-lane-a-architect.md  | @@LaneA -> @@Architect (reports)       |
| event-lane-b-architect.md  | @@LaneB -> @@Architect (reports)       |
| event-lane-c-architect.md  | @@LaneC -> @@Architect (reports)       |
| event-lane-a-alex.md       | @@LaneA -> @@Alex (escalation)         |
| event-lane-b-alex.md       | @@LaneB -> @@Alex (scope/infra gates)  |
| event-lane-c-alex.md       | @@LaneC -> @@Alex (escalation)         |
| event-alex-lane-c.md       | @@Alex -> @@LaneC (ad-hoc requests)    |
| event-architect-alex.md    | @@Architect -> @@Alex                  |

Cross-lane channels `event-lane-X-lane-Y.md` are created on first use (the
codemod-vs-feature seams will need a-b/b-a, a-c/c-a, b-c/c-b).

## Entry format

```
## 2026-05-27 14:30 @@LaneA -> @@Architect
<one-line subject>

<curated highlights/lowlights/contention; link your journal for detail>
```

## The phase-12 coordination problem

All three lanes touch `web/src`; @@LaneB's codemod touches ~everything. @@LaneB
scopes FIRST (no codemod while scoping); the codemod lands in an @@Architect-
chosen sequenced window (quiescent checkpoint or last/mechanical), announced as
a brief freeze on the affected files. Declare cross-lane touches. See
`../bootstrap.md` for the full strategy.

## Escalation to @@Alex

@@LaneB has a standing gate: the drive->workspace scope decisions (tunnel
domain rename, crate/registry break, team-workspace collision) go to
`event-lane-b-alex.md` and wait for ratification before the codemod. Otherwise
escalate only on a human-decision blocker.
