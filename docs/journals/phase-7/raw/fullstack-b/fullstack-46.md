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

## 2026-05-19 13:00 BST — landed (@@FullStackB)

Spelling sweep audit: ran `grep -rEn '>([^<]*\b(color|
colors|customize|behavior|organize|analyze|recognize|
favorite|gray)\b[^<]*)<'` plus `aria-label="` / `title="`
/ `placeholder="` variants across `web/src/**/*.svelte`
and `crates/**/*.rs`. The codebase already speaks
British in most places — the only literal American
spelling in a user-facing string was the pane hamburger
"Focus border color" label. All other matches landed on
CSS property names (`background-color`, `border-color`,
…), code comments, or JSDoc — out of scope per the
task ("CSS property names stay American; code comments
are not user-facing").

Files:

* `web/src/state/shortcuts.ts` — new `app.pane.mode`
  shortcut entry (`Mod+K`) so the cheatsheet table
  and the hamburger chord column resolve through the
  same SHORTCUTS registry.
* `web/src/components/Pane.svelte`:
  * Imports `enterPaneMode` from tabs state +
    `LayoutGrid` lucide icon.
  * New `onEnterPaneMode` handler.
  * Hamburger menu gains the "Enter Pane Mode" entry
    at the top with the chord hint, followed by a
    separator. Final order matches the task spec:
    ```
    Enter Pane Mode  (Cmd+K)
    ─────
    Focus border colour (blue / green / pink)
    ─────
    Next pane / Previous pane
    ─────
    Split right / Split down / Close all tabs /
    Close pane
    ```
  * `Focus border color` → `Focus border colour`.
* `web/src/components/Pane.test.ts` — assertion list
  prepended with `"Enter Pane Mode"`; legacy label
  literal updated to `"Focus border colour"`.

Out-of-scope clarification: "Next pane / Previous pane"
hamburger entries already render without a chord hint
(via `chordLabel("app.pane.next/prev")` returning empty
string since those ids aren't in the SHORTCUTS registry
on `origin/main`). The task note about losing the chord
hint after `fullstack-42` is already the state; no
change needed here.

Verification:

* `npm run check` → 0 errors / 0 warnings.
* `npx vitest run Pane EmptyPaneCarousel` → 9 / 9
  pass.
* Full vitest run interleaves with @@FullStackA's
  in-flight `fullstack-42` workdir (App.svelte
  contamination), so a `paneModeKeymap.test.ts`
  assertion fails locally; it will pass once
  @@FullStackA commits + the rebase lands. My commit
  scope is only `Pane.svelte`, `Pane.test.ts`, and
  `shortcuts.ts` — none of those touch App.svelte.

Commit message proposed:
`British spelling sweep + Enter Pane Mode hamburger entry (fullstack-46)`.
