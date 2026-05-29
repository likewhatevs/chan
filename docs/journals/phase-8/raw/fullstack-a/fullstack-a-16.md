# fullstack-a-16: Hybrid NAV help overlay copy uses "Stage:" but runtime is immediate-commit

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Update the Hybrid NAV help overlay copy so the 1/2/3 chord
descriptions match the immediate-commit runtime behaviour
that landed in `fullstack-a-3`. Today the help text still
labels them as "Stage:" implying a two-step stage-then-
commit flow that the runtime no longer follows.

## Background

Side observation from @@WebtestA's Round-1 sweep on
2026-05-20:

> Hybrid NAV help overlay still labels 1/2/3 as "Stage:"
> while the runtime behavior is immediate-commit (bug 7).

Filed in [`../phase-8-bugs.md`](../phase-8-bugs.md). Trivial
copy fix; small but Alex-visible inconsistency between docs
and behaviour.

## Acceptance criteria

* `PaneModeHelp.svelte` (or wherever the overlay copy lives)
  labels 1/2/3 with the immediate-commit verb (e.g. "Commit
  to slot 1/2/3" or "Save to slot 1/2/3" — whatever reads
  naturally and matches the actual runtime effect).
* No regression on the other chord descriptions in the
  same overlay.
* No code-side change required — pure copy update.

## How to start

1. Open `web/src/components/PaneModeHelp.svelte`.
2. Find the rows labelled "Stage:" for 1/2/3.
3. Replace with the immediate-commit verb.
4. Reload the lane-A test server and confirm the overlay
   matches the runtime behaviour Alex flagged in
   `fullstack-a-3`.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.

## 2026-05-20 — implementation note

`PaneModeHelp.svelte` Spawn section had three rows labeled
`Stage: Terminal` / `Stage: File Browser` / `Stage: Graph`.
The "Stage:" prefix was a leftover from the pre-`fullstack-a-3`
era when 1/2/3 staged a spawn intent that committed only on
Enter. `fullstack-a-3` collapsed that to immediate-commit:
pressing the number spawns the target right then, no Enter
required.

Replaced the action verbs with `Spawn Terminal` / `Spawn File
Browser` / `Spawn Graph` — immediate-effect verbs that match
the section title and the runtime behaviour. Refreshed the
leading section comment to record the immediate-commit rule
and drop the stale stage-and-commit framing (the `fullstack-72`
attribution was load-bearing for the old behaviour, so the
new comment cites `fullstack-a-3` instead).

Also updated `paneModeHelpClickable.test.ts` line 59: the
regex pinned `action:\s*"Stage: Terminal"` as the marker on
the terminal-spawn row to verify the cap structure
(`fullstack-b-9` added the `t` alias). Updated the literal
to `"Spawn Terminal"` so the assertion keeps tracking the
same row.

Files touched:

* `web/src/components/PaneModeHelp.svelte` — three action
  labels + section comment.
* `web/src/components/paneModeHelpClickable.test.ts` — match
  the new action label on the `1`/`t` row pin.

Pre-push gate (SPA portion): vitest 480/480 green;
`npm run check` 0 errors / 0 warnings; `npm run build` clean.

To verify on the lane-A server (post-restart): enter Hybrid
NAV (Cmd+. on the new binding), press H to open the help
overlay, see the Spawn section reads "Spawn Terminal" /
"Spawn File Browser" / "Spawn Graph" — no "Stage:" prefix.
Press 1 and confirm a terminal spawns immediately (the
runtime behaviour the copy now describes).

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Clean copy fix. "Spawn" reads naturally as an immediate-
effect verb that matches the section title and the actual
runtime behaviour from `fullstack-a-3`. Updating the
`paneModeHelpClickable.test.ts` regex literal so the
fullstack-b-9 row pin keeps tracking the same row is the
right hygiene — those structural-test invariants are load-
bearing for the chord-table audit. Citing `fullstack-a-3`
in the refreshed comment instead of the stale
`fullstack-72` attribution is correct.

Pre-push gate green.

**Commit clearance**: approved. Suggested commit subject:

```
PaneModeHelp: "Stage:" → "Spawn" to match immediate-commit runtime (fullstack-a-16)
```

Push waits for Round-1 close.