# Phase 11 coordination protocol

Append-only directional task channels for the three phase-11 agents.
Mirrors the phase-8/phase-10 process so @@Alex can watch one place
without relaying copy/paste between agents.

## Where these files live (important with worktree-per-lane)

The per-lane git worktrees (`../chan-lane-a`, `../chan-lane-b`) are for
SOURCE CODE only. All phase-11 coordination docs (the two lane plans,
the per-agent journals, and these channels) live and are edited in the
MAIN checkout:

`/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-11/`

Read and append there by absolute path, NOT in your worktree copy. This
keeps the channels one live shared bus that @@Alex watches in one place,
and avoids git merge conflicts on append-only files across lane branches.
Code commits happen on your lane branch and merge to `main`; channel and
journal writes happen in the main checkout.

## Roster

| Handle      | Role                                                      |
|-------------|-----------------------------------------------------------|
| @@Architect | Orchestrator. Plan, dispatch, decisions, phase journal.   |
| @@LaneA     | Architect: drive streaming spine. Spawns webdev+rustacean.|
| @@LaneB     | Architect: editor/desktop/release. Spawns webdev+rustacean.|
| @@Alex      | Human owner. Watches; rules on the one design gate.       |

## Channel convention

One file per direction, named `event-<from>-<to>.md`. You APPEND to the
file whose `<to>` is the recipient. You never edit another agent's
entries. Once a peer has started a task, a new ask is a new appended
entry, not a rewrite of the old one.

| File                          | Direction                          |
|-------------------------------|------------------------------------|
| event-architect-lane-a.md     | @@Architect -> @@LaneA             |
| event-lane-a-architect.md     | @@LaneA -> @@Architect (reports)   |
| event-architect-lane-b.md     | @@Architect -> @@LaneB             |
| event-lane-b-architect.md     | @@LaneB -> @@Architect (reports)   |
| event-lane-a-lane-b.md        | @@LaneA -> @@LaneB (merge/seam)    |
| event-lane-b-lane-a.md        | @@LaneB -> @@LaneA (merge/seam)    |
| event-lane-a-alex.md          | @@LaneA -> @@Alex (escalation)     |
| event-lane-b-alex.md          | @@LaneB -> @@Alex (escalation)     |
| event-architect-alex.md       | @@Architect -> @@Alex              |

## Entry format

Append a dated, signed block. Keep it scannable.

```
## 2026-05-26 14:30 @@LaneA -> @@Architect
<one-line subject>

<body: what happened, what you need, what is blocked. Curated
highlights/lowlights/contention, not a tabular dump. Link your journal
for detail.>
```

## Merge cadence (worktree-per-lane)

- Both lanes branch off `main`: `phase-11-lane-a`, `phase-11-lane-b`.
- Merge to `main` in small frequent slices; each slice passes the full
  gate (fmt, clippy -D warnings, test, build --no-default-features, web
  build + svelte-check) before merge.
- @@LaneA owns the structural shape of the shared files
  (`store.svelte.ts`, `tabs.svelte.ts`, `lib.rs::router()`, `state.rs`)
  and lands those slices early; @@LaneB rebases onto `main` frequently.
- `App.svelte` is a two-sided merge point (Cmd+N from @@LaneB,
  overlay/status from @@LaneA): keep edits minimal, announce on the
  cross-lane channel, second-to-merge reconciles.
- Integration seam: when @@LaneA's bootstrap/init slice merges, @@LaneB
  rebases and re-validates desktop launch (Linux) against the new init
  path.

## Escalation to @@Alex

Escalate only on a human-decision blocker. @@LaneB has one standing gate:
the CLI-to-desktop handoff design note goes to `event-lane-b-alex.md` and
waits for ratification before implementation. Everything else is
architect-approved.
