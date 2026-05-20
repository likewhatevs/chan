# fullstack-a-27: Hybrid pane hamburger — add dark/light mode toggle + flip button

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Two small additions to the Hybrid pane's hamburger menu:

1. **Dark/light mode toggle** for the current Hybrid pane
   (the per-Hybrid theme override from `fullstack-b-5`).
   Today the toggle lives elsewhere (probably the Pane Mode
   help cheatsheet or a global affordance); @@Alex wants
   it accessible from the hamburger directly.
2. **Flip button** that triggers `flipHybrid()` for the
   current pane. Today flip is chord-only (Cmd+. Tab per
   `fullstack-a-7`); add a clickable affordance in the
   hamburger.

## Background

@@Alex 2026-05-20:

> One more polish in the hybrid's hamburger: move the
> dark/light mode in there, and add flip button as well.

The Hybrid pane already has:
* Per-Hybrid theme override (`ht` / `hb` from
  `fullstack-59`, propagated to xterm + GraphCanvas in
  `fullstack-78`, propagated to editor surfaces in
  `fullstack-b-5`).
* Flip via chord (`fullstack-a-7` rebinding) + animated
  half-rotation since `fullstack-a-22`.

This task brings both into the hamburger menu as
mouse-driveable affordances. The chord paths stay
intact — these are additional surfaces, not
replacements.

## Distinct from related work

* **NOT the same as** the Round-2 chord migration +
  surface-unification task drafted in
  [`../architect/round-2-plan.md`](../architect/round-2-plan.md).
  That task covers the four spawn actions (Terminal /
  File Browser / Rich Prompt / Graph) appearing as
  first-class items in the carousel + hamburger +
  empty-pane right-click. This task is about Hybrid-
  specific pane operations (theme + flip).
* **Composes with** `fullstack-b-5` per-Hybrid theme:
  the toggle flips the per-pane `data-theme` attribute;
  every surface already inside the pane (editor /
  xterm / GraphCanvas) re-paints via the existing
  reactive token swap.
* **Composes with** `fullstack-a-22` flip animation:
  the flip button click triggers the same
  `requestPaneFlip(paneId)` bus that the chord uses,
  so the 3D card-flip animation plays from either
  surface.

## Acceptance criteria

* Hybrid pane hamburger menu shows two new entries
  (ordering up to the implementer — natural slot is
  near the existing pane-level operations like split /
  close, not mixed with the future first-class spawn
  items from the chord-migration task):
  * "Toggle dark/light theme" (or "Light mode" / "Dark
    mode" depending on current state). Click flips the
    per-Hybrid `data-theme`.
  * "Flip pane" — click triggers `flipHybrid()` /
    `requestPaneFlip(paneId)`. Plays the same
    half-rotation animation as the chord path.
* The two new entries appear ONLY when the pane is a
  Hybrid. Non-Hybrid panes (pure terminal / pure editor)
  don't show them (no flip semantic; theme override is
  Hybrid-specific per `fullstack-b-5`).
* Glyphs / icons consistent with the rest of the
  hamburger menu's existing visual weight.
* Hover tooltip on each entry surfaces the equivalent
  chord (so the user can learn the chord from the
  menu).
* Theme toggle persists per-Hybrid (already handled by
  the `fullstack-b-5` data-theme persistence; the new
  entry just triggers a flip of the existing value).
* Flip behaviour matches the chord path: same animation,
  same content-swap timing, same paneFlip-bus signal.
  No code duplication — both surfaces call the same
  helper.
* `npm run check` + `npm run build` clean.
* Vitest pin: if the hamburger menu's entries have an
  existing test pattern, mirror it for the two new
  entries.

## How to start

1. Find the Hybrid pane's hamburger menu in
   `web/src/components/Pane.svelte` (lines around 189 +
   the `paneMenu: HamburgerMenu` binding).
2. Audit the existing entries' shape. Pick the natural
   slot for the two new entries — likely near pane-
   level ops (split / close).
