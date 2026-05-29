# fullstack-34: pane chrome refinements + wobble + close-all-tabs/close-pane split

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Three small pane-level UX refinements from @@Alex's
click-around session 2026-05-19 05:30 BST. They're
unrelated but all live in the pane chrome / hamburger
menu, so bundle them.

## Acceptance criteria

### Pane chrome — floating shade + rounded + border space

* Add a small amount of padding/border space around each
  pane so panes don't butt directly against each other
  or against the workspace edge.
* Slightly round the pane corners (subtle radius —
  ~4-6px feel, not visually loud).
* Each pane gets a slight shadow / shade behind it so
  it reads as "floating" over the workspace background.
  * **Dark mode** → shadow is white-ish (subtle white
    glow / soft white shadow).
  * **Light mode** → shadow is black-ish (standard soft
    drop-shadow).
* The pane focus-border color (from `fullstack-30`) sits
  on top of this base chrome; nothing about that
  changes.

### Wobble on split / delete / pane-move

* When a pane is split (creating two), both the source
  pane and the new sibling do the existing CSS hover-
  wobble animation once.
* When a pane is closed (sibling absorbs the freed
  space), the absorbing pane wobbles.
* When focus moves between panes via Cmd+K mode +
  WASD/arrow-swap (`fullstack-16`), the panes involved
  wobble.
* Reuse the existing hover-wobble keyframes — no new
  animation. Just trigger them on these events.
* Wobble is single-fire (not looping); duration matches
  the hover wobble.

### Remove Split right / Split down from non-hamburger menus

@@Alex 2026-05-19 05:45 BST: Split entries currently
appear in multiple right-click menus (empty-pane
welcome menu per `fullstack-28`, possibly elsewhere).
Strip them from everywhere except the pane's hamburger
menu. The hamburger is the canonical home for
structural actions; redundant copies in other menus
just create choice paralysis.

* Audit: terminal tab right-click menu, doc tab right-
  click menu, empty-pane welcome menu, any other
  surface that exposes a Split affordance.
* Drop Split right + Split down from each. Keep them
  only in the pane hamburger menu.
* `splitPane` programmatic API stays — drag-detach
  (`fullstack-15`) uses it.

### Empty-pane left-click opens the welcome menu (regression)

@@Alex 2026-05-19 05:50 BST: clicking the empty pane's
background with the LEFT mouse button also triggers
the welcome (right-click) menu. Same class as B15 from
`fullstack-6`; the welcome menu added in `fullstack-28`
must have re-wired left-click. Fix: only right-click
opens the welcome menu; left-click on empty-pane
background is a no-op (or selects the pane).

### Hamburger menu: split "Close all tabs" from "Close pane"

* Today the hamburger menu has a single combined "close
  all tabs and pane" action (per current `Close pane`).
* Split into two items:
  * **Close all tabs** — closes every tab in the pane
    but keeps the pane itself (welcome state shows
    again).
  * **Close pane** — closes the pane (and its tabs).
    Sibling absorbs the space.
* Both items render in the hamburger; sequence stays
  as `fullstack-30` shipped, with "Close all tabs"
  inserted right above "Close pane":
  ```
  Focus border color
  ─────────────────────
  Next pane / Previous pane
  ─────────────────────
  Split right / Split down
  Close all tabs           ← new
  Close pane
  ```

## Out of scope

* The carousel widget on empty panes (separate task,
  `fullstack-35`).
* Graph-from-here behavior (separate task,
  `fullstack-32`).
* Animation primitives beyond reusing hover-wobble.

## How to start

1. Pane chrome: `Pane.svelte` (or its stylesheet).
   Add the padding + border-radius + shadow rules.
   Use CSS variables tied to the theme so dark/light
   mode swap is automatic.
2. Wobble: grep for the existing hover-wobble
   keyframes. Trigger them via a small `wobbleOnce`
   helper that adds a class for one animation cycle
   then removes it. Wire into the split / close /
   pane-move code paths.
3. Hamburger items: add the new `Close all tabs`
   action — clears `pane.tabs` array but keeps the
   pane node in the tree. `Close pane` remains
   as-is.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-19 10:20 BST — implementation note (@@FullStackB)

Landed. Files touched:

* `web/src/App.svelte` — added `--pane-shadow` theme vars
  (dark = subtle white glow, light = soft black drop) and
  set `main { background: var(--bg-card) }` so the new
  pane chrome reads as a card floating on the workspace.
  Without the backdrop step the shadow had nothing to
  fall on (both pane and `main` shared `--bg`).
* `web/src/state/tabs.svelte.ts` — added the
  `paneWobble` bus + `requestPaneWobble()`. Wired into
  `splitPane` (both source + sibling), `collapseEmptyPane`
  (whichever leaf absorbs the freed space), and
  `paneModeSwap` (both panes). `paneModeMoveFocus` does
  NOT wobble — that's a focus highlight move, not a
  structural change.
* `web/src/components/Pane.svelte`:
  * chrome on `.pane`: `margin: 4px`, `border-radius:
    6px`, `overflow: hidden`, `box-shadow: var(--pane-
    shadow)`. `.focused` composes the inset focus ring
    with the chrome shadow.
  * `pane-wobble-once` keyframe (0% → 40% scale 1.012 →
    100%, easeOutBack cubic-bezier). Class toggled via
    bus version + rAF retrigger; cleared in
    `onanimationend`.
  * Hamburger menu: new "Close all tabs" item (ListX
    icon) between "Split down" and "Close pane". Calls
    `closeTabsInPane(pane.id)` which clears tabs but
    keeps the pane (vs `closePane` which also collapses
    the pane node).
  * Empty-pane welcome menu: stripped the `Split right`
    and `Split down` entries.
  * `.placeholder` lost its `onclick={() =>
    setActivePane(pane.id)}`. The parent `.pane`
    `onmousedown` already activates the pane on any
    mouse press, so left-click on the empty background
    is now a no-op visually. Right-click still opens the
    welcome menu via `oncontextmenu`.
* `web/src/components/FileEditorTab.svelte` — dropped
  `Split right` / `Split down` from the tab right-click
  menu. Removed now-unused `canSplit` / `splitActive`
  imports and `splitsAllowed` derivation.
* `web/src/components/TerminalTab.svelte` — same strip
  on the terminal tab right-click menu.
* `web/src/components/Pane.test.ts`:
  * Updated the hamburger menu labels test to include
    "Close all tabs" between "Split down" and "Close
    pane".
  * Updated the empty-pane welcome menu labels test to
    drop "Split right" / "Split down".
  * Added a regression test: left-click on `.placeholder`
    leaves the welcome menu closed (assert no
    `.hamburger-menu` portal node in the body).

Verification (BST 10:20):

* `npm run test -- Pane tabs` → 2 files, 51 tests pass.
* `npm run test` → 30 files, 268 tests pass.
* `npm run check` → 0 errors / 0 warnings.
* `npm run build` → clean (existing chunk-size + dynamic
  import warnings unchanged).
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` → green
  (fmt + clippy + cargo test + no-default-features build).

Visual sanity (one screenshot in light mode before
backing off per the new lane boundary): the
`--bg-card` backdrop fix was necessary — without it the
pane edges and shadow had no contrast against the
workspace. With the fix applied, the pane chrome reads
correctly. Going forward, end-to-end visual passes route
through @@Webtest via @@Architect.

Commit message proposed: `Pane chrome + structural
wobble + Close all tabs (fullstack-34)`.
