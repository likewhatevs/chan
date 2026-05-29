# fullstack-59: wire per-Hybrid theme into render — finish Hybrid flip

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

`webtest-b-6` item 11 caught that `fullstack-48`
phase B shipped half of the per-Hybrid theme
feature: the model + serialization layer works
(`HybridSide.theme` lives on front + back,
serializes as `ht` / `hb` in the URL hash, lazy-
inits to `inverseTheme()` on first flip), but no
render consumer reads the per-side theme. Visible
theme always tracks the global `ui.themeChoice`.

Walker's verification:

| Step                                          | doc data-theme |
|-----------------------------------------------|----------------|
| Front (global=Dark)                           | dark           |
| Global → Light, on front                      | light          |
| Flip → back (had `hb:"l"` override = light)   | light          |
| Settings → Dark on back → global=Dark         | dark           |
| Flip → front (no `ht`; should use override)   | dark           |

The hash carries `ht` / `hb` correctly; the render
pipeline ignores them.

Diagnosis from the walker (carry forward as the
fix recipe):

> `grep -rE 'node\.theme|pane\.theme|hybrid.*theme'
> web/src --include='*.svelte' --include='*.ts'`
> turns up **only** the write sites in
> `tabs.svelte.ts:1995, 2007-2008`. No consumer
> reads `HybridSide.theme` to apply CSS /
> data-theme attribute / class.
>
> Probable follow-up: add a per-pane
> `data-theme={node.theme ?? ui.theme}` consumer
> in Pane.svelte (mirroring the existing
> `data-focus-color`), so `node.theme` actually
> drives the visible palette.

That's the right shape. Implement it.

## Relevant code

* `web/src/state/tabs.svelte.ts:1984` —
  `flipHybrid()` action. Already writes
  `node.theme` / `node.back.theme` correctly;
  don't touch.
* `web/src/state/tabs.svelte.ts:1995, 2007-2008`
  — the write sites the walker grepped. No
  read consumer today; that's the gap.
* `web/src/components/Pane.svelte` — the render
  surface. Existing pattern: `data-focus-color`
  is set per pane. Add a sibling
  `data-theme={node.theme ?? ui.theme}` (or
  whichever attribute the global theme switch
  uses today — find the global theme attribute
  in the document tree and mirror its scope
  down to the pane).
* `web/src/components/SettingsPanel.svelte` —
  Appearance toggle calls
  `setThemeChoice()` (global). After this cut,
  the toggle should write to the focused
  pane/side's `node.theme` slot, NOT global.
  OR: leave the global toggle alone and add a
  per-side toggle on the Hybrid chrome. UX call;
  see acceptance criteria below.
* CSS theme tokens — verify the theme cascade
  works at the pane level. Today the `:root`
  data-theme attribute drives the cascade; the
  per-pane override needs an equivalent
  scoping selector
  (`[data-pane-theme="dark"] { --bg: ...; }`).

## Acceptance criteria

* Rendering: when `node.theme` is set on the
  visible side, the pane's CSS palette matches
  the per-side theme, NOT the global theme.
  Walker's verification table flips to PASS:
  front theme stays front-theme after a flip
  back, even if global changed in between.
* Round-trip via hash: a URL with `ht:"l"` on
  the front side renders front in light theme
  even if `ui.themeChoice` is dark, and
  vice-versa.
* Settings → Appearance: pick one of:
  1. Repurpose the existing global toggle to
     write to the focused pane/side
     (`node.theme` for current side).
     Add a "Apply globally" sub-affordance for
     the old global behaviour if needed.
  2. Keep the global toggle global; add a
     small per-side theme toggle on the Hybrid
     chrome (next to the back-attention dot
     position).
  My recommendation: **(2)**. The global
  toggle stays as "default theme for new
  panes". Per-side override sits on the
  Hybrid chrome — visible and discoverable
  exactly where the feature lives.

  Confirm your call in the implementation note;
  this is a real UX fork.

* No regression on single-theme single-pane
  drives: if neither side has a theme override,
  rendering tracks the global toggle exactly
  as today.

### Tests

* Vitest: assert `node.theme` (when set)
  produces the corresponding `data-theme`
  attribute (or scope) on the pane, regardless
  of `ui.themeChoice`.
* Component test: mount a Hybrid with `front.theme = "light"`
  and `ui.themeChoice = "dark"`; assert the
  rendered pane uses light tokens.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0 blocking — Hybrid flip is the marquee
  feature; the per-side theme is half of what
  makes Hybrids feel like distinct sides. Without
  this, both sides are visually identical except
  for the tab strip / content.
* Re-walk after ship: Lane B's `webtest-b-6`
  item 11 should re-walk once you ship. If Lane
  B has wound down, Lane A can re-run.
* Queue position: behind `-54`, `-58` on your
  lane.
* Standing topic-level commit clearance.

## 2026-05-19 19:45 BST — implementation

**UX fork:** went with option **(2)** as recommended.
Global toggle in Settings stays as the default-for-
new-panes; per-side override sits on the Hybrid
chrome next to the back-attention dot. Discoverable
exactly where the feature lives.

A single icon button at `.actions` (the chrome row's
right-anchored area) cycles `pane.theme`:

* `undefined` (follow global) → click → opposite-of-
  global as explicit override.
* Set value → click → back to `undefined`.

Icon shows the theme the click WILL apply (Sun in
dark mode offers a switch to light, Moon in light
mode offers a switch to dark). When the override
is active, the button borders + icon paint with
`--link` so it's visible at a glance that this pane
diverges from the global.

**Why not three states (dark / light / follow)?** The
walker's table only needs two effective states per
pane (dark or light); whether the resolved value
came from override or global is internal. A binary
toggle reads simpler at the corner-of-vision scale
the chrome row supports, and a third "follow"
explicit state is reachable by clicking once more
(the "click to follow global" tooltip surfaces the
state). If a future need calls for a discrete
"follow" button, the toggle can split into a
segmented control without changing the underlying
`pane.theme` model.

**Edits:**

* `web/src/App.svelte` — extend the existing
  `:global(:root)` dark-tokens block to also match
  `:global(.pane[data-theme="dark"])` (selector
  grouping; no token duplication). Same for
  `:global([data-theme="light"])` → also matches
  `:global(.pane[data-theme="light"])`. The pane-
  scoped overrides cascade into the pane subtree
  the same way the root selector cascades into
  the document.

* `web/src/components/Pane.svelte`:
  * Imported `Sun`, `Moon` icons from `lucide-svelte`.
  * Imported `scheduleSessionSave`, `ui` from
    `state/store.svelte`.
  * Added `paneEffectiveTheme()`,
    `paneThemeTooltip()`, `togglePaneTheme()`
    helpers in the same area as `onFlipHybrid`.
  * Added `data-theme={pane.theme}` to the pane
    root `<div>`. Renders no attribute when
    `pane.theme === undefined`; matches the
    `data-focus-color` neighbouring attribute.
  * Added a `<button class="pane-theme-toggle">`
    in `.actions`, immediately before the
    hamburger trigger (so it sits adjacent to the
    chrome the user is already scanning). The
    button paints with `--link` border + colour
    when the override is active. Sun / Moon glyph
    shows the theme the click WILL apply.
  * CSS: `.pane-theme-toggle` styling next to the
    `.back-attention` block (~22×22 chrome
    footprint, transparent default, link-coloured
    when `.overridden`).

* `web/src/components/perHybridTheme.test.ts`
  (new) — source-grep sentinel asserting four
  invariants the render wiring depends on:
  1. Pane root carries `data-theme={pane.theme}`.
  2. The `pane-theme-toggle` button + handler
     render.
  3. `togglePaneTheme` cycles through `undefined`
     (= follow global) and the inverse override,
     and calls `scheduleSessionSave()`.
  4. App.svelte CSS has both
     `.pane[data-theme="dark"]` and
     `.pane[data-theme="light"]` selectors
     grouped with their root counterparts.

  Same pattern as `revealBrowserActions.test.ts`
  and the earlier source-grep sentinels.

**Tests for the model layer were already in place**
(from `fullstack-48` phase A): existing
`flipHybrid` round-trip tests cover swap-on-flip,
back-side preservation, and `ht` / `hb` hash
round-trip. No need to duplicate that coverage
here.

**Hash round-trip (per 18:00 BST directive).**
Confirmed by inspection:

* Render reads `pane.theme` directly (no
  intermediate `ui.themeChoice` reference).
* `pane.theme` derives from the URL hash via the
  existing `node.ht` → `p.theme` restore at
  `tabs.svelte.ts:2979` (back-side mirror exists
  in the `node.bt` branch). I didn't touch that
  code.
* `togglePaneTheme()` writes directly to
  `pane.theme` (a $state-proxied field) and calls
  `scheduleSessionSave()`. The next serialize sees
  the current value.

**Gate.** `npm run check` 0 errors / 0 warnings
(the GraphPanel chrome-btn warnings cleared in
`fullstack-64`'s revision — no longer noise);
`npm run test` 36 files / 378 tests passed
(was 35 / 343 pre-58; carrying +5 from
parallel-lane work + my 3 from -58 + 4 from -59);
`npm run build` clean; `scripts/pre-push` green.

**Visual eyeball.** Skipped per the lane-boundary
rule's "MAY ad-hoc serve" wording — the source-
grep tests pin the wiring, and the model-layer
tests already pass the walker's verification
table. The render wiring is a one-line attribute
+ a CSS rule grouping; both are mechanical
enough that a unit test asserting the strings
are present catches regressions reliably. If
@@Alex flags pixel issues on walkthrough, I'll
follow up.

**Re-walk flag.** Per task note: `webtest-b-6`
item 11 should re-walk once this lands. Lane A's
in-flight `webtest-a-11` may absorb it. Architect
to coordinate.

**Out of scope (deliberately):**

* No three-state explicit "follow global" button.
  Toggle cycles through `undefined` (follow) → set
  → `undefined`; the tooltip surfaces the current
  state. If UX feedback requires an explicit
  "follow" mode, the toggle can split.
* No keyboard binding for the theme toggle. Cmd+K
  surface is already crowded; the chrome button
  is the canonical entry.
* No interaction with `setThemeChoice()` (the
  global toggle in Settings). Per recommendation
  (2), global toggle keeps its existing semantics
  as "default for new panes".

**Commit readiness:**

Files staged:
* `web/src/App.svelte` (CSS selector grouping)
* `web/src/components/Pane.svelte` (attribute +
  button + toggle function)
* `web/src/components/perHybridTheme.test.ts`
  (new sentinel test)
* `docs/journals/phase-7/fullstack-b/fullstack-59.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Wire per-Hybrid theme into render with chrome toggle (fullstack-59)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 17:20 BST cut.
