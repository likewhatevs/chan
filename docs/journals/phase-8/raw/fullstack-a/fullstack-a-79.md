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

## 2026-05-23 — slice 1 (orchestrator core chain)

SPA-only. `systacean-41` shipped the
chan-server team create/duplicate routes
+ killed the silent axum-syntax bug on the
`-31` load/unload routes. Unblocked; slice 1
wires the orchestrator's core chain.

### Shape applied

**API client extension**

* New types: `TeamMemberWire`,
  `TeamConfigWire`, `TeamRefView`,
  `TeamLoadResponse` — snake_case mirrors of
  chan-drive's `TeamConfig` / `Member` /
  `TeamRef` / `TeamLoadResponse`.
* 5 new endpoints: `teamCreate`, `teamLoad`,
  `teamUnload`, `teamListLoaded`,
  `teamDuplicate`.

**New `state/teamOrchestrator.svelte.ts`**

* `parseEnvLines(text)` — KEY=VALUE
  newline-separated → Record<string,string>.
  Mirrors `SpawnDialog.svelte`'s parser.
* `memberHandle(member, autoPrefix)` —
  `@@<name>` when autoPrefix on AND name
  doesn't already start with `@@`; raw
  otherwise. Matches the dialog's
  `handleOf` helper.
* `translateConfig(config)` — SPA
  camelCase `TeamDialogConfig` →
  chan-drive snake_case `TeamConfigWire`.
  Auto-injects `CHAN_TAB_NAME=<handle>`
  per member when env doesn't already
  carry it (addendum-b clarification #8).
  Per-call `created_at` set to ISO 8601 UTC.
* `identityPrompt(hostHandle)` — addendum-b
  clarification #4 verbatim. `$CHAN_TAB_NAME`
  intentionally NOT escaped so worker
  shells expand it.
* `runTeamBootstrap(config, hostSessionId?)`
  — orchestrator entry point. Steps:
  1. `api.teamCreate(name, wire)` persists
     `config.toml` at
     `Drafts/team-{name}/config.toml`.
  2. `api.teamLoad(name)` spins up the
     per-team watcher.
  3. For each non-lead member,
     `api.spawnTerminal({ name: handle,
     command, env, orchestrator_session:
     hostSessionId })` + opens a TerminalTab
     in the active pane with the new
     session + seedInput = identityPrompt.
  4. notify() on success.
  Split-pane real estate scope-poked via
  notify (slice 1 falls back to tabs).

**TerminalRichPrompt.svelte rewiring**

* `openNewTeamDialog`'s `onBootstrap`
  callback swapped from log-only to
  `await runTeamBootstrap(config,
  terminalSessionId)`.

### Slice 2 deferred

Per the addendum-b spec:

* Process-template placement (copy
  `-a-81`'s parameterised docs into
  `Drafts/team-{name}/docs/`).
* Lead-side pre-flight survey trigger.
* Split-pane real estate (paneSplit +
  per-cell assignment).
* `dispatch_agent_event`-driven identity
  prompts (slice 1 uses `seedInput` for
  in-process delivery; the event-channel
  path lands when `systacean-21`'s
  rich-poke flow consumes a team
  channel).

### Files touched

* `web/src/api/client.ts`
  * 5 team endpoints + 4 wire-shape
    interfaces.
* `web/src/state/teamOrchestrator.svelte.ts`
  (new) — orchestrator module.
* `web/src/components/TerminalRichPrompt.svelte`
  * Import `runTeamBootstrap`.
  * `onBootstrap` callback dispatches the
    orchestrator.
* `web/src/state/teamOrchestrator.test.ts`
  (new) — 17 architectural pins for
  parseEnvLines / memberHandle /
  translateConfig / identityPrompt.
* `web/src/components/teamBootstrapOrchestrator.test.ts`
  (new) — 10 integration pins for the
  TerminalRichPrompt dispatch, the
  orchestrator's chain ordering, and the
  api client team endpoints.

### Decisions

* **Lead's terminal = host session**, per
  addendum-b clarification #1. The
  orchestrator doesn't spawn a new terminal
  for the lead; the user's existing rich-
  prompt terminal IS the lead's surface.
  Slice 1 skips the lead in the spawn
  loop; the identity prompt for the lead
  arrives via the existing rich-prompt
  buffer (slice 2 wires the auto-fill).
* **Auto-inject CHAN_TAB_NAME** (addendum-b
  clarification #8) — users don't have to
  type it in the env field; the orchestrator
  fills it from the resolved handle. User
  overrides preserved.
* **seedInput for identity prompt** —
  in-process delivery via the existing
  terminal seed. The task body mentions
  `dispatch_agent_event`; that's a separate
  delivery path (event-channel) that
  belongs to a slice 2 once the team
  event channel is consumed.
* **Split-pane scope-poked**, not silently
  fallen back. The dialog supports a
  split-pane real estate today; the
  orchestrator surfaces a notify() so the
  user sees the limitation.

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1278 / 1278** (+27 from
  `-a-75b`'s 1251).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy
  --all-targets -- -D warnings` → clean
  (no Rust delta).

### Suggested commit subject

```
Team Bootstrap orchestrator slice 1: config + load + spawn + identity prompt (fullstack-a-79 slice 1)
```

### Files (per-path)

* `web/src/api/client.ts`
* `web/src/state/teamOrchestrator.svelte.ts` (new)
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/state/teamOrchestrator.test.ts` (new)
* `web/src/components/teamBootstrapOrchestrator.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-79.md`

Autonomous-commit mode. No clearance held.
Picking up `-a-80` (Load Team flow) next —
duplicate branch + dialog populated with
existing config + already-loaded notice.

## 2026-05-23 — slice 2 (lead identity prompt via rich-prompt buffer)

SPA-only follow-up. Closes the lead-side
identity prompt gap from slice 1.

### Shape applied

Per addendum-b clarification #1, the lead's
terminal IS the user's current rich-prompt
terminal (the host session). Slice 1 skipped
the lead in the spawn loop; slice 2 delivers
the identity prompt to it by populating the
rich-prompt buffer programmatically.

**Two new helpers in `tabs.svelte.ts`**

* `findTerminalBySession(sessionId)` walks
  `allTerminalTabs()` + matches on
  `terminalSessionId`. Returns null when no
  matching tab is open — the orchestrator
  silently skips lead-prompt in that case
  (e.g. host terminal closed mid-flight).
* `primeTerminalRichPrompt(tab, text)` —
  initializes the buffer + flags open. If
  `richPrompt` is already armed, overwrites
  buffer + flips open + defaults mode to
  wysiwyg.

**Orchestrator step 4**

After the worker spawn loop:

```ts
if (hostSessionId) {
  const leadTab = findTerminalBySession(hostSessionId);
  if (leadTab) primeTerminalRichPrompt(leadTab, prompt);
}
```

Followed by the existing notify("Team
…bootstrapped.") so the success message
fires only after the lead is staged.

### Decisions

* **Rich-prompt buffer, not seedInput** —
  the lead's terminal is already mounted +
  attached to a live PTY; `seedInput` is
  for fresh terminal opens. The buffer
  surface lets the user review + submit
  the identity prompt themselves, matching
  the existing rich-prompt UX for outbound
  agent dispatches.
* **Silently no-op when host session can't
  be found** — closed terminals or wrong
  invocation surfaces don't deserve a
  notify spam; the workers still got their
  prompts.
* **`mode ??=` on the already-armed
  branch** — preserves an existing `source`
  selection if the user happened to be
  composing in source mode; otherwise
  defaults to wysiwyg.

### Files touched

* `web/src/state/tabs.svelte.ts`
  * Added `findTerminalBySession` +
    `primeTerminalRichPrompt` exports.
* `web/src/state/teamOrchestrator.svelte.ts`
  * Imports the two new helpers.
  * Step 4 added after the spawn loop.
* `web/src/state/teamLeadPrompt.test.ts`
  (new) — 6 architectural pins for the
  helpers + the orchestrator step ordering.

### Still deferred to slice 3+

* Process-template placement
  (`Drafts/team-{name}/docs/bootstrap.md`)
  — needs decision on template-source
  delivery (bundle into SPA via vite ?raw
  vs chan-server endpoint).
* Lead pre-flight survey trigger —
  needs the survey shape to consume the
  team event channel.
* Split-pane real estate — paneSplit loop
  + per-cell tab assignment. Larger SPA
  piece; needs its own slice.
* `dispatch_agent_event`-driven identity
  prompts — needs the team event channel
  consumer in chan-server.

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1295 / 1295** (+17 from
  `-a-80` slice 1's 1278; 6 new pins for
  slice 2 + 11 net from -a-80 slice 1's
  earlier batch).
* `npm run build` → clean.
* Rust gate: chan-server build flagged a
  separate lane's unfinished WIP
  (`api_team_get_config` added but not
  route-registered — looks like
  @@Systacean responding to my `-a-80`
  slice-2 scope-poke). NOT committed in
  this slice; leaving the WIP in the
  working tree for the lane to finish.
  This slice is SPA-only.

### Suggested commit subject

```
Team orchestrator slice 2: lead identity prompt via rich-prompt buffer (fullstack-a-79 slice 2)
```

### Files (per-path)

* `web/src/state/tabs.svelte.ts`
* `web/src/state/teamOrchestrator.svelte.ts`
* `web/src/state/teamLeadPrompt.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-79.md`

Autonomous-commit mode. No clearance held.

## 2026-05-23 — slice 3 (process-template placement via vite ?raw)

SPA-only. Architect routed delivery shape on
2026-05-23: vite `?raw` ships the templates with
the SPA build; no chan-server endpoint, no
network round-trip on bootstrap. Builds on
`-a-81`'s parameterised templates +
`substituteTeamTemplate` helper.

### Shape applied

**Vite config**

* `server.fs.allow: [".", ".."]` lets the
  parent-dir traversal resolve when importing
  `../../../docs/templates/team-process/
  bootstrap.md.tpl?raw`. Default
  `server.fs.strict: true` would otherwise
  block.

**Type declarations**

* `raw.d.ts`: declared `*.tpl?raw` + `*.md?raw`
  modules so the new bundled imports
  type-check.

**Orchestrator additions**

* `templateVarsForWire(wire) → TeamTemplateVars`
  — derives `hostHandle` (from `wire.host_handle`),
  `leadHandle` (from the member flagged
  `is_lead`, falling back to `host_handle`),
  `workerHandles` (non-lead members in declared
  order), `teamName` (from `wire.team_name`).
  Phase-slug omitted — chan-drive's TeamConfig
  doesn't persist one today; `teamTemplate.ts`'s
  default of `phase-1` is the new-team
  baseline.
* `placeTeamTemplates(wire)` — substitutes the
  bundled `bootstrapTemplate` via
  `substituteTeamTemplate` + writes to
  `Drafts/team-{name}/docs/bootstrap.md` via
  `api.create`. The docs/ subdir already exists
  (`Drive::create_team` materializes
  team-{name}/{config.toml, events/, docs/} in
  step 1).
* `runTeamBootstrap` chain reshape: steps
  renumbered (1: teamCreate, 2:
  placeTeamTemplates, 3: teamLoad, 4: spawn
  worker terminals, 5: lead identity prompt).
  Template-placement failures don't bail the
  chain — caught + reported via notify so the
  watcher load + worker spawn still bring up
  a working team.

### Why placement before watcher load

The team watcher polls `team-{name}/events/`
not `docs/`, so step ordering between
placement + load doesn't materially affect the
watcher. The architect's task body lists
template placement as step 2 (right after
config persistence) per readability — agents
read the bootstrap doc on their first
read-on-spawn, so having the file in place
before the watcher fires keeps the user-flow
sequencing intuitive.

### Files touched

* `web/vite.config.ts`
  * `server.fs.allow: [".", ".."]`.
* `web/src/raw.d.ts`
  * `*.tpl?raw` + `*.md?raw` module
    declarations.
* `web/src/state/teamOrchestrator.svelte.ts`
  * `bootstrapTemplate` raw import.
  * `substituteTeamTemplate` /
    `TeamTemplateVars` import from
    `./teamTemplate`.
  * `templateVarsForWire` + `placeTeamTemplates`
    exports.
  * `runTeamBootstrap` step 2 insert + step
    renumbering.
* `web/src/components/teamBootstrapOrchestrator.test.ts`
  * Chain-walk pin updated to include the new
    placement step between teamCreate and
    teamLoad.
* `web/src/state/teamLeadPrompt.test.ts`
  * Step-number pin renumbered from 3/4 to
    4/5.
* `web/src/state/teamTemplatePlacement.test.ts`
  (new) — 8 architectural pins for the
  bundle import + vite fs.allow + the two
  new helpers + the chain wiring + the
  defensive error handling.

### Decisions

* **vite ?raw not chan-server endpoint** —
  architect's routed shape. Trade-off: SPA
  rebuild needed when templates change.
  Acceptable for v0.12.0.
* **Non-fatal placement failure** — the watcher
  load + worker spawn still produce a working
  team if api.create fails (e.g. permission
  hiccup). Notify the user so they can re-run
  manually if needed.
* **No phase-slug surfacing** — chan-drive
  doesn't persist one; new teams default to
  `phase-1` via `teamTemplate.ts`'s helper.
  Chan-internal substitution (`phase-8`) lives
  in `CHAN_INTERNAL_TEAM_VARS`, not this
  orchestrator's path.

### Remaining deferred (slice 4+)

* Lead pre-flight survey trigger.
* Split-pane real estate (paneSplit + per-
  cell assignment).
* `dispatch_agent_event`-driven identity
  prompts (closes @@WebtestA's seedInput-
  visibility note).

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1314 / 1314** (+9 from `-a-80`
  slice 2's 1305; 8 new pins + the 2 fixed
  pins for step renumbering, net +9 due to
  one slice-2 pin overlap).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy --all-targets
  -- -D warnings` → clean (no Rust delta).

### Suggested commit subject

```
Team orchestrator slice 3: process-template placement via vite ?raw (fullstack-a-79 slice 3)
```

### Files (per-path)

* `web/vite.config.ts`
* `web/src/raw.d.ts`
* `web/src/state/teamOrchestrator.svelte.ts`
* `web/src/components/teamBootstrapOrchestrator.test.ts`
* `web/src/state/teamLeadPrompt.test.ts`
* `web/src/state/teamTemplatePlacement.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-79.md`

Autonomous-commit mode. No clearance held.

## 2026-05-23 — slice 4 (split-pane real estate wired)

SPA-only. Closes the slice-1 scope-poke
("split-pane real estate not yet wired —
falling back to tabs"). The dialog's
airplane-grid + drag&drop slot assignment from
`-a-78` slice 2 is now respected end-to-end.

### Shape applied

**New `buildSplitGrid(startPaneId, rows, cols)`
helper in `tabs.svelte.ts`**

* Materialises an R×C grid of panes starting
  from a given pane id.
* Strategy:
  1. Build a top row of `cols` panes by
     splitting horizontally (`direction:
     "row"`) from the starting pane
     `cols - 1` times.
  2. For each of the `cols` column-heads,
     split vertically (`direction: "column"`)
     `rows - 1` times to populate the rest.
* Returns pane IDs in **row-major** order
  matching the dialog's `slots[i]` cell
  numbering.
* 1×1 short-circuits — no splits, returns
  `[startPaneId]`.
* Side effect: `layout.activePaneId` ends on
  the bottom-right pane after construction;
  callers restore focus afterwards.

**New `resolveMemberPaneIds(config)` in
`teamOrchestrator.svelte.ts`**

* Returns `{ lead, workers: (string |
  undefined)[] }` — one pane id per member,
  plus the lead's pane separately.
* `tabs` mode: every member → starting pane
  (where the user's rich-prompt terminal
  lives = lead's pane).
* `split` mode: walks `realEstate.slots[]`
  (row-major; one entry per cell, each
  entry the list of member-indexes assigned
  to that cell), inverts to per-member pane
  assignment.
* Gaps + invalid slot entries fall back to
  cells[0] (= starting pane) so no member
  is silently dropped.
* The lead's pane is ALWAYS the starting
  pane per addendum-b clarification #1 —
  even if the user assigned the lead to a
  different cell on the dialog, the lead's
  terminal IS the host session and isn't
  moved. Slice 5 could add a `moveTab` step
  if lead-relocation becomes a real
  workflow.

**Orchestrator chain extension**

* New step 4 (between teamLoad + spawn loop):
  call `resolveMemberPaneIds(config)`. For
  split mode this triggers `buildSplitGrid`
  side effects (the layout splits before the
  spawn loop runs).
* Spawn loop: indexed walk (`for (let i = 0;
  …)`) instead of `for (const m of …)` so
  each member can look up its assigned pane.
  `openTerminalInActivePane` swap to
  `openTerminalInPane(paneId, …)`.
* After the spawn loop: `setActivePane
  (leadPaneId)` restores focus to the
  lead's pane (otherwise the bottom-right
  grid pane would be active from the last
  split / spawn).
* The slice-1 scope-poke notify ("Split-pane
  real estate not yet wired") is gone.

### Files touched

* `web/src/state/tabs.svelte.ts`
  * `buildSplitGrid` exported.
* `web/src/state/teamOrchestrator.svelte.ts`
  * Imports: added `buildSplitGrid`, `layout`,
    `openTerminalInPane`, `setActivePane`.
    Removed `openTerminalInActivePane`.
  * `resolveMemberPaneIds` exported.
  * `runTeamBootstrap` step 4 added (real-
    estate materialisation); spawn loop
    indexed-walk + per-pane open; focus
    restore after spawn. Step renumber: lead
    prompt is now step 6.
* `web/src/state/teamSplitPaneRealEstate.test.ts`
  (new) — 14 architectural pins for
  `buildSplitGrid` shape, `resolveMemberPaneIds`
  tabs/split branches, orchestrator imports +
  wiring + the gone scope-poke.
* `web/src/components/teamBootstrapOrchestrator.test.ts`
  * "spawn loop walks each member" pin
    updated to the new indexed-walk shape.
  * "opens in active pane" pin updated to
    "opens in the resolved pane".
  * "split-pane scope-poked" pin flipped to
    assert the scope-poke is GONE +
    `resolveMemberPaneIds` is wired.
  * New "focus restored to lead's pane" pin.
* `web/src/state/teamLeadPrompt.test.ts`
  * Step renumber 5 → 6.
  * Imports pin loosened (orchestrator now
    imports more helpers from tabs.svelte).

### Decisions

* **Lead pane immutable** — addendum-b
  clarification #1's "lead = host session"
  framing means the lead's terminal can't be
  moved by the orchestrator. If the user
  drags the lead to a different cell on the
  dialog, the workers see the cells as drawn
  but the lead stays at cells[0]. Slice 5
  could add `moveTab` if the workflow
  surfaces. Documented inline.
* **Row-major slots** match
  `emptySlotsForGrid(grid).length === rows *
  cols`. Cells are numbered left-to-right,
  top-to-bottom. The buildSplitGrid result
  is also row-major so indexes line up
  without translation.
* **Fallback to cells[0]** on gap /
  out-of-range slot entries instead of
  throwing. Cleaner UX: if the user leaves a
  member unassigned, the orchestrator drops
  them next to the lead rather than failing
  the whole bootstrap.

### Remaining deferred (slice 5+)

* Lead pre-flight survey trigger.
* `dispatch_agent_event`-driven identity
  prompts (closes @@WebtestA's seedInput-
  visibility note; needs the team event
  channel consumer in chan-server).
* `moveTab` for lead-relocation when the
  user assigns the lead to a non-starting
  cell.

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1336 / 1336** (+15 net from
  `-a-95`'s 1321; 14 new pins + 1 from the
  flipped scope-poke pin).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy
  --all-targets -- -D warnings` → clean
  (no Rust delta).

### Suggested commit subject

```
Team orchestrator slice 4: split-pane real estate (fullstack-a-79 slice 4)
```

### Files (per-path)

* `web/src/state/tabs.svelte.ts`
* `web/src/state/teamOrchestrator.svelte.ts`
* `web/src/state/teamSplitPaneRealEstate.test.ts` (new)
* `web/src/components/teamBootstrapOrchestrator.test.ts`
* `web/src/state/teamLeadPrompt.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-79.md`

Autonomous-commit mode. No clearance held.

## 2026-05-23 — slice 5 (lead-terminal rename + PTY restart) + round close

SPA-only. Closes the round per @@Alex's
teardown poke (`08180b2`). The architect added
step 7 (lead rename + restart) into my working
tree mid-flight; this slice finalises it with
matching test pins + the import fix
(`TeamMemberDraft` re-export wasn't valid;
moved to `./teamDialog.svelte`).

### Shape applied

**Architect-side step 7 addition**

The host's rich-prompt terminal IS the lead's
terminal (addendum-b clarification #1), but
its `CHAN_TAB_NAME` env-var was whatever the
user spawned the terminal with (e.g. some
default name, not the lead's handle). The
identity prompt staged in step 6 references
`$CHAN_TAB_NAME` literally — the lead's shell
needs the new env BEFORE the user submits.

Step 7 wiring:
1. `findTerminalBySession(hostSessionId)` →
   leadTab.
2. `wire.members.find(is_lead)` → leadHandle.
3. `renameTerminalTab(leadTab, leadHandle)`
   updates the in-SPA tab title +
   `tab.terminalEnvTabName` so the env-stale
   prompt clears.
4. `api.restartTerminal(sessionId, { name:
   leadHandle, window_id: sessionWindowId() })`
   bounces the PTY with the new
   `CHAN_TAB_NAME` env.
5. `markTerminalEnvNameRestarted(leadTab)`
   confirms the env refresh; step succeeds.
6. Restart failure is non-fatal — surfaces via
   notify, does not bail the chain.

**Test pin support**

* New `teamLeadRestart.test.ts` (8 pins):
  step ordering, gating chain, rename-before-
  restart, restartTerminal payload, success +
  failure paths, imports.
* `teamLeadPrompt.test.ts` "notify success"
  pin loosened — step 7 now lives between the
  lead-prompt step and the success notify;
  the regex tolerates the extra block.
* `teamSplitPaneRealEstate.test.ts` import
  fix: `TeamMemberDraft` re-imported from
  `./teamDialog.svelte` directly (orchestrator
  doesn't re-export it; my slice-4 test
  imported from the wrong module).

### Files touched

* `web/src/state/teamOrchestrator.svelte.ts`
  * Architect's step 7 addition (in tree
    pre-commit).
* `web/src/state/teamLeadRestart.test.ts`
  (new) — 8 architectural pins.
* `web/src/state/teamLeadPrompt.test.ts`
  * "notify success" pin loosened.
* `web/src/state/teamSplitPaneRealEstate.test.ts`
  * `TeamMemberDraft` import path fix.

### Round close

Per @@Alex's `08180b2` ("close round; transfer
any in-flight @@FullStackA/B + @@Systacean +
@@WebtestB work to @@WebtestA"):

* All my in-flight slices acked or shipped.
* Step 7 inherited from the architect's mid-
  flight addition; tested + committed cleanly.
* No outstanding lane work; the round ends
  with @@WebtestA + @@Architect + @@CI per
  the teardown.

### Remaining deferred (lifts to Round 3+)

* Lead pre-flight survey trigger.
* `dispatch_agent_event`-driven identity
  prompts (closes @@WebtestA's seedInput-
  visibility note).
* `moveTab` for lead-relocation when the user
  assigns the lead to a non-starting cell.
* Jitter (chan-server preferences + delay
  layer).

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1344 / 1344** (+8 new pins;
  intermittent flake on 1 pre-existing
  terminal-renderer test, unrelated).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy --all-targets
  -- -D warnings` → clean (no Rust delta).

### Suggested commit subject

```
Team orchestrator slice 5: lead rename + PTY restart (fullstack-a-79 slice 5; round close)
```

### Files (per-path)

* `web/src/state/teamOrchestrator.svelte.ts`
* `web/src/state/teamLeadRestart.test.ts` (new)
* `web/src/state/teamLeadPrompt.test.ts`
* `web/src/state/teamSplitPaneRealEstate.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-79.md`

Autonomous-commit mode. Round closes.
