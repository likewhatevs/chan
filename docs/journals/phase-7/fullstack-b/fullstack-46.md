# fullstack-46: British spelling sweep + pane hamburger gets "Enter Pane Mode" entry

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Goal

Two related cleanups:

1. **British spelling everywhere** — @@Alex lives in
   Britain; chan should reflect that. Replace
   American spellings with British across all
   user-facing strings.
2. **Pane hamburger menu adds an "Enter Pane Mode"
   item** so users discover Cmd+K through the menu.

## Relevant links

* @@Alex's chat note 2026-05-19 12:45 BST ("british
  spelling, this should be *all around Chan*, I
  live in britain!").
* Pane Mode = the name of the `Cmd+K` transactional
  mode (from `fullstack-16` / `ui-exploration.md`
  Phase 2).

## Acceptance criteria

### British spelling

* Audit all user-facing strings (`web/src/**/*.svelte`
  + `web/src/**/*.ts` UI label literals + any chan-
  server-emitted user-facing strings) for American
  spellings. Common ones:
  * `color` → `colour`
  * `customize` → `customise`
  * `behavior` → `behaviour`
  * `organize` → `organise`
  * `analyze` → `analyse`
  * `recognize` → `recognise`
  * Also `-er` → `-re` where applicable: `center`
    → `centre` (rare in UI strings but check).
* CSS property names (`background-color`, `color`)
  stay American — they're spec. **Do NOT touch CSS
  property names.**
* `data-*` attribute names ditto — code, not user-
  facing.
* JS/TS variable names stay American where they map
  to CSS / web APIs — `color`, `selectionColor`,
  etc.
* Variable names that ARE user-facing-string sources
  (e.g. a `colorPickerLabel = "Color"`) get the
  British form on the string side; the variable
  name itself can stay as-is.

### Pane hamburger menu

Pane hamburger menu (from `fullstack-30`'s order)
adds an item at the top: **"Enter Pane Mode (Cmd+K)"**.
Clicking it enters Pane Mode (same as pressing
`Cmd+K`).

New order:

```
Enter Pane Mode     (Cmd+K)
─────────────────────
Focus border colour (blue / green / pink)
─────────────────────
Next pane           (Cmd+])
Previous pane       (Cmd+[)
─────────────────────
Split right
Split down
Close all tabs
Close pane
```

Note: "Focus border color" → "Focus border **colour**"
as part of the spelling sweep.

Also note: after `fullstack-42`, `Cmd+]` / `Cmd+[`
shortcuts go away. The "Next pane / Previous pane"
menu entries lose the shortcut hint (they trigger
the same action; just no hotkey on the right).

## Out of scope

* Translations / i18n machinery (just British
  English across the existing strings).
* CSS / web API spellings (stay American).

## How to start

1. Grep `web/src/` for the offending words in
   user-facing string contexts (between `>` and `<`
   in Svelte, inside `"..."`/`'...'`/`\`...\`` for
   strings that are obviously labels).
2. Replace + scan again to catch any spillover.
3. Audit chan-server emitted messages too (e.g.
   error toasts surfaced from the API).
4. Add the "Enter Pane Mode" item to the hamburger
   menu in `Pane.svelte`.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-b-architect.md`.
