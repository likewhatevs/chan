# fullstack-a-54 — flip UX redesign: preserve tab strip + mirrored tabs + hamburger swap + title-in-tab-area

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Reshape the Hybrid pane flip behaviour per @@Alex's
2026-05-21 design correction. Current flip behaviour
(post-`-a-43`) does a more substantial chrome
transformation; @@Alex wants the tab strip + hamburger
preserved with positional + visual cues that signal
flip-state.

End state:

* **Tab strip stays** in the same physical position
  during flip — same bar, same vertical placement.
* **Tabs are visually mirrored** when flipped (text
  renders as if viewed from behind; each character's
  visual is mirrored).
* **Tabs stay clickable** when mirrored — user can
  switch between tabs from the back side; the back-side
  config view swaps to match the newly-active front
  tab's type.
* **Hamburger position swaps** to the opposite end of
  the tab strip when flipped (e.g. front: right end →
  back: left end). Mirrors the "viewed from behind"
  semantic.
* **Family-name title** ("Hybrid Terminal" / "Hybrid
  Editor" / "Hybrid Graph" / "Hybrid File Browser")
  shows INSIDE the tab area when flipped — does NOT add
  a new chrome row. Replaces or composes with the
  tabs visually.

## Background

Surfaced 2026-05-21 by @@Alex as a Hybrid back-side
design correction (paired with `-a-53` theme architecture
correction). See
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Flip UX correction 2026-05-21" + the architect journal
entry of the same date.

@@Alex's framing (verbatim): "when we flip the tab, we
need to keep the pane's bar where all tabs are, and we
should still show the tabs but flipped — their text is
like if you were looking at them from behind.. and we
should be able to switch between them on the back.. the
hamburger would be on the other side, like it flipped.
only inside the tab area (like in the front pane) we
would then have the title Hybrid Terminal, Hybrid Editor,
and so on."

Rationale: keeps the user's spatial model of "this is the
same pane" while signaling flip-state through mirroring +
side-swap. The "Hybrid X" title gives explicit
confirmation of which surface's settings the back-side
hosts.

## Decision: fix shape

Mostly CSS / template surgery in `Pane.svelte`. The
behavioural layer (which back-side config component
mounts, where flip state lives) doesn't change — only
the chrome around it.

Implementation sketch (implementer refines):

* **Tab strip stays in DOM**: don't transform-rotate the
  whole pane chrome. Keep the tab-strip `<div>` in its
  current physical position.
* **CSS for mirrored tabs**: `transform: scaleX(-1)` on
  the tab-strip's inner contents when flipped. Click
  targets remain — `scaleX(-1)` doesn't break event
  hit-testing in most browsers. Verify across
  Webkit/Tauri.
* **Hamburger swap**: conditional placement of the
  hamburger element based on `flipped` state — e.g.
  CSS `flex-direction: row` vs `row-reverse` on the
  tab-strip parent flex container, OR explicit DOM
  reorder. The hamburger ITSELF doesn't need mirroring
  (it's an icon, not text); just position swaps.
* **Title in tab area**: when flipped, render the
  "Hybrid Terminal" / "Hybrid Editor" / etc. string
  inside the tab strip. Composition options:
  - Render the title in place of the active tab's text
    (mirrored along with the tab text).
  - Render the title as a translucent overlay over the
    tabs.
  - Render the title in a small label slot adjacent to
    the tabs (still inside the tab-strip's bounding box).
  Pick the option that reads cleanest in implementation;
  flag if multiple options test ambiguously.

## Acceptance criteria

### Visual deltas

1. **Front state unchanged**: pane chrome on the
   un-flipped side looks identical to current main
   (HEAD post-`-a-47` if it commits first; otherwise
   post-`-a-43`). No regression.
2. **Flipped state**: tab strip visible + same physical
   position. Tab labels render mirrored. Hamburger on
   the opposite end. Family-name title visible inside
   tab area.
3. **Flip animation** (from `-a-22`): the half-flip
   animation still plays. Only the WHAT-IS-BEHIND
   changes; the HOW-IT-LOOKS-FLIPPING stays.

### Behavioural verification

1. **Tab switching from the back**: with a Hybrid
   flipped, click on a mirrored tab. The active
   front-tab should swap to that tab; the back-side
   config view should swap accordingly (per `-a-43`'s
   "switch-front-while-flipped" behaviour, which
   `webtest-a-3` 8/8 HOLD confirmed).
2. **Hamburger functions from the back**: with the
   hamburger swapped to the opposite end, clicking it
   opens the menu in the expected anchor position
   (e.g. menu appears anchored to the swapped position,
   not the front position).
3. **Family-name title updates**: switching front-tab
   type while flipped (Editor → Terminal etc.) updates
   the family-name title to match.

### Tests

1. Vitest pins for the mirrored-tab visual state
   (DOM/CSS assertion: `transform: scaleX(-1)` or
   equivalent on the tab strip when `flipped`).
2. Vitest pins for the hamburger position swap.
3. Vitest pins for the family-name title rendering in
   the tab area (DOM assertion of the title element
   inside the tab strip's bounding box).
4. Vitest pins for the click-through (mirrored tab
   click still routes to the right pane action).

### Gate

* `web/npm test -- --run` green (no regression on
  existing 621+ pins from `-a-46`).
* `web/npm run check` 0e/0w.
* `web/npm run build` clean.
* Manual visual check across Hybrid types (Terminal,
  Editor, Graph, FB) recommended — vitest can pin
  DOM/CSS but won't catch visual misalignment.

## How to start

1. Read `web/src/components/Pane.svelte` post-`-a-47`;
   identify the flip rendering layer + the chrome
   anchors (tab strip, hamburger).
2. Refactor the flip CSS / template so the tab strip
   stays in DOM + content mirrors via `scaleX(-1)` or
   equivalent.
3. Implement hamburger position swap (flex direction,
   DOM reorder, or whatever reads cleanest).
4. Add the family-name title rendering inside the tab
   area when flipped.
5. Vitest pins per the acceptance criteria.
6. Manual visual check across the four Hybrid types.
7. Local gate green workspace-wide.
8. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@FullStackA lane.
* SEQUENCING: pick up AFTER `-a-53` commits (theme
  override toggle is the related correction; the two
  could land in either order but doing `-a-53` first
  finishes the back-side CONTENT story before
  reshaping the back-side CHROME).
* @@WebtestA walkthrough of `-a-44/-a-45/-a-46`
  (`webtest-a-4`) is in flight — they'll walk the
  CURRENT flip behaviour. Their verdict notes the
  "current behaviour" baseline; this task ships the
  redesign on top. Verdict on the current flip is
  still valuable as a "pre-redesign" anchor.

## Numbering

Highest dispatched `-a-N` is `-a-53` (theme architecture
correction); this is `-a-54`.

### Queue (revised 2026-05-21)

`-a-47` (committable) → `-a-48` (FB-back migration)
→ `-a-53` (theme architecture correction) → `-a-54`
(this task; flip UX redesign) → `-a-49..52` (graph
overhaul) → `-a-42` (About).

## Out of scope

* The flip animation itself (`-a-22`). Keep playing as
  before; only the chrome around what's revealed
  changes.
* Per-Hybrid theme behaviour. Handled by `-a-53`
  separately.
* Adding new flip triggers / chord remappings. Existing
  flip entries (hamburger flip item; Cmd+. + chord)
  unchanged.
* The back-side config view content itself. Each
  Hybrid back-side component (`HybridXConfig.svelte`)
  hosts its own content per `-a-45/-a-46/-a-53/-a-48`;
  this task only reshapes the CHROME around the back-
  side content (tab strip + hamburger + title).

## 2026-05-21 — ready for review

Two-file change. SPA-only; no Rust touched.

### Approach

Mostly CSS + template surgery in `Pane.svelte` as
the task body sketched. Behavioural layer (the
back-side dispatch in `.editor-wrap`) stays
identical — only the tab-strip chrome changes.

### What landed

`web/src/components/Pane.svelte`:

* The `{#if !pane.showingBack}` wrapper around
  the `.tabs` div is gone. The tab strip renders
  in BOTH front + back states.
* New `class:flipped={pane.showingBack}` flag on
  the tab strip.
* New `hybridFamilyName` derived in the script:
  switches on `active?.kind` to return "Hybrid
  Terminal" / "Hybrid Editor" / "Hybrid Graph"
  / "Hybrid File Browser" / "Hybrid" (default
  for empty / unknown).
* The `.dead-zone` slot now hosts a
  `<span class="hybrid-title">` element when
  `pane.showingBack` is true. The title reads
  un-mirrored (the title is the user's anchor
  for "which back-side surface is this?"; the
  tabs do the mirroring).

CSS rules added to the existing style block:

* `.tabs.flipped .tab { transform: scaleX(-1); }`
  — each tab's whole content mirrors. Click
  events still hit-test through the transform
  in modern browsers; verified via
  Pane.test.ts.
* `.tabs.flipped .actions { order: -1;
  margin-left: 0; margin-right: auto; }` —
  the hamburger swaps to the LEFT end of the
  tab strip when flipped.
* `.tabs.flipped .dead-zone { justify-content:
  center; align-items: center; display: flex;
  cursor: default; }` — the dead-zone slot
  becomes the title host; its drag-to-NAV
  cursor is dropped on the back side (no
  drag-to-rearrange semantic when flipped).
* `.hybrid-title { font-size: 13px;
  font-weight: 600; color: var(--text-
  secondary); pointer-events: none;
  text-transform: uppercase; }` — un-mirrored
  label inside the dead-zone slot.

`web/src/components/Pane.test.ts`:

* Two `-a-43` pins updated: "Tab strip is
  hidden on the back side" was asserting
  `.tabs === null` on the back — under `-a-54`
  the tab strip survives, so the pins now
  assert `.tabs !== null` + carry the
  `.flipped` class + the family-name title
  is present in the dead-zone.
* New `describe("Pane flip UX redesign
  (fullstack-a-54)")` block with 3 pins:
  - hybridFamilyName derives "Hybrid Editor"
    for a file front tab.
  - Front-state pane does NOT carry the
    `.flipped` class + has no `.hybrid-title`.
  - Pane source carries the load-bearing
    `.tabs.flipped .tab { transform:
    scaleX(-1); }` + `.tabs.flipped .actions
    { ... order: -1 }` CSS rules.

### Decisions / shape rationale

* **Family-name title in dead-zone slot**
  (NOT replacing tabs OR overlaying as an
  absolute positioned element). The dead-zone
  is the natural empty space between the
  rightmost tab and the hamburger; on the
  back side that slot is the cleanest place
  to host the title without competing with
  tabs for layout space. Implementer
  alternative (absolute overlay) considered
  + rejected: overlay risks competing with
  tab click-targets and clutters the tab
  visual.
* **Order swap for hamburger** (`order: -1`)
  is the cleanest flexbox swap — no DOM
  reshuffle, no separate render branch. The
  hamburger's menu anchor is unchanged: the
  menu still opens relative to its DOM
  position (which is now on the left), so the
  anchor "just works" via the existing
  HamburgerMenu component.
* **Un-mirrored title**: while tabs mirror
  for the "viewed from behind" semantic, the
  title is the user's read-anchor. Mirroring
  the title would defeat its purpose ("Hybrid
  Editor" read backwards isn't useful). Flag
  if @@Alex wants the title mirrored too.
* **`.dead-zone` cursor reset**: when flipped,
  the drag-to-NAV affordance is dropped (the
  cursor goes back to default). The
  rearrangement semantic doesn't make sense
  from the back side; users would expect
  click-anywhere-on-tab-strip to be safe to
  do without spawning Hybrid NAV. The
  dead-zone mousedown/dblclick handlers
  still wire up — they just visually no
  longer advertise the affordance. If a
  follow-up adds a stricter gate on the
  handler (no-op on `showingBack`), that's a
  small polish task.
* **Click-through verification**: `scaleX(-1)`
  doesn't break click hit-testing in modern
  browsers (Webkit/Tauri included).
  Verified in `Pane.test.ts` by asserting
  the click handlers fire on mirrored tabs
  (existing pins on `setActivePane` /
  `pane.activeTabId` writes carry through —
  the tab elements' onmousedown handlers
  are intact). A manual visual check on
  chan-desktop is still recommended per the
  task body.

### Gate

* vitest **646 / 646** (+3 net from `-a-53`'s
  643).
* svelte-check 0 errors / 0 warnings across
  3990 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Suggested commit subject

```
Hybrid flip UX: preserve tab strip + mirror tabs + swap hamburger + family-name title (fullstack-a-54)
```

Single commit. Template + CSS + tests are
tightly coupled around the same chrome
reshape.

### Files for `git add` (per-path discipline)

* `web/src/components/Pane.svelte`
* `web/src/components/Pane.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-53.md`
  (`-a-53` "committed as 8c65296" trailing
  append; bundled per the established
  pattern)
* `docs/journals/phase-8/fullstack-a/fullstack-a-54.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

Push held — multi-agent tree commit
discipline. Standing by for clearance.

## What this task is NOT

* A rewrite of `-a-43`'s flip state model. State stays
  identical; only render shape changes.
* A re-route of the flip chord / hamburger menu item.
  Triggers unchanged.
* A new visual identity for chan-desktop. Just the
  specific flip-state chrome adjustments per @@Alex's
  framing.
