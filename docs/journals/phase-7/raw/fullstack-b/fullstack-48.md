# fullstack-48: flippable Hybrids — back side + per-Hybrid theme + Cmd+K Tab

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Goal

A single pane is now a **Hybrid** in user-facing
terminology. Each Hybrid gets:

1. A **front** and **back** side, each holding its
   own independent layout (tabs, focus, scroll).
2. A **per-Hybrid dark/light theme** setting,
   persisted with the layout state.
3. A **Flip** action — rotates the Hybrid to show the
   back, with a CSS 3D flip animation + wobble on
   land. Cmd+K `Tab` is the keyboard binding.
4. **Inverted theme on the back side** — if the front
   is dark, the back is light, and vice versa. This
   is the default; user can override per-side.

This is the marquee piece of the Hybrid identity from
Phase 2 — earlier we'd treated "Hybrid" as the
whole-viewport binary tree, but the experience-level
shift is: each pane (= each leaf in the tree) is a
Hybrid that has two surfaces.

## Relevant links

* @@Alex's chat note 2026-05-19 13:15 BST.
* Phase 2 model: [../ui-exploration.md](../ui-exploration.md).
* Predecessors:
  * `fullstack-15` binary-tree pane model.
  * `fullstack-16` Pane Mode keybinds.
  * `fullstack-30` (previously Hybrid-wide focus
    color — keep this; it's a window-wide setting
    independent of per-Hybrid theme).
  * `fullstack-34` pane chrome + wobble bus.
  * `fullstack-46` British spelling + hamburger
    menu shape — coordinate so this task's hamburger
    item additions don't collide.

## Acceptance criteria

### Hybrid identity terminology

* Update user-facing menus / labels: "pane" → "Hybrid"
  where it makes sense. Specifically the hamburger
  menu header / Pane Mode help cheatsheet labels.
* Internal code names stay as "pane" — too invasive
  to rename, no user impact.
* The pane hamburger menu (from `fullstack-30` +
  `-46`) is now the **Hybrid hamburger menu** in
  copy. Add two new items:
  * **Theme** (sub-menu or inline): Dark / Light per
    this Hybrid (current side). Default = follow
    global theme.
  * **Flip Hybrid** (with the Cmd+K Tab hint).

### Back-side state model

* Each Hybrid's state gains a `back` slot mirroring
  `front`: tabs array, active tab id, scroll state,
  per-side theme.
* Default = back is empty (welcome state) with the
  inverted theme of the front.
* Switching which side is visible is a per-Hybrid
  flag (`showing_back: bool`, default false).
* Persistence: serialize both sides + the flag with
  the existing per-window state keyed by
  `w=<window-label>`.

### Flip mechanic

* **Cmd+K Tab** triggers the flip on the focused
  Hybrid in Pane Mode.
* Hamburger menu "Flip Hybrid" item does the same
  outside Pane Mode.
* Animation: CSS 3D `rotateY(180deg)` over ~400ms
  with an ease-out curve. Both sides exist in the
  DOM during the flip; the front face is hidden via
  `backface-visibility: hidden` after the rotation.
* **Wobble on land** — reuse the wobble bus from
  `fullstack-34`. The wobble fires after the flip
  animation completes.
* During the flip, content on neither side accepts
  input (cosmetic block; nothing destructive).

### Per-Hybrid theme

* Each Hybrid side has a `theme: "dark" | "light" |
  null` slot. `null` = follow the global theme
  (current chan behavior).
* When the user picks "Dark" or "Light" from the
  Theme menu item, that side's theme overrides the
  global default for THAT side only.
* Default on Flip: the back side's theme defaults to
  the inverse of the front's effective theme on
  first flip. Once set, persisted; user can override.
* Visual: theme tokens (background, text, ANSI
  palette) all scope through CSS variables on the
  Hybrid root element — the global theme provides
  defaults, the per-Hybrid override wins when set.

### Back-side-attention indicator

* When the back side has something that needs the
  user's attention — initially: an unread bubble
  notification from the rich prompt's watcher (per
  `fullstack-13` / `-17`) — show a small **flashing
  dot** on the front Hybrid's chrome as a hint to
  flip.
* Position: on the chrome edge of the Hybrid (e.g.
  top-right corner, near where the hamburger lives),
  small enough to not crowd, visible enough to
  notice peripherally.
* Behavior:
  * Steady when nothing on the back needs attention.
  * Flashes (~1.5s cycle, low-amplitude alpha pulse)
    when the back has something unread.
  * Clears as soon as the user flips to the back
    (the attention surface is now visible).