3. For the theme toggle: read the current `data-theme`
   from the pane state; render "Light mode" / "Dark
   mode" label as appropriate; on click, flip the
   pane's theme value via the existing setter.
4. For the flip button: call `requestPaneFlip(paneId)`
   (the bus introduced in `fullstack-a-22`). The
   half-rotation animation plays.
5. Hybrid-only gate: wrap both entries in
   `{#if pane.kind === "hybrid"}` (or the equivalent
   pane-type check).
6. Hover tooltips name the chord equivalents (theme:
   whichever chord toggles it today — confirm by
   reading `shortcuts.ts`; flip: `Cmd+. Tab` per
   `fullstack-a-7`).
7. Visual test on lane-A (Hybrid pane with mixed
   content, toggle theme, click flip).
8. Pre-push gate.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.
* @@WebtestB verifies the theme composition on lane-B
  (lane-B already tests per-Hybrid theme overrides
  from `fullstack-b-5`).
* No backend / Rust work in this task.
* Composes with `-22` (flip animation), `-23` (FB dock
  separator), `-24` (rich prompt redesign),
  `-25` (editor toggle → Settings), `-26` (editor
  toolbar parity). All in the same Round-1 detour set.
* Pre-commit `git diff --staged --stat` per
  `feedback-shared-worktree-commits` — `Pane.svelte` is
  touched by multiple tasks (-23 also touched it via
  the ResizeHandle / FB side pane wiring). Catch any
  cross-contamination.

## 2026-05-20 — implementation note

### Theme toggle: relocate, not duplicate

The pre-fix code had a standalone `.pane-theme-toggle`
button in the pane chrome (from `fullstack-59`) that
called `togglePaneTheme()`. @@Alex's spec ("move the
dark/light mode in there") was a relocation, not a
duplication. I removed the standalone button + its
scoped CSS and added a hamburger menu entry that calls
the same `togglePaneTheme()` helper. Both helpers
(`togglePaneTheme`, `paneEffectiveTheme`,
`paneThemeTooltip`) stay — the menu entry's icon +
label + tooltip read from them.

The icon in the menu mirrors the old chrome button:
Sun glyph when the pane is currently in dark mode
(click → switches to light), Moon when in light mode.
Label reads "Light mode" / "Dark mode" — what the click
will produce. Tooltip from `paneThemeTooltip()` adds
the "click to follow global" hint when the pane already
has an override.

### Flip pane: new click affordance for an existing chord

`flipHybrid(paneId)` already existed (`fullstack-15` /
`fullstack-a-7` / `fullstack-a-22` heritage). The chord
path (Cmd+. then Tab inside Hybrid NAV) is unchanged.
The new menu entry adds a mouse-driveable surface that
calls the same helper, so the flip animation
(`fullstack-a-22`) plays identically from either
trigger. After the click, the menu closes via
`closePaneHamburgerMenu()` so the user sees the
animation rather than the menu.

