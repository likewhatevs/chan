# Phase 13 coordination protocol

Append-only directional channels for the phase-13 agents. Mirrors
phase-8 / 10 / 11 / 12 so @@Alex watches one place.

## Where these files live

Per-lane git worktrees (`../chan-lane-{a,b}`) are for SOURCE CODE only.
All phase-13 coordination docs (lane requests, per-agent journals,
these channels) live and are edited in the MAIN checkout:

`/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/`

Read + append there by ABSOLUTE PATH, not in your worktree copy.

## Roster

| Handle  | Role                                                         |
|---------|--------------------------------------------------------------|
| @@Alex  | Human owner. Coordinates execution; final word on scope.     |
| @@LaneA | Content surfaces: Editor + Terminal + Inspector.             |
| @@LaneB | Structural shell: Pane chrome + Graph + Dashboard.           |
|         | ALSO merge-gate orchestrator: serializes merges to main +    |
|         | cuts v0.17.0.                                                |

## Channel convention

One file per direction, `event-<from>-<to>.md`. APPEND to the file whose
`<to>` is the recipient; never edit another agent's entries.

| File                       | Direction                                |
|----------------------------|------------------------------------------|
| event-alex-lane-a.md       | @@Alex -> @@LaneA                        |
| event-alex-lane-b.md       | @@Alex -> @@LaneB                        |
| event-lane-a-alex.md       | @@LaneA -> @@Alex (reports/merge-ready)  |
| event-lane-b-alex.md       | @@LaneB -> @@Alex (reports + merge-gate  |
|                            | confirmations + release cut)             |
| event-lane-a-lane-b.md     | @@LaneA -> @@LaneB (created on first use)|
| event-lane-b-lane-a.md     | @@LaneB -> @@LaneA (KIND route signature)|

## Entry format

```
## 2026-05-28 14:30 @@LaneA -> @@Alex
<one-line subject>

<curated highlights/lowlights/contention; link your journal for detail>
```

Per `feedback_curated_status_reports`: highlights / lowlights /
contention, not tabular dumps. Detail in the journal.

## Cross-lane sequencing

Lane A's Inspector KIND-chip wiring is gated on Lane B's KIND route
signature. @@LaneB posts the signature on `event-lane-b-lane-a.md` as
early as possible; @@LaneA wires after.

All other touches are file-disjoint. Declare any unexpected `web/src`
overlap on the cross-lane channels BEFORE editing.

## Escalation to @@Alex

Standing gate: scope creep, release timing, anything not in
`roadmap-round-1.md`. Otherwise escalate only on a human-decision
blocker.

## Docs commit timing

Per `feedback_coordination_docs_commit_timing`: keep phase-13
plans / journals / channels UNTRACKED / dirty as the live bus during
the round; commit as `docs(phase-13): close round 1` at round close.