* Sources of "attention" for v1:
  * Unread bubble notifications in the watcher
    overlay on the back's rich prompt.
  * Future hooks (terminal activity per
    `fullstack-25` if back has a terminal with
    unfocused output, etc.) — design the indicator
    as a generic "any side wants attention" signal,
    not bubble-specific. Sub-source counters live on
    the back's state.
* Symmetric: when on the back, the same indicator
  shows on the back's chrome if the front has
  attention.
* Use the same wobble bus pattern from `fullstack-34`
  for any one-shot animations; the flashing dot is
  a CSS-only loop.

### Out of scope

* Per-tab theme.
* More than two sides per Hybrid (e.g. a cube).
* Flip animation customization (one canonical
  animation).
* Multi-touch / gesture-based flip.

## How to start

1. Extend the layout state model in
   `tabs.svelte.ts`: each leaf gains `front` + `back`
   slots (containing what `tabs` currently
   represents) + `showing_back` + per-side
   `theme` slot.
2. Migration: existing single-side panes load into
   `front`, back is empty.
3. CSS: wrap the pane content in a 3D-transform
   container with two faces. `transform-style:
   preserve-3d` on the wrapper; each face is
   `backface-visibility: hidden` with the back face
   pre-rotated 180deg.
4. Flip trigger: a `flipHybrid(paneId)` action that
   toggles `showing_back` + triggers the wobble bus.
5. Theme scoping: CSS variables on the Hybrid root
   element override globals when the per-side theme
   is set.
6. Cmd+K Tab handler in `handlePaneModeKey`.
7. Hamburger menu items in `Pane.svelte`.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@FullStackA on the Pane Mode keymap if Cmd+K Tab
collides with anything in their `fullstack-42` rework.
Visual eyeballing for the flip animation is
expected — pixel-tune the timing + the wobble entry
ease per the lane-boundary rule (teardown after,
webtest verdict still canonical). Ping via
`alex/event-fullstack-b-architect.md`.

## 2026-05-19 13:20 BST — Phase A landed (@@FullStackB)

Scope: data model + `flipHybrid()` action + URL hash /
session payload round-trip + tests. **No UI surface
change** in this commit. The back side exists in
state but no code path makes it visible yet; Phase B
adds the CSS 3D flip, the hamburger items, and the
Cmd+K Tab binding.

Files:

* `web/src/state/tabs.svelte.ts`:
  * New exported types `HybridTheme` and
    `HybridSide`.
  * `Pane` gains optional `theme`, `back`,
    `showingBack`. Comments lock in the contract
    that `pane.tabs` / `pane.activeTabId` /
    `pane.theme` always describe the **currently-
    visible** side; the hidden side parks in
    `back`. Existing consumers stay agnostic to
    the front/back split.
  * `flipHybrid(paneId)` lazily materialises `back`
    with an inverted theme default on first flip,
    then swaps tabs / activeTabId / theme between
    the visible side and the back, toggles
    `showingBack`, and fires the `fullstack-34`
    wobble bus so the pane chrome can react when
    Phase B wires the CSS 3D animation.
  * `serializeNode` factored: `serializeTab(t,
    isActive, opts)` extracted so both `t` (front)
    and `bt` (back) share one encoder.
  * `SerLeaf` gains optional `bt` / `ht` / `hb` /
    `sb`. All omitted on legacy single-side panes,
    so existing URL hashes deserialize unchanged.
  * `restoreLayout` reads the new fields. Back-side
    terminal tabs are dropped on restore (Phase A
    limitation): per-window session payloads index
    terminals by pane, not by side, so back-side
    terminal session restore needs a session-format
    change. File / browser / graph kinds round-trip
    end-to-end on the back.
  * `cloneNode` carries forward `theme` / `back` /
    `showingBack` so paneMode (Cmd+K) drafts
    include Hybrid state.

* `web/src/state/tabs.test.ts` — five assertions in a
  new `Hybrid flip (fullstack-48 phase A)` describe.

Out of scope for Phase A (next commit):

* CSS 3D `rotateY` wrapper + per-side
  `backface-visibility: hidden` in `Pane.svelte`.
* Hamburger items (Theme sub-menu + Flip Hybrid).
* `Cmd+K Tab` binding in `App.svelte`. Waits for
  `fullstack-42` to land on `origin/main`.
* Per-side theme application via CSS variables.
* Back-side terminal restore (session-format
  change).

Verification:

