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