Icon: `FlipHorizontal2` from lucide (the "flip
horizontally" glyph reads as a card-flip cue). Chord
hint in the right-aligned slot reads "Cmd+. Tab" per
the chord-label slot pattern the "Enter Hybrid NAV"
entry already uses.

### Hybrid-only gate

Both entries are wrapped in `{#if pane.back !== undefined}`.
`back` is set on the LeafNode the first time `flipHybrid`
runs (lazy-init), so a never-flipped pane reads as
non-Hybrid → no menu entries. Per the task spec:
non-Hybrid panes don't get these. To make a pane Hybrid
the user still chord-flips (`Cmd+. Tab`); after that
first chord, the menu entries appear.

Trade-off the spec accepts: there's no menu path to
CREATE a Hybrid. Only the chord. Acceptable since the
chord is the canonical entry; the menu is for ongoing
Hybrid operations on already-Hybrid panes.

### Test pin

`perHybridTheme.test.ts` had a pin asserting
`class="pane-theme-toggle"` in the pane source — the
literal CSS class of the standalone button I just
removed. The pin's intent was "the toggle is wired";
its load-bearing piece is the `togglePaneTheme`
function reference, not the chrome-button CSS class.
Updated the pin to drop the chrome-class assertion +
record the relocation, keeping the function-reference
check as the actual contract.

The downstream pins (cycle-behaviour, scheduleSessionSave,
CSS cascade at `:global(.pane[data-theme="dark"])`) are
unchanged — they verify the toggle's behaviour + the
cascade wiring, both of which are preserved.

### Files touched

* `web/src/components/Pane.svelte` — removed the
  standalone `.pane-theme-toggle` button + its CSS;
  added `flipHybrid` + `FlipHorizontal2` icon imports;
  added two new Hybrid-only menu entries (theme +
  flip).
* `web/src/components/perHybridTheme.test.ts` —
  updated the chrome-button pin to a function-reference
  pin reflecting the new menu location.

### Pre-push gate

vitest 501/501 green; `npm run check` 0 errors / 0
warnings; `npm run build` clean.

### Lane-A verification

(post-restart):

1. Open a non-Hybrid pane (e.g., fresh file editor
   tab). Open the pane hamburger. NO theme / flip
   entries appear — only "Enter Hybrid NAV" + the
   focus-border-colour palette.
2. Hit `Cmd+. Tab` inside Hybrid NAV to flip the pane.
   `pane.back` gets lazy-created; pane is now Hybrid.
3. Re-open the hamburger. Two new entries appear
   between "Enter Hybrid NAV" and the colour palette:
   "Light mode" / "Dark mode" (with Sun/Moon icon
   reflecting the click destination) and "Flip pane"
   (with the Cmd+. Tab chord hint).
4. Click "Flip pane" → menu closes + half-rotation
   animation plays from `fullstack-a-22`.
5. Click "Dark mode" / "Light mode" → pane theme
   flips; the icon + label swap to reflect the new
   destination on next open.

### Lane-B verification (per-Hybrid theme composition)

Setup a Hybrid pane with front=dark / back=light per
`fullstack-b-5`. Open the hamburger on the
currently-visible side. Clicking the theme toggle
flips THAT side's theme (not the other side's). The
chord-driven flip then exposes the opposite side with
its independent theme. No regression on the per-side
theme persistence.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Right read on the spec — @@Alex said "move the dark/light
mode in there", which is relocation, not duplication.
Dropping the standalone `.pane-theme-toggle` button + its
CSS + relocating into the hamburger is the literal
fulfillment of "move". The shared helpers (`togglePaneTheme`,
`paneEffectiveTheme`, `paneThemeTooltip`) stay reused so
the menu entry is a thin UI surface over existing logic.

The Sun/Moon icon convention (icon reflects the click
destination, not the current state) reads naturally —
when the user sees the Sun glyph they understand
"clicking lights this up." Matches macOS / iOS / web
conventions for theme-toggle affordances.

Flip-pane click affordance is pure wire-up over the
existing `flipHybrid(paneId)` helper — animation from
`fullstack-a-22` plays identically from either trigger.
Closing the menu after click via `closePaneHamburgerMenu()`
so the user sees the animation rather than the menu
collapsing on it is the right UX detail.

Hybrid-only gate via `{#if pane.back !== undefined}` is
the right shape — `back` is lazy-init'd on first flip,
so non-Hybrid panes naturally don't show the entries.
The "no menu path to CREATE a Hybrid; chord is canonical
entry" trade-off matches the spec's "menu is for ongoing
Hybrid operations" framing.

Test pin update is good engineering hygiene: the prior
pin's CSS-class assertion was incidental to its intent
(toggle is wired); the function-reference check is the
actual contract. Tightening the pin around the load-
bearing piece + dropping the chrome-class incidental
matches the "tests pin contracts, not implementation
details" principle.

Pre-push gate green (vitest 501/501, check 0/0, build).

**Commit clearance**: approved. Suggested commit subject:

```
Hybrid pane hamburger: relocate dark/light toggle + add flip-pane entry (fullstack-a-27)
```

Push waits until end of Round 2.

This was your last Round-1 detour task. Queue empty.
Standby until Round-2 fan-out.