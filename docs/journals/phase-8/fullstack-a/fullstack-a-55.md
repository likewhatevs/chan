# fullstack-a-55 — remove family-name title from tab strip in flipped state (-a-54 follow-up)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Remove the family-name title ("HYBRID FILE BROWSER" /
"HYBRID TERMINAL" / "HYBRID EDITOR" / "HYBRID GRAPH")
from the flipped tab strip. `-a-54` placed it in the
dead-zone slot of the tab strip per my misinterpretation
of @@Alex's original framing; @@Alex 2026-05-21 (chat
screenshot, post-`-a-54`) corrected: "we should keep
just the tabs there, flipped, no need to add that extra
label; i saw the same with terminal."

End state in flipped tab strip:

* Mirrored tab labels (per `-a-54`, kept).
* Hamburger on opposite end (per `-a-54`, kept).
* **Tabs aligned to the RIGHT** (NEW; @@Alex 2026-05-21:
  "when we flip, the tabs must be aligned to the right..
  not to the left, because we flipped"). Currently the
  tabs flow from the left even when flipped; the
  flipped semantic should reverse their flow.
* **NO family-name title in the tab strip**.

The back-side config view BELOW the tab strip already
displays the family-name title (from `-a-43`'s stubs);
no duplication needed in the chrome.

## Background

@@Alex's original framing on the flip UX correction
2026-05-21 read (verbatim): "only inside the tab area
(like in the front pane) we would then have the title
Hybrid Terminal, Hybrid Editor, and so on."

Architect misinterpretation: I read "inside the tab area"
as "in the tab strip chrome." Spec'd in `-a-54` task body
under "Family-name title in tab area" — explicitly
saying "shows INSIDE the tab area when flipped — does
NOT add a new chrome row." @@FullStackA implemented the
title in the dead-zone slot per my spec.

@@Alex's actual intent (clarified post-`-a-54` ship):
"tab area" referred to the back-side CONFIG VIEW area
(the surface that mounts when flipped — `HybridXConfig.svelte`
components). Those components already carry the
family-name title at their top per `-a-43`'s stub
shape. No need to add a SECOND title in the chrome.

Screenshot in chat showed the duplication clearly:

* Tab strip: `× chan-test-phase8-wa-r…` + `CLAUDE.md` +
  "HYBRID FILE BROWSER" (the unwanted title).
* Below: the actual back-side config view, with its own
  "Hybrid File Browser" title from the component.

This task removes the tab-strip occurrence.

## Decision: fix shape

Pure removal in `Pane.svelte`'s flipped-state template
+ the supporting CSS. The back-side config view
component is untouched (it already carries the family
title at its top per `-a-43`).

* Remove the family-name title element from the tab
  strip when flipped.
* Remove the supporting CSS class (e.g. `.hybrid-title`
  or whatever `-a-54` introduced).
* Update the corresponding `Pane.test.ts` pins from
  `-a-54` — the test that asserts the family-name title
  is in the tab area becomes the regression-guard that
  it ISN'T (or remove that pin entirely if the back-side
  config view's title is already covered by other tests).

## Acceptance criteria

### Tab strip in flipped state

1. Open a Hybrid pane; flip it. Confirm the tab strip
   shows ONLY mirrored tabs + the hamburger (on the
   swapped end). NO family-name title in the strip.
2. Walk all four Hybrid types (Terminal, Editor, Graph,
   File Browser); confirm none of them show the
   family-name title in the tab strip.
3. **Tabs aligned to the RIGHT** in flipped state. The
   tabs flow from the right-hand side; the hamburger
   sits on the LEFT (the swapped position). Visual
   layout in flipped state:
   ```
   [≡ hamburger] [....empty space....] [tabN] [tabN-1] ... [tab1] [tab0]
   ```
   (Tabs mirrored individually; their COLLECTIVE
   alignment also reverses — flow from right.)

   Implementation hint: this likely composes with the
   existing `flex-direction` swap from `-a-54`. If the
   parent flex container is `row-reverse` when flipped,
   the children naturally flow from the right; combined
   with `transform: scaleX(-1)` per-tab, the visual reads
   as "looking from behind, tabs flow from the right
   edge."

   Alternative: explicit `justify-content: flex-end` on
   the tab container when flipped, with `row` direction
   preserved. Whichever shape composes cleanest with the
   existing hamburger swap.

### Back-side config view (unchanged)

3. The back-side config component still displays its
   family-name title at the top of its content area
   (per `-a-43`'s stub shape; `-a-45` / `-a-46` /
   `-a-48` / `-a-53` keep this).

### Tests

4. Update `Pane.test.ts` pins from `-a-54`:
   * Remove the "family-name title visible in tab area"
     pin OR invert it into a "family-name title NOT in
     tab area" regression guard.
   * Keep the mirrored-tab pin.
   * Keep the hamburger-swap pin.
   * Keep the click-through pins.
5. Existing `HybridXConfig.test.ts` pins for the
   family-name title at the top of the config view
   should remain (those tests verify the back-side
   component's own title, which is unchanged).

### Gate

* `web/npm test -- --run` green (vitest count drops by
  whatever pins were specifically about the tab-area
  title; otherwise unchanged).
* `web/npm run check` 0e/0w.
* `web/npm run build` clean.

## How to start

1. Read `web/src/components/Pane.svelte` post-`-a-54`;
   identify the family-name title rendering + the
   supporting CSS class.
2. Remove the title element + CSS class.
3. Update / invert the `Pane.test.ts` pins from `-a-54`.
4. Verify the four Hybrid types still flip cleanly (tab
   strip + back-side config view both correct).
5. Local gate green workspace-wide.
6. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@FullStackA lane.
* Pure SPA / CSS surgery in `Pane.svelte` +
  `Pane.test.ts`. No interaction with other lanes.
* `webtest-a-5` (in flight as dispatched task) has been
  updated to reflect this correction — @@WebtestA's
  walk will GRADE the absence of the family-name title
  in the tab area as PASS (or graceful HOLD with note
  that `-a-55` is the corrective follow-up).

## Numbering

Highest dispatched `-a-N` is `-a-54`; this is `-a-55`.

### Queue (revised 2026-05-21)

```
-a-55 (this task; tab-strip title removal — short correction)
-a-49..52 (graph overhaul first sub-wave)
-a-42 (About; A+B+C+F all in HEAD)
```

`-a-55` inserts AHEAD of `-a-49..52` because it's a
small correction to the just-shipped `-a-54`; better to
close the design-correction loop before moving to the
next major surface.

## Bundled scope addition 2026-05-21 — fix -a-54 click-existing-mirrored-tab PARTIAL

@@WebtestA's `webtest-a-5` walk surfaced one PARTIAL
on `-a-54` check #6: from the flipped back side,
clicking an existing mirrored tab in the tab strip
does NOT swap the active tab. The spawn-from-FB-sidebar
+ spawn-via-chord paths DO swap the back-side config +
family-name title cleanly — so the back-side title-swap
mechanic itself works. Only the click-driven active-tab
switch is broken.

### Root-cause hypotheses (from @@WebtestA's verdict)

* CSS `scaleX(-1)` transform on mirrored tab elements
  may be capturing pointer events incorrectly.
* OR the back-side tab strip may be rendering a static
  visual copy without binding the click handler.

Verified empirically via Chrome MCP click on the DOM ref
AND programmatic `tab.click()` + full-sequence
`pointerdown/mousedown/pointerup/mouseup/click` —
neither swapped active.

### Why bundle into -a-55

`-a-55` is already touching `Pane.svelte`'s flipped tab
strip chrome (removing family-name title + adding tab
right-alignment). The click-handler fix likely lives in
the SAME surgery surface. Folding the PARTIAL fix into
the same commit keeps the queue compact AND ensures the
right-alignment + the click-handler land together (so
the fix doesn't need to be re-verified against a
half-shipped tab strip).

### Acceptance criterion

After `-a-55` lands: from a flipped Hybrid, click on any
mirrored tab. Active tab swaps to that tab; back-side
config view + back-side family-name title update
accordingly. Add a Vitest pin for this; the existing
"click-through" pin from `-a-54` may already cover it
or may need extending — implementer picks.

If the root cause is the `scaleX(-1)` transform, fix
likely involves applying the transform to a CHILD element
(the tab label) rather than the entire tab `<button>` /
`<a>` so the click target stays unmirrored. Alternative:
keep the transform on the parent but explicitly set
`pointer-events: auto` on the click-targeted child
elements. Implementer picks the cleaner shape.

If the root cause is missing handler binding (the back-
side tab strip rendering a copy), wire the click handler
to the same dispatch the front-side tab strip uses.

## Out of scope

* Touching the back-side config view's own family-name
  title (those are correct; per `-a-43`'s stub shape).
* Re-routing the flip animation or the back-side
  component dispatch (both stay as `-a-54` shipped).
* Adding any NEW visual indicator in place of the
  removed title. @@Alex's framing: "we should keep
  just the tabs there, flipped, no need to add that
  extra label." The mirrored tabs + the swapped
  hamburger are the visual cues; the back-side config
  view's own content is the contextual cue. No
  additional chrome-level cue needed.

## What this task is NOT

* A wholesale revert of `-a-54`. The mirrored-tab shape +
  hamburger swap are CORRECT + stay.
* A re-architecture of the flip behaviour. Just removing
  the misplaced family-name title.

## 2026-05-21 — ready for review

Two-file change. SPA-only; no Rust touched. Three
load-bearing pieces.

### Architecture

**1. Family-name title removed from tab strip**:

* `Pane.svelte`: dropped the `hybridFamilyName` derived
  helper (was the script-side feeder for the title).
* Dropped the `<span class="hybrid-title">` element from
  the `.dead-zone` slot — the dead-zone goes back to
  pure spacer.
* Removed the `.hybrid-title` CSS class + the
  `.tabs.flipped .dead-zone { display: flex; justify-
  content: center; ... }` centering rules.
* Back-side config view's own family-name title (the
  one in `HybridXConfig.svelte`'s `<h2>` per `-a-43`'s
  stub) is unchanged. That's the canonical surface.

**2. Right-align tabs when flipped**:

* `.tabs.flipped` gains `flex-direction: row-reverse`.
* `.tabs.flipped .actions` order swapped from `-1` to
  `1`. Under row-reverse, the highest order ends up
  visually first (LEFT edge).
* Layout result (left-to-right visually):
  `[≡ hamburger] [dead-zone fills slack] [tabN ... tab0]`.
  Tabs flow from the right edge; tab0 is rightmost.

**3. Fix click-on-mirrored-tab handler (webtest-a-5
PARTIAL on -a-54 check #6)**:

* Root-caused: `-a-54` applied `transform: scaleX(-1)`
  to the whole `.tab` element. The transform broke
  click routing on the back-side per @@WebtestA's
  empirical verification (DOM ref click + programmatic
  `tab.click()` + full pointer-event sequence all
  failed to swap active).
* Fix: move the mirror to per-CHILD selectors
  (`.tab-icon` + `.path` + `.dirty` + `.broadcast-
  marker` + `.marker`). Each visual child mirrors via
  `transform: scaleX(-1); display: inline-block;` —
  this preserves "viewed from behind" semantics while
  keeping the `.tab` element's own bounding box in
  natural coordinates so its click target lives where
  the browser expects.
* The close button (`<button class="close">×</button>`)
  is NOT mirrored — `×` is a standard close affordance
  and mirroring it makes it look reversed.

### Files

`web/src/components/Pane.svelte`:

* Script: `hybridFamilyName` derived removed.
* Template: `<span class="hybrid-title">` removed from
  the `.dead-zone` slot.
* CSS: `.hybrid-title` rule removed; `.tabs.flipped
  .dead-zone` centering removed (kept the
  `cursor: default` reset); the
  `.tabs.flipped .tab { transform: scaleX(-1) }`
  whole-element transform removed; `.tabs.flipped`
  gains `flex-direction: row-reverse`;
  `.tabs.flipped .actions` order flipped `-1` → `1`;
  per-child mirror rules added for
  `.tab-icon`/`.path`/`.dirty`/`.broadcast-marker`/`.marker`.

`web/src/components/Pane.test.ts`:

* Existing `-a-54` "Hybrid X title in tab area" pin
  inverted into a regression guard: assert
  `.hybrid-title` IS null in the flipped state, and
  the back-side config view IS rendered.
* `-a-54` raw-source CSS guard rewritten:
  - Pin per-child mirror selectors instead of the
    whole-tab transform.
  - Pin `flex-direction: row-reverse` on
    `.tabs.flipped`.
  - Pin `.tabs.flipped .actions { order: 1 }`.
  - Old whole-tab transform + old `order: -1`
    rejected via `not.toMatch` so a future revert
    trips the guard.
* New click-swap pin: dispatches `mousedown` on a
  flipped-state tab; asserts `pane.activeTabId`
  swaps. The handler bound to `.tab`'s `onmousedown`
  is now reachable via the natural click path.
  Verified locally; webtest-a-N empirically re-walks
  via Chrome MCP.

### Gate

* vitest **647 / 647** (+1 net from `-a-54`'s 646;
  one pin rewritten in place, one new click-swap
  pin added).
* svelte-check 0 errors / 0 warnings across
  3990 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Per-child mirror** vs other fixes (single
  wrapper span + `display: contents`,
  `pointer-events: auto` reset, `dir="rtl"` on tabs):
  per-child is the cleanest. Wrapper + `display:
  contents` breaks transforms (transforms require a
  box). `pointer-events: auto` doesn't address the
  underlying issue (the .tab IS the click target;
  resetting pointer-events on children just adds
  noise). `dir="rtl"` doesn't visually flip characters
  the way @@Alex's "viewed from behind" framing
  needs.
* **Close button NOT mirrored**: keeps the
  universally-readable `×` upright. Flag if @@Alex
  wants it mirrored too.
* **Counter-mirror NOT applied to hamburger icon**:
  the hamburger icon is symmetric horizontally
  (three stacked lines); mirroring/unmirroring is
  visually identical, so the position swap alone is
  enough.

### Manual verification recommendation

`webtest-a-N` walk-through on the next dispatch will
re-walk the click path via Chrome MCP. Local vitest
covers the handler binding (mousedown swaps active);
the empirical-browser side will confirm the click
path works through the modern browser's hit-testing
+ Tauri webview.

### Suggested commit subject

```
Hybrid flip UX: remove tab-strip title + right-align tabs + fix mirrored-tab click (fullstack-a-55)
```

Single commit. Three pieces are tightly coupled
chrome surgery on the same `.tabs.flipped` rule
set.

### Files for `git add` (per-path discipline)

* `web/src/components/Pane.svelte`
* `web/src/components/Pane.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-54.md`
  (`-a-54` "committed as 714ec48" trailing append;
  bundled per the established pattern)
* `docs/journals/phase-8/fullstack-a/fullstack-a-55.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

Push held — multi-agent tree commit discipline.
Standing by for clearance.

## Architect-side lesson logged

My `-a-54` task body's interpretation of "inside the tab
area" was wrong. @@Alex's actual intent was the back-
side config view's title band (which the
`HybridXConfig.svelte` stubs ALREADY had). Should have
read @@Alex's framing more carefully — "like in the
front pane" was the contextual hint that the title
behavior should mirror what already EXISTS on the front
pane, where the back-side config component carries its
own title. The chrome-level title was a redundancy I
introduced.

Pattern: when a design framing references "like the
front pane" or "like the existing X", read the existing
shape FIRST before specifying. Same discipline as the
`feedback_ground_descriptions_in_source` memory rule
applied to design framings, not just crate
descriptions.
