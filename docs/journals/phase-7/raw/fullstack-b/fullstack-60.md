# fullstack-60: trim pane hamburger after Focus border colour

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged the pane hamburger menu has
duplicate-of-Cmd+K entries past the colour
swatches. Cmd+K is canonical for those actions
(`fullstack-42` cleanup direction + `fullstack-52`
"New Terminal" drop, same pattern). The menu
should stop right after the PINK colour swatch.

## Current menu shape (from @@Alex's screenshot)

```
[icon] Enter Pane Mode        Cmd+K
[icon] Focus border colour
  ● blue ✓
  ● green
  ● pink
───────  (separator)
[icon] Next pane                        ← drop
[icon] Previous pane                    ← drop
───────  (separator)                    ← drop
[icon] Split right                      ← drop
[icon] Split down                       ← drop
[icon] Flip Hybrid            Cmd+K Tab ← drop
[icon] Close all tabs                   ← drop
[icon] Close pane                       ← drop
```

## Desired menu shape after trim

```
[icon] Enter Pane Mode        Cmd+K
[icon] Focus border colour
  ● blue ✓
  ● green
  ● pink
```

Nothing else. No separator after pink, no trailing
items.

## Relevant code

* `web/src/components/Pane.svelte` — the pane
  hamburger menu items. The walker's
  `webtest-b-6` item 5 verdict transcribed the
  current shape; find the section that renders
  the entries between Focus border colour and
  Close pane and drop them.
* Audit the imports for the dropped entries
  (icons, action handlers) — if any are only
  used by the removed rows, drop them too
  (same hygiene as `fullstack-52`'s drop of
  `TerminalIcon`, `openTerminalInPane`,
  `openNewTerminal`).
* The corresponding action paths (Next pane,
  Previous pane, Split right, Split down, Flip
  Hybrid, Close all tabs, Close pane) all
  stay reachable via Pane Mode keystrokes
  (`Cmd+K` + the appropriate binding). Don't
  touch the keystroke surface.

## Acceptance criteria

* Pane hamburger menu renders only:
  1. Enter Pane Mode (chord `Cmd+K`)
  2. Focus border colour header + the three
     colour swatches (blue / green / pink) with
     the existing checkmark for the active one.
* No separator after the pink swatch.
* No "Next pane" / "Previous pane" / "Split
  right" / "Split down" / "Flip Hybrid" /
  "Close all tabs" / "Close pane" entries
  anywhere in the pane hamburger.
* Pane Mode keystrokes for those actions
  (`Cmd+K + arrow keys` next-pane,
  `Cmd+K + W/A/S/D` split, `Cmd+K + Tab` flip,
  `Cmd+K + Q` close pane, etc.) unchanged and
  still wired.
* Other menus (terminal-tab hamburger,
  file-browser hamburger, etc.) untouched.

### Tests

* Update or add a snapshot/structure test
  asserting the pane hamburger contains exactly
  the two expected sections (Enter Pane Mode +
  Focus border colour) and nothing else.
* Negative assertions: rendered DOM does NOT
  contain "Next pane", "Split right", "Flip
  Hybrid", "Close pane" labels.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Small change; ship cleanly behind `-58` /
  `-59`. You're already in `Pane.svelte` for the
  per-Hybrid theme render work, so the
  context-switch is zero.
* No re-walk cost worth flagging — the
  walkthroughs are wrapping anyway, and the
  menu cleanup is visible by inspection. If
  @@Alex spot-checks the menu in the Tauri
  shell post-tag and it reads cleanly,
  that's the verdict.
* Standing topic-level commit clearance.
* Queue position: behind `-54`, `-58`, `-59`.

## 2026-05-19 19:55 BST — implementation

**Trim:** dropped 51 lines of JSX from the pane
hamburger (everything past the colour swatches):
* "Next pane" + "Previous pane" rows + the
  separator that joined them to the swatches and
  the one that followed.
* "Split right" + "Split down" rows (inside the
  `{#if splitsAllowed}` gate).
* "Flip Hybrid" row.
* "Close all tabs" row.
* "Close pane" row.

Plus the post-swatch separator that the spec
explicitly said should not survive ("no separator
after pink, no trailing items"). After the
trim, the menu reads exactly the desired shape:
```
[icon] Enter Pane Mode        Cmd+K
[icon] Focus border colour
  ● blue ✓
  ● green
  ● pink
```

**Hygiene sweep** (same pattern as
`fullstack-52`'s drop of `TerminalIcon` etc.):

* Dropped handlers no longer needed:
  `onSplitRight`, `onSplitDown`, `onCloseAllTabs`,
  `onClosePane`, `onFlipHybrid`, `doSelectNextPane`,
  `doSelectPrevPane`.
* Dropped `splitsAllowed` derived (its only
  consumer was the now-removed `{#if}` gate).
* Dropped imports that had no remaining consumer:
  * From `state/tabs.svelte`: `canSplit`,
    `closePane`, `closeTabsInPane`, `flipHybrid`,
    `selectNextPane`, `selectPrevPane`,
    `splitPane`.
  * From `lucide-svelte`: `ArrowDown`, `ArrowRight`,
    `FlipHorizontal2`, `ListX`,
    `SquareSplitHorizontal`, `X`.

All seven dropped state imports' actions stay
reachable via Pane Mode (`Cmd+K + arrow keys`
next-pane, `Cmd+K + W/A/S/D` split, `Cmd+K + Tab`
flip, `Cmd+K + Q` close pane, etc.). The keymap
layer in `App.svelte` dispatches `chan:command`
events that route to the underlying `tabs.svelte`
exports directly — not through `Pane.svelte`'s
wrappers. Verified by `svelte-check` (0 errors)
and the full test suite (379 passed).

**Test update + new sentinel:**

* `web/src/components/Pane.test.ts`:
  * Existing `hamburger uses window-wide focus
    color before navigation and split actions`
    test asserted the OLD 11-item menu shape.
    Flipped to the new 4-item shape (Enter Pane
    Mode + the three colour swatches).
  * New `pane hamburger no longer renders Cmd+K-
    canonical entries (fullstack-60)` test
    asserts the trim with negative assertions
    on each removed label (Next pane, Previous
    pane, Split right, Split down, Flip Hybrid,
    Close all tabs, Close pane). Matches the
    task's "Negative assertions" requirement.

**Gate.** `npm run check` 0 errors / 0 warnings;
`npm run test` 36 files / 379 tests passed
(was 378; +1 from my new sentinel); `npm run
build` clean; `scripts/pre-push` green.

**Visual eyeball.** Not required per task note
("No re-walk cost worth flagging"). The
component test renders the hamburger and asserts
the structure end-to-end; the wider gate covers
the rest.

**Out of scope:**

* Other menus (terminal-tab hamburger,
  file-browser hamburger, etc.) untouched per
  task note.
* Right-click context menu (`paneContextMenu`)
  contents untouched — that one carries the
  "Reload + toggle inspector" pane-level dev
  actions; not in the trim's spec.

**Commit readiness:**

Files staged:
* `web/src/components/Pane.svelte` (the trim +
  the hygiene sweep)
* `web/src/components/Pane.test.ts` (flipped
  existing assertion + new sentinel)
* `docs/journals/phase-7/fullstack-b/fullstack-60.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Trim pane hamburger to Enter Pane Mode + colour swatches (fullstack-60)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 17:20 BST cut.
