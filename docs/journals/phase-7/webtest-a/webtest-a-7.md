# webtest-a-7: Round 2 wave-B walkthrough lane (Lane A)

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Rolling walkthrough on Round 2 wave-B as it lands. Lane A
angle: the frontend surface — spawn dialog, pre-flight
survey rendering, activity indicator UX on tab strip.

## Relevant links

* Wave-B tasks:
  * Backend: [../systacean/systacean-12.md](../systacean/systacean-12.md),
    [../systacean/systacean-13.md](../systacean/systacean-13.md),
    [../systacean/systacean-14.md](../systacean/systacean-14.md).
  * Frontend: [../fullstack/fullstack-20.md](../fullstack/fullstack-20.md).
  * SKILL: [../architect/architect-1.md](../architect/architect-1.md).

## Acceptance criteria

Report PASS / FAIL / PARTIAL per cluster.

### When `fullstack-20` + `systacean-12` land

1. "Spawn agent" affordance visible in rich-prompt
   context menu.
2. Dialog accepts tab name + command + env. Submit
   spawns the agent in the active pane; tab appears
   with the chosen name.
3. Spawn a `bash -c 'echo hi; sleep 5; echo bye'`;
   verify tab captures both lines.
4. Spawn a command that triggers pre-flight (e.g. a
   shell script that prints "please log in" first):
   bubble overlay renders the pre-flight survey with
   1/2/3 options (open terminal, kill, retry).
5. Spinner + counter visible next to the pre-flight
   bubble; ticks every second.
6. Pick option 2 (kill) — spawn process exits, tab
   closes.

### When `systacean-13` lands

7. Activity indicator on terminal tabs: produce
   output in an unfocused terminal, see the marker
   appear on its tab. Focus the tab — marker clears.
8. Distinguish from existing dirty / watcher bullets
   — no visual collision.

### When `systacean-14` lands

9. Spawn `claude` (or whichever agent we have local);
   confirm chan's MCP server appears in claude's
   config without manual setup.
10. Verify the user's existing MCP entries are
    untouched.

### When `architect-1` lands

11. Read the orchestration SKILL files; confirm the
    spawn protocol matches what `fullstack-20` /
    `systacean-12` actually do; flag any drift.

## How to start

* Test drive `/tmp/chan-webtest-a-1/` and 8801 still
  yours. Rebuild + bounce after each commit.
* Permission scope carried over.

## Hand-off

Ping after each cluster via
`alex/event-webtest-a-architect.md`.