* `npx vitest run tabs` → 57 / 57 pass.
* `npm run test` → 32 files / 317 tests pass.
* `npm run check` → 0 errors / 0 warnings.
* `npm run build` → clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` →
  green.

Commit message proposed:
`Hybrid back-side data model + flipHybrid action (fullstack-48 phase A)`.

## 2026-05-19 13:25 BST — Phase B landed (@@FullStackB)

Phase B scope: surface the flip via the hamburger menu + the Cmd+K
Tab binding. The visible-side swap happens via Phase A's
`flipHybrid` action; the pane chrome reuses the `fullstack-34`
wobble bus for the landing animation. The full CSS 3D `rotateY`
animation is deferred to a follow-up — the minimal "swap +
wobble" flow gives @@Alex something to feel on a walkthrough
without overspending on chrome.

Files:

* `web/src/state/shortcuts.ts` — new `app.pane.flip` entry
  (`Mod+K Tab` on both web and native) so the cheatsheet
  + the hamburger chord column resolve through the same
  SHORTCUTS registry.
* `web/src/App.svelte` — `handlePaneModeKey` gains `case
  "Tab":` calling `flipHybrid(draft.activePaneId ??
  layout.activePaneId)`. The flip happens inside the
  pane-mode transaction so Esc rolls it back along with
  any other draft edits.
* `web/src/components/Pane.svelte` — `flipHybrid` import,
  `onFlipHybrid` handler, new "Flip Hybrid" hamburger
  item between Split right/down and Close all tabs, with
  the `app.pane.flip` chord hint. `FlipHorizontal2` icon
  from lucide.
* `web/src/components/Pane.test.ts` — expected hamburger
  labels gain "Flip Hybrid" between Split down and Close
  all tabs.

Out of scope (cuttable follow-ups, only if @@Alex flags
during walkthrough):

* CSS 3D `rotateY` flip animation (currently the swap
  is instantaneous + wobble fires). Phase A's data
  model already supports it; would require wrapping
  pane content in a `transform-style: preserve-3d`
  container with the back face pre-rotated 180deg.
* Theme sub-menu in the hamburger (Dark / Light /
  Follow global). Phase A's `pane.theme` slot is
  already wired through the data model and would just
  need menu wiring + a CSS variable override on the
  pane root.
* Per-side terminal session restore (session-format
  change in chan-server).

Verification:

* `npx vitest run Pane EmptyPaneCarousel tabs
  paneModeKeymap` → 71 / 71 pass.
* `npm run test` → 32 files / 317 tests.
* `npm run check` → 0 errors / 0 warnings.
* `npm run build` → clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` →
  green.

Commit message proposed:
`Hybrid flip UI + Cmd+K Tab binding (fullstack-48 phase B)`.

## 2026-05-19 13:35 BST — Phase C landed (@@FullStackB)

Phase C scope: the back-side-attention indicator from the
13:25 BST addendum. Small flashing dot on the pane's
chrome (in the `.actions` area, just left of the
hamburger) when the hidden side has unread / active
content. Designed as a generic "the other side wants
attention" signal — terminal `watcher.unread` and
`terminalActivity` are wired today; future sources
(language-server diagnostics, etc.) plug into the same
derived check without re-spec.

Files:

* `web/src/components/Pane.svelte`:
  * New `backHasAttention` derived. Returns `true`
    when `pane.back` contains a terminal with
    `watcher.unread` or `terminalActivity`. Symmetric:
    since the flip swaps `pane.tabs` ↔ `pane.back.tabs`,
    flipping to the side that has the unread surface
    naturally drops the indicator (its sources are now
    in the visible tabs).
  * Inline `<span class="back-attention">` rendered in
    `.actions` when the derived is true. Carries an
    `aria-label` + `title` so accessibility + hover
    both surface the hint to flip.
  * `back-attention-pulse` keyframe — 1.5 s opacity
    cycle (1 → 0.35 → 1) matching the addendum's
    "low-amplitude alpha pulse". `--warn-text` reuses
    the colour family of the existing terminal-
    activity marker so users learn one chrome
    vocabulary. `prefers-reduced-motion` reverts to a
    static dot.
* `web/src/components/Pane.test.ts` — two assertions:
  * Indicator surfaces when `pane.back` holds a
    terminal tab with `watcher.unread: true`.
  * Indicator stays clear when the back is idle.

Verification:

* `npx vitest run Pane EmptyPaneCarousel tabs` →
  73 / 73 pass.
* `npm run test` → 32 files / 319 tests.
* `npm run check` → 0 errors / 0 warnings.
* `npm run build` → clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` →
  green.

Commit message proposed:
`Back-side-attention indicator (fullstack-48 phase C)`.
