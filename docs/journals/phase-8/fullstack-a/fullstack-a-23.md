# fullstack-a-23: Drop the visible idle separator on the docked file-browser resize handle

Owner: @@FullStackA
Date: 2026-05-20

## Goal

The vertical bar painted by `ResizeHandle.svelte` between
the docked file-browser tree and the editor pane should NOT
be visible in the idle state. The 4 px hit area + the
drag-resize behaviour stay; only the visible
`background: var(--separator)` paint goes.

## Background

@@Alex flagged 2026-05-20 (with a screenshot) that the
docked FB shows a vertical separator line that doesn't
belong there visually. They want the element kept (so
drag-resize keeps working) and only the visual paint
removed.

Source of the visible bar:
`web/src/components/ResizeHandle.svelte` — the shared
component used by `FileBrowserSidePane.svelte` (FB dock,
both sides). The relevant CSS:

```css
.handle {
  position: relative;
  width: 4px;
  flex-shrink: 0;
  background: var(--separator);   /* ← idle paint */
  cursor: col-resize;
  touch-action: none;
  transition: width 0.1s, background 0.1s;
}
.handle:hover {
  width: 6px;
  background: var(--separator-hover);  /* ← hover paint */
}
```

`ResizeHandle` is shared (today only FB dock uses it; the
component doc mentions "future left-side panel" + file
editor inspector + graph details as theoretical reuse).
Changing the idle paint globally would affect any current
or future consumer.

## Implementation shape (Option A — locked by @@Alex 2026-05-20)

Per-instance opt-out via a new `idleVisible?: boolean`
prop on `ResizeHandle` (default `true` to preserve
current behaviour for any other consumer).
`FileBrowserSidePane.svelte` passes `idleVisible={false}`
to both its `ResizeHandle` instances (left + right dock).

When the prop is `false`:

* Idle `.handle` paints `background: transparent` (or
  `background: none` — pick what reads cleanest in the
  scoped CSS; the 4 px width + `flex-shrink` + cursor
  + touch-action all stay).
* Hover `.handle:hover` keeps the 6 px width +
  `var(--separator-hover)` paint as the discovery
  affordance.

When the prop is `true` (default), behaviour is
unchanged.

The shared component stays shared; the per-consumer
control lives on the prop. If a future consumer wants the
same invisible-idle behaviour, they pass the prop too.

## Acceptance criteria

* Docked file-browser dock (both `side: "left"` and
  `side: "right"`) shows NO visible vertical line in the
  idle state.
* Hovering over the 4 px hit area still expands it to
  6 px + paints `--separator-hover` (the user gets a
  visible cue on hover that the handle is draggable).
* Drag-resize behaviour unchanged: pointer-down + drag
  still resizes the panel; `setPointerCapture` still
  works; `paneWidths.browser` persists.
* `cursor: col-resize` still surfaces when the user
  hovers the 4 px hit area (the cursor itself is the
  fallback discovery affordance even if the hover paint
  feels too subtle).
