# fullstack-a-78 — Rich Prompt "New Team" button + dialog (airplane-grid + drag&drop)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: addendum-b wave-1

## Goal

Repurpose the Rich Prompt's "create watcher" button
to open the New Team dialog. Build the dialog UX
per [`../alex/addendum-b.md`](../alex/addendum-b.md).

## Reference

[`../alex/addendum-b.md`](../alex/addendum-b.md)
§"The Team Feature" / §"New Team" + clarifications
#3 (size semantics), #8 (CHAN_TAB_NAME auto-populate),
#9 (airplane-grid).

## Scope

### Repurpose the button

Today the Rich Prompt's button creates a watcher.
Now it opens the New Team dialog. (Loading existing
teams covered by `-a-80`.)

### Dialog inputs

* **Your name** — host's name. With `@@` auto-prefix
  toggle ON, the host handle becomes `@@<name>`.
* **Team name** — used in config + dir name.
* **Size** — agent count (lead + N workers). Starts
  at 2 (lead + 1 worker). Max 16.
  - User is NOT counted; user sits in the
    rich-prompt terminal hosting the lead.
* **Auto-prefix `@@` checkbox** — when ON, prepend
  `@@` to all member names (including host).
* **Per-member rows** (one per agent in the team):
  - `[robot-icon] [name input] [command + flags] [env k=v ...] [is_lead radio]`
  - One row MUST be marked as lead.
* **Real estate selector**:
  - (a) **Tabs in current Hybrid** — all terminals
    spawn as tabs in the current pane.
  - (b) **Split pane** — open an airplane-style grid
    picker for the user to drag&drop robots into
    slots.

### Airplane-grid picker (option b)

* Visual grid of available split shapes for the
  chosen team size (e.g. 4 agents → 1x4 / 2x2; 6 →
  2x3 / 3x2 / 1x6).
* Each shape shows empty cells.
* User drags robot-icons from the member rows into
  cells.
* **Dropping multiple robots on the same cell** =
  those robots become tabs in the same pane.
* For team sizes that don't fit common grid shapes
  (5, 7, 11, 13): fall back to 1xN OR show the
  nearest grid with empty cells.

### CHAN_TAB_NAME auto-populate

Per clarification #8: the `env` for each member is
auto-populated with `CHAN_TAB_NAME=<name input value>`.
User CAN add additional env vars in the env input,
but CHAN_TAB_NAME stays chan-controlled.

### Bootstrap button

Final "Bootstrap" button collects all inputs +
fires the bootstrap orchestrator (`-a-79`). This
task's scope ends at "click Bootstrap → hand off
to orchestrator"; the actual bootstrap lives in
`-a-79`.

## Acceptance

1. Rich Prompt button opens New Team dialog (not
   the old watcher dialog).
2. All inputs render + validate (size 2-16; team
   name unique among existing teams; lead exactly
   one).
3. Auto-prefix toggle updates displayed handles
   in real time.
4. Airplane-grid renders shapes for the chosen
   size; drag&drop works; multi-robot on same
   slot = tabs.
5. Bootstrap button hands off to `-a-79`'s
   orchestrator entry point.

### Tests

Vitest pins for dialog rendering + input validation
+ auto-prefix behavior + grid drag&drop + handoff
call.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only for this task.
* Consumed by `-a-79` (bootstrap orchestrator).
* Load-team flow (`-a-80`) reuses the dialog shape;
  factor the dialog component so `-a-80` can
  populate it with existing config.

## Authorization

Yes for SPA Rich Prompt + new TeamDialog component
+ tests + task tail + outbound.

## Numbering

This is `-a-78`.

## Out of scope

* Bootstrap orchestration (`-a-79`).
* Load existing team (`-a-80`).
* Process template generalisation (`-a-81`).
* chan-drive config schema (`systacean-30`).
* Multi-team watcher (`systacean-31`).

## 2026-05-22 — slice 1 (dialog shell + button repurpose) ready for review

Per architect's slice-friendly framing,
splitting `-a-78` into:
* **Slice 1 (this commit)**: dialog shell with
  inputs + button repurpose + state singleton.
* **Slice 2**: airplane-grid + drag&drop for
  the `split` real-estate option.

Six-file change. SPA-only.

### What landed

`web/src/state/teamDialog.svelte.ts` (new):

* `TeamDialogRequest` + `TeamDialogConfig` +
  `TeamMemberDraft` + `TeamRealEstate` types.
* `teamDialogState` singleton +
  `openTeamDialog` / `closeTeamDialog` helpers
  (mirrors the `spawnDialog` pattern from
  `-a-4`).
* `defaultTeamConfig()` returns a 2-member
  config (lead + 1 worker; auto-prefix on;
  real estate = tabs).
