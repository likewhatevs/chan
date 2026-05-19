# fullstack-72: Hybrid NAV spawn keys (1/2/3) use draft/commit model

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged a UX inconsistency in Hybrid NAV:

* **Tab** (flip Hybrid) uses a draft/commit
  model — pressing Tab shows a preview;
  Enter commits the flip; Esc discards.
  Verified in `webtest-a-12` item 9.
* **1/2/3 spawn keys** commit immediately on
  the number key — no preview window, Pane
  Mode exits on the keystroke. Inconsistent
  with Tab's pattern.

The Pane Mode pill at the bottom already
reads "Enter commit · Esc discard" —
implying a draft/commit model for every
action. The pill's promise is honest for Tab
but lies for the spawn keys.

Align spawn keys with Tab's draft/commit
pattern so the pill's wording matches every
keypress in Pane Mode.

## Spec

Spawn keys to update:

* **1** — spawn terminal in focused pane.
* **2** — spawn file-browser in focused pane.
* **3** — spawn graph in focused pane.
* **4** — if it exists per the current keymap
  (check `paneModeKeymap.ts`), same treatment.

New behavior per key:

1. User presses Cmd+K (enters Pane Mode,
   sees H-for-help flash + pill).
2. User presses `1` (or `2`, `3`, `4`).
3. Pane Mode stays open. A preview surfaces
   showing what's about to happen (e.g.
   "About to spawn terminal in focused pane"
   or a ghost-rendering of the tab that's
   about to appear).
4. User presses Enter → action commits (spawn
   happens), Pane Mode exits.
5. User presses Esc → action discards,
   Pane Mode exits without spawning.
6. Pressing a different staging key (e.g.
   `2` after `1`) replaces the draft —
   doesn't commit-then-restage.

This matches Tab's pattern, which the user
already knows.

## Out-of-scope keys

These stay immediate-commit per their current
semantics:

* WASD splits — split lands immediately, Pane
  Mode stays open. The split is so
  reversible (just close the pane) that
  staging adds friction.
* Arrow focus-move — same: focus-move is
  zero-cost to undo, staging is friction.
* `<` / `>` dock toggles — also reversible,
  immediate is right.
* `Q` close pane — debatable but currently
  immediate; keep that for now. Could be
  added in a follow-up if you want symmetry,
  but @@Alex's ask is specifically 1/2/3.
* `p` rich prompt — already a multi-step
  interaction (prompt opens, user types,
  etc.); double-staging would be weird.
* `h` help — toggle, immediate.

Tab already uses draft/commit; this task
brings the spawn keys into line with that.

## Relevant code

* `web/src/App.svelte` —
  `handlePaneModeKey()` dispatch. Currently
  has direct calls like
  `paneModeOpenTerminal(...)` for the spawn
  keys; these need to flow through the
  draft state instead.
* `web/src/state/paneMode.svelte.ts` (or
  wherever the `paneMode` state lives) —
  draft model. Tab flip uses it; check the
  shape and extend the draft to carry the
  spawn intent.
* `web/src/components/Pane.svelte` — preview
  surface for the draft. Tab uses a preview
  pane render; spawn keys may want
  something similar (e.g. a ghosted tab in
  the tab strip showing the upcoming kind).
* `web/src/components/PaneModeHelp.svelte` —
  help cheatsheet wording. Update any
  per-key descriptions that say
  "Spawn X tab" to indicate the draft
  semantics (e.g. "Draft: spawn X tab").

## Acceptance criteria

* Pressing Cmd+K then `1`: spawn terminal
  intent staged. Pane Mode stays open with a
  preview indicator. Enter commits (terminal
  spawns); Esc dismisses (no spawn).
* Same for `2` (file-browser) and `3`
  (graph). And `4` if it exists.
