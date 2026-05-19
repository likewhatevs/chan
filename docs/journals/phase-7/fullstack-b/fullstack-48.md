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