* `validateTeamConfig(cfg, existingNames)`:
  validates host/team name non-empty, size
  in [2, 16], exactly one lead, every member
  has a name, team name not already taken.
* `resizeTeamMembers(cfg)`: grow appends
  fresh `WorkerN`; shrink truncates from end;
  preserves the lead (defaults to slot 0 if
  the prior lead got truncated).
* `TEAM_MIN_SIZE` (2) + `TEAM_MAX_SIZE` (16)
  per addendum-b clarification #3.

`web/src/components/TeamDialog.svelte` (new):

* Inputs: host name, team name, auto-prefix
  checkbox, size slider, per-member rows
  (icon + name + command + env + lead radio).
* Renders `@@<name>` previews live via
  `handleOf()` when auto-prefix is on.
* Bootstrap button gates on `validateTeamConfig`
  returning null; surfaces issue messages
  inline.
* Cancel / X / Esc / backdrop-click all close
  the dialog.
* Real-estate selector renders a placeholder
  pointing at slice 2.
* CSS scoped to the dialog; styled to match
  the SpawnDialog visual language.

`web/src/App.svelte`:
* Imports `TeamDialog` + `teamDialogState`.
* Mounts `<TeamDialog request={...} />` under
  `{#if teamDialogState.request}` at App root
  (same stacking-context discipline as
  SpawnDialog).

`web/src/components/TerminalRichPrompt.svelte`:
* New `openNewTeamDialog()` helper. Repurposes
  the icon-btn that previously called
  `watchDirectory`.
* Icon-btn aria-label / title / onclick now
  point at the New Team flow.
* The dropdown "Watch directory" menu entry
  stays for now (legacy attach-watcher);
  slice 2 may collapse it.

`web/src/state/teamDialog.test.ts` (new): 14
pins covering defaultConfig shape,
validateTeamConfig (host/team name / size /
lead / member name / duplicate), resize grow /
shrink / shrink-past-lead, open/close bus.

`web/src/components/newTeamButton.test.ts`
(new): 12 pins covering App root mount, Rich
Prompt button repurpose, TeamDialog component
shell shape, Escape-to-close wiring, slice 2
placeholder presence.

`web/src/components/TerminalRichPrompt.test.ts`:
existing "Watch directory uses the path
prompt" test rewritten for the new flow —
asserts the New Team icon-btn opens the
global team dialog with the correct
`hostSessionId`.

### Acceptance (slice 1)

1. **Rich Prompt button opens New Team dialog**
   ✓ — mechanism via tests; @@WebtestA walk
   for empirical.
2. **All inputs render + validate** (size
   2-16, team name uniqueness deferred to
   slice 2's load-team integration, lead
   exactly one) ✓.
3. **Auto-prefix toggle updates handles**
   live ✓ via `handleOf()` in the host-name
   hint row.
4. **Airplane-grid renders shapes** —
   DEFERRED to slice 2 (placeholder rendered
   pointing at the slice).
5. **Bootstrap button hands off to `-a-79`**
   ✓ via the `request.onBootstrap` callback.
   Stub logs the config; orchestrator wires
   actual spawn in `-a-79`.

### Gate

* vitest **879 / 879** (+24 net from `-a-82`'s
  855).
* svelte-check 0 errors / 0 warnings across
  4018 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Per-slice split** matches `-a-67` /
  `-a-66` / `-a-81` precedent. Substantial
  dialog UX warrants splitting.
* **State singleton pattern** mirrors
  `spawnDialog.svelte.ts` (-a-4). Dialog
  mounted at App root → free of every parent
  stacking context.
* **Bootstrap stub** logs the config until
  `-a-79` ships the orchestrator. The dialog
  closes on Bootstrap regardless so the user
  sees the click-through.
* **Dropdown "Watch directory" stays** —
  slice 2 may collapse. The icon-btn is the
  load-bearing repurposed entry per the
  addendum's framing.
* **Auto-prefix off doesn't strip existing
  `@@`** — if the user types `@@Alex`
  manually + toggles off, the rendered
  handle stays `@@Alex` (rendered as-is).
  The handle preview row makes this
  transparent.

### Suggested commit subject

```
Rich Prompt: repurpose watcher button → New Team dialog shell (fullstack-a-78 slice 1)
```

Single commit. State + dialog + button repurpose
+ tests tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/state/teamDialog.svelte.ts` (new)
* `web/src/state/teamDialog.test.ts` (new)
* `web/src/components/TeamDialog.svelte` (new)
* `web/src/components/newTeamButton.test.ts` (new)
* `web/src/App.svelte`
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/TerminalRichPrompt.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-78.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