* Pressing `1` then `2` replaces the draft
  (don't commit-then-restage; only the most
  recent key's draft is live).
* The pane mode pill's "Enter commit · Esc
  discard" wording is now honest for every
  action (Tab + spawn keys + future
  additions).
* WASD / arrows / `<` / `>` / `Q` / `p` /
  `h` semantics unchanged.
* Help overlay (`-63` clickable buttons)
  still works — clicking the `1` cap in the
  cheatsheet stages the draft, same as
  pressing the key.

### Tests

* Vitest: pressing `1` while in Pane Mode
  sets `paneMode.draft` to a "spawn
  terminal" intent without firing
  `paneModeOpenTerminal()`.
* Enter while a spawn draft is active fires
  the corresponding spawn function.
* Esc while a spawn draft is active clears
  the draft without firing the spawn.
* Replacing draft: `1` then `2` results in
  the file-browser draft alone (no terminal
  spawned).
* Clicking the `1` cap in the help overlay
  produces the same state as the keystroke
  (clickable-help parity from `-63` stays).

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0-blocking-soft. Strong UX
  consistency win, but the spawn keys
  function today (just inconsistently with
  Tab). If this slips, can ship as v0.11.1.
* Preview surface for spawn drafts: your
  call on the visual. Tab uses a full pane
  preview because the back-side content is
  what's about to render. Spawn keys are
  spawning a new tab — preview could be a
  ghosted entry in the tab strip + a
  small chip on the pill. Match the visual
  weight of Tab's preview so users see one
  consistent "drafting" affordance.
* Coordinate with `-61` (H-for-help flash):
  the flash still shows on every Pane Mode
  entry; no conflict with the new draft
  state.
* Queue position: end of Lane A queue,
  after `-70`. Updated queue: `-70` → `-72`.
* Standing topic-level commit clearance.

## 2026-05-19 18:34 BST — @@FullStackA implementation note

Implementation:

* `tabs.svelte.ts`: extended `paneMode` state with
  `spawnIntent: PaneModeSpawnIntent | null`.
  New exports: `paneModeStageSpawn(kind, ctx)`,
  `clearPaneModeSpawnIntent()`,
  `PaneModeSpawnKind`, `PaneModeSpawnIntent`.
  `enterPaneMode` / `cancelPaneMode` clear the
  intent on enter / cancel. `commitPaneMode`
  reads the intent BEFORE merging the draft;
  routes to `paneModeOpenTerminal/Browser/Graph`
  so the spawned tab lands inside the draft as
  part of the same transaction.
* `App.svelte`: `case "1"` / `"2"` / `"3"`
  switched from direct `paneModeOpenX(ctx)`
  calls to `paneModeStageSpawn(kind, ctx)`. No
  draft modification on the number key. The
  `Enter` case peeks `paneMode.spawnIntent`
  before commitPaneMode and, when the staged
  kind is `browser`, primes
  `revealAndSelect(ctx.file ?? ctx.dir)` so the
  new tab's tree lands expanded to the
  contextual node (preserves the
  `fullstack-43` prime behavior).
* `AppStatusBar.svelte`: pane-mode pill grows a
  `→ stage <kind>` muted span when an intent
  is staged. Cleared automatically on commit /
  cancel via the spawnIntent's lifecycle.
* `PaneModeHelp.svelte`: 1 / 2 / 3 rows
  reworded from `Terminal` / `File Browser` /
  `Graph` to `Stage: Terminal` / etc. so the
  cheatsheet matches the draft/commit
  semantics.

Out-of-scope keys (WASD splits, arrows, `<` /
`>`, `Q` / `K`, `p`, `h`) keep their existing
immediate-commit semantics per the task spec.

Behavior:

* `1` → `paneMode.spawnIntent = { kind:
  "terminal", ctx }`. Draft unchanged.
* `1` then `2` → intent replaced
  (`{ kind: "browser", ctx }`). Terminal
  doesn't fire.
* Enter → spawn applies into the draft;
  draft merges into layout. Intent cleared.
* Esc → intent + draft cleared, nothing
  spawned.
* `4` (new file), `s` (Search), `p` (rich
  prompt), `x` / `k` (close) — these still
  call commitPaneMode; if an intent is
  staged it fires as part of the commit
  (matches Tab's pattern + the pill's
  promise).

Tests added in `tabs.test.ts`
(`pane state` describe):

* Stage stores intent without touching the
  draft.
* Commit applies the staged spawn to the
  focused pane with the resolved context
  (scopeId / pendingSelectId for graph;
  parent dir / file for browser / terminal).
* Pressing two staging keys replaces the
  intent — only the second kind fires.
* Esc / cancel discards the staged intent
  without spawning.
* Staging outside Pane Mode is a no-op.

Tests updated in `paneModeKeymap.test.ts`:

* Existing `1 spawns Terminal` and
  `2 primes …` assertions reframed as
  staging asserts (`paneModeStageSpawn(
  "terminal", ...)` / `..."browser"...`).
* New assert on the Enter handler's peek-
  and-prime sequence for browser intents.

Gate green:

* `npm run test -- tabs paneModeKeymap` (105
  passed),
* `npm run test` (399 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: Cmd+K then `1`
— pill shows `→ stage terminal` chip, no
new tab in the strip yet. Press `2` — chip
flips to `→ stage browser`. Press Esc —
both intents drop, no tab spawned. Re-enter
Pane Mode, stage `1`, press Enter — terminal
tab appears in the focused pane.

Proposed commit message:

> Stage spawn keys instead of immediate commit (fullstack-72)
>
> Cmd+K 1/2/3 now stage a `paneMode.spawnIntent`
> instead of pushing a tab directly into the
> draft on keystroke. `commitPaneMode()` applies
> the staged intent into the draft before sealing
> the layout, so the pill's "Enter commit · Esc
> discard" promise reads honest for every keypress
> in Hybrid NAV — matches Tab's draft/commit
> pattern. Replacing the intent (`1` then `2`)
> only fires the most recent kind on commit. The
> pane-mode pill grows a `→ stage <kind>` chip
> when an intent is staged; PaneModeHelp's
> spawn rows now read `Stage: Terminal` / etc.
> The Enter handler peeks the intent before
> commit so a browser spawn still primes
> `revealAndSelect(ctx)` like fullstack-43 did.
