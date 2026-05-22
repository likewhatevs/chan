# fullstack-a-79 — Team bootstrap orchestrator (config write + spawn + watcher + template + identity prompt + pre-flight)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: addendum-b wave-1
Dependencies: `systacean-30` (config schema), `systacean-31` (watcher), `fullstack-a-78` (dialog), `fullstack-a-81` (process template)

## Goal

Execute the Bootstrap action from the New Team
dialog. Wire all the pieces together: persist
config → prepare real estate → spawn terminals
with env → load team watcher → place process
template → prompt agents with identity → trigger
pre-flight survey from lead.

## Reference

[`../alex/addendum-b.md`](../alex/addendum-b.md)
§"When they click Bootstrap" + §"Clarifications"
#1 (lead = coordinator = same terminal as user's
rich-prompt) + #4 (unescaped prompt) + #5 (spawn
with name from get-go) + #6 (template params) +
#7 (relative paths).

## Bootstrap sequence

### 1. Persist config

Call `Drive::create_team(team_name, config)` from
`systacean-30`. Config persists at
`Drafts/team-{name}/config.toml`.

### 2. Place process template in team's docs/

Per `-a-81`'s generalised template:

* Copy `docs/agents/bootstrap.md` (parameterised) to
  `Drafts/team-{name}/docs/bootstrap.md` with
  `{host-handle}` + `{lead-handle}` substituted.
* Copy related process docs (journals dir scaffold,
  task-file template) with the same param
  substitution.

### 3. Prepare real estate

Per dialog's airplane-grid choice:
* **Tabs-in-current-Hybrid**: spawn N terminals as
  tabs in the current pane.
* **Split-pane**: execute splits to match the grid
  shape; in each cell, spawn the assigned robots
  (multi-robot = multi-tab in that cell).

### 4. Spawn terminals with env

For each member (including the lead):
* Lead's terminal IS the user's current rich-prompt
  terminal (per clarification #1) — restart it with
  the lead's command + env, OR repurpose the
  existing terminal in place.
* Worker terminals: spawn with `CHAN_TAB_NAME=<handle>`
  env from the get-go (per clarification #5; the
  existing 1-agent spawn button already does this).

### 5. Load the team watcher

Call chan-server's `team_load_start(team_name)` IPC
from `systacean-31`. Per-team event channel watcher
comes up.

### 6. Send identity prompt to each agent

Per clarification #4: send the literal string
`I'm {host-handle}. You're $CHAN_TAB_NAME. Identify yourself, and then read docs/agents/bootstrap.md`
to each agent terminal. The `$CHAN_TAB_NAME` is
NOT escaped — agents read it as a live env-var.

Use the existing `dispatch_agent_event` pathway
(per `systacean-21`'s rich poke + path/heading
fields if applicable).

### 7. Trigger lead's pre-flight

The lead receives a slightly different prompt OR a
follow-up event instructing them to run the
pre-flight check + send a survey to the host
("are the agents up and running?" + next-step
description). The survey uses the existing
survey/survey-reply event shape.

## Acceptance

1. **Click Bootstrap → config persists** at the
   correct path.
2. **Real estate set up** per dialog choice.
3. **All N terminals spawn** with correct
   `CHAN_TAB_NAME` env (verifiable via the agent
   echo).
4. **Team watcher active** (event in team's events/
   dir flows through to chan-server + SPA).
5. **Process template placed** at
   `Drafts/team-{name}/docs/bootstrap.md` with
   substitutions.
6. **Identity prompt delivered** to each agent.
7. **Lead runs pre-flight + sends survey** to the
   host. Host sees the survey in their Rich Prompt
   bubble overlay (existing survey UX).

### Tests

Vitest pins for each stage's call surface +
integration test for the full bootstrap flow against
a throwaway drive.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA SPA primary.
* Heavy cross-component: depends on chan-drive
  (`-30`) + chan-server (`-31`) + process template
  (`-81`) + dialog (`-78`).
* Sequencing: wait for `-30` + `-31` + `-78` + `-81`
  in HEAD; then start.
* Can shell-and-stub stages 1-4 in parallel with
  dependencies; full integration when all deps
  land.

## Authorization

Yes for SPA orchestrator + IPC bridging + tests +
task tail + outbound.

## Numbering

This is `-a-79`.

## Out of scope

* Load existing team (`-a-80`).
* Process template content (`-a-81`).
* chan-drive primitives (`-30`).
* chan-server watcher IPCs (`-31`).