* No regression on the visible separator anywhere else
  in the app (if you pick Option B, audit the other
  consumers if any exist; today there shouldn't be).
* Both light and dark theme.
* `npm run check` + `npm run build` clean.

## How to start

1. Open `web/src/components/ResizeHandle.svelte` and the
   `FileBrowserSidePane.svelte` site that mounts it.
2. Pick Option A or B per the sketch above.
3. Visual check on lane-A: dock the file browser, view
   the boundary — no visible line idle; hover → 6 px
   visible cue; drag works.
4. Sanity sweep: any other place that uses `ResizeHandle`
   today? Grep for the import; if there are other
   consumers, they decide whether to opt out too (Option
   A) or just inherit (Option B).
5. Pre-push gate.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.
* No backend / Rust work in this task. Pure SPA / CSS.
* Independent of the other detour tasks (-21 / -22); can
  land in any order.

## 2026-05-20 — implementation note

Followed Option A exactly as the task locked it: per-instance
opt-out via a new prop on `ResizeHandle`. Default keeps the
existing visible-separator behaviour for the other two
consumers (file editor inspector and Graph details inspector,
both via `Inspector.svelte` and `GraphPanel.svelte`).

### `ResizeHandle.svelte`

* New prop `idleVisible?: boolean` with default `true` — so
  every existing consumer keeps its current paint without
  needing to opt in.
* Class binding `class:invisible-idle={!idleVisible}` on the
  handle root.
* Scoped CSS: `.handle.invisible-idle { background: transparent; }`
  overrides the base `background: var(--separator)`. The 4 px
  width, `cursor: col-resize`, `touch-action: none`, and the
  pointer event handlers stay untouched. Hover still expands to
  6 px + paints `var(--separator-hover)` so the discovery
  affordance is preserved.

Considered using `data-idle-visible="false"` instead of a class,
but the existing component already pairs class bindings with
scoped CSS (no data-attribute pattern in this file); kept the
local style consistent.

### `FileBrowserSidePane.svelte`

Both `ResizeHandle` instantiations (left-dock + right-dock) gain
`idleVisible={false}`. Re-formatted to multi-line attribute
style so the new prop fits without an 80-col blowup.

### Other ResizeHandle consumers — verified intact

* `Inspector.svelte` line 66 / 68 — file editor inspector. No
  `idleVisible` prop passed → inherits the `true` default →
  unchanged.
* `GraphPanel.svelte` line 39 import + (consumer site) — Graph
  details inspector. Same default-inherit path → unchanged.

The shared component stays shared; the per-consumer opt-out lives
on the prop. Any future consumer that wants the same invisible-
idle behaviour passes `idleVisible={false}`.

### Files touched

* `web/src/components/ResizeHandle.svelte` — new prop +
  conditional CSS variant.
* `web/src/components/FileBrowserSidePane.svelte` — two
  `idleVisible={false}` adds + multi-line attribute formatting.

### Pre-push gate

vitest 481/481 green (no regression; ResizeHandle is structural
DOM-only, no test pin needed); `npm run check` 0 errors / 0
warnings; `npm run build` clean.

### Lane-A verification

(post-restart so the rebuilt binary picks up the bundle):

1. Dock the file browser on either side (Cmd+. > or
   Cmd+. <). The boundary between the FB tree and the
   editor pane shows NO visible vertical line in the idle
   state.
2. Hover over the 4 px hit area at the boundary — handle
   expands to 6 px + paints `var(--separator-hover)` as the
   discovery cue. Cursor turns `col-resize`.
3. Press + drag — the dock width changes; release commits to
   `paneWidths.browser` and persists through reload.
4. Open the file editor inspector (cog icon on the editor
   pane) or the Graph view's details panel — those inspectors
   still show their visible separator line in the idle state
   (the inherit-default path).
5. Repeat in dark + light theme; transparent stays transparent
   in both.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Option A locked + landed cleanly. The new
`idleVisible?: boolean` prop (default `true`) preserves
every other consumer's current paint via inherit; the
two FB side-pane instantiations opt out via
`idleVisible={false}`. Audit of Inspector.svelte +
GraphPanel.svelte consumers confirmed they inherit the
default → no regression for the other surfaces.

The `class:invisible-idle` + scoped `.handle.invisible-idle
{ background: transparent; }` pattern is the right
local-CSS shape (matches the existing class-binding
convention in the file; data-attribute would have been
the odd one out). Hover still expands to 6px +
`var(--separator-hover)` so the discovery affordance is
preserved; `cursor: col-resize` stays as the fallback
cue. All four pointer-event handlers untouched →
drag-resize behaviour identical.

Pre-push gate green (vitest 481/481, check 0/0, build
clean).

**Commit clearance**: approved. Suggested commit subject:

```
FB dock: drop visible idle paint on the resize handle via per-instance idleVisible prop (fullstack-a-23)
```

Push waits until end of Round 2.

After this commits: pick up `-24` (rich prompt redesign)
and `-25` (editor toggle → Settings) which I cut while
you were detour-clearing. Both in your queue per the
2026-05-20 dispatch poke above.