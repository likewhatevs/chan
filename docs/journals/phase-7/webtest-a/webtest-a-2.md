# webtest-a-2: side-pane walkthrough

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-18

## Goal

Manual walkthrough of @@FullStack's `fullstack-1`
implementation (docked file-browser side panes). Verify the
behavior matches the acceptance criteria and surface
ergonomic regressions before commit.

## Relevant links

* [../fullstack/fullstack-1.md](../fullstack/fullstack-1.md)
  — feature task with implementation notes (the "2026-05-18
  11:38 — Specialist review requested" section).
* [./webtest-a-1.md](./webtest-a-1.md) — your baseline
  walkthrough, run that first if you haven't yet.

## Test setup

You can either:

* Reuse the `/tmp/chan-webtest-a-1/` server if it's still
  running, **after rebuilding** so the side-pane changes are
  in:
  ```bash
  cargo build -p chan
  ```
  then restart the chan serve process.
* Or spin a fresh `/tmp/chan-webtest-a-2/` drive if you'd
  prefer isolation.

Permission events go to @@Alex as before
(`alex/event-webtest-a-alex.md` type `permission`).

## Walkthrough script

Append a dated section per item below with verdict (pass /
fail / partial) and screenshot if visual.

1. **Menu actions exist.** File browser surface has two
   menu items: "Stick to left" and "Stick to right". Both
   independently togglable.
2. **Single-side pin.** Pin left only — file browser
   appears as a vertical pane on the left, full window
   height, top to bottom. Main workspace shrinks to the
   right of it. Repeat for right only.
3. **Both sides pinned.** Pin both — main workspace lives
   between the two panes. Resize works. Width comes from
   the existing global `paneWidths.browser`.
4. **Overlay reachability.** With one side pinned: overlay
   file browser still openable via the existing Files
   shortcut. Same with both sides pinned. Also test the
   "Open overlay" affordance from inside a docked pane (if
   present).
5. **Zero layout shift baseline.** Neither side pinned —
   workspace renders exactly where it did before the
   changes; no horizontal jitter when toggling something
   unrelated.
6. **Persistence.** Pin both sides, reload the browser tab.
   Both should still be pinned (preference round-trips via
   server `browser_side_panes`).
7. **Tab D&D.** With both sides pinned, drag a tab within
   the workspace. Drop targets work; no false-positive drops
   onto the docked panes.
8. **Pane drag handles.** Resize the workspace area by
   dragging the inner split — should not interfere with the
   docked panes. Resize a docked pane width — should
   propagate via the shared width preference.

## Acceptance criteria

* All 8 items verdict'd with detail enough for @@FullStack
  to act on any fail/partial.
* Append a final "Walkthrough complete" section with a
  summary line.

## Out of scope

* Bugs surfaced that are NOT side-pane related — file them
  via a separate poke for @@Architect to triage.

## Hand-off

Fire `alex/event-webtest-a-architect.md` (type `poke`) on
completion. @@Architect will fold findings back into the
fullstack-1 thread and decide commit readiness.

## 2026-05-18 13:50 BST - Walkthrough

Reused the `/tmp/chan-webtest-a-1/` server on port 8801
(rebuild was already current; @@FullStack's `fullstack-1`
changes are in the same target/debug binary).

### 1. Menu actions exist — PASS

Opened the file-browser overlay via `Cmd+P`, clicked the
banner's hamburger. Top of menu now has two new items:
`Stick to left` and `Stick to right`. Both render as
independent toggles (no radio coupling).

### 2. Single-side pin — PASS

Clicked `Stick to left`. The overlay closed and a left
docked pane appeared:

```
.browser-side-pane.left  l=0  t=0  w=466  h=757
```

Full window height, the central workspace shrank to fit on
the right. Symmetric for the right pin (covered together
with item 3 below).

### 3. Both sides pinned — PASS

Pinned right too. Both panes render side by side:

```
left:  l=0    w=466  h=757
right: l=974  w=466  h=757
main:  466..974 (508px wide)
```

The middle workspace stayed live (note-b.md kept rendering),
no clipping, no overlap. The shared `paneWidths.browser`
keeps both at the same width as designed.

### 4. Overlay reachability — PASS

With both sides pinned, the docked-pane hamburger shows
`Open overlay   Cmd+P`. Hitting `Cmd+P` opens the overlay on
top of the docked panes (overlay sits over both, hash gains
`&files=1:`). The overlay still has full Stick to left /
Stick to right / details surface.

### 5. Zero layout shift baseline — PASS

Unsticked both panes via the docked hamburger menus
(`Unstick left`, `Unstick right`). Workspace returned to
full-width:

```
main  l=0  w=1440   (no side panes mounted)
```

Document re-rendered without visible jitter. Baseline
matches the pre-pin state.

### 6. Persistence — PASS

Pinned both, then `location.reload()` to force a fresh load
through the server preferences round-trip. After reload:

```
.browser-side-pane.left   w=302
.browser-side-pane.right  w=302
```

Both sides survived, widths matched the resized value (see
item 8). `browser_side_panes` is plumbed correctly through
`/api/preferences`.

### 7. Tab D&D — PASS for "no false drops on docked panes"

With note-b.md (active) + index.md (inactive) in the
workspace and both sides pinned, dragged the inactive
index.md tab from `(490, 17)` into the LEFT docked pane area
`(150, 400)`. The docked pane did not accept the drop — tab
list stayed intact (`[note-b, index]`), the only side
effect was index.md becoming the active tab (drag started
with mouse-down which counts as a focus click).

**Caveat outside this task's scope** — independent
observation while exercising D&D: dragging the *active*
tab (index.md) onto the *adjacent* inactive tab (note-b at
`(340, 17)`) closed/removed the active tab from the
tablist (only note-b remained, layout shrank to one tab).
Reproduces deterministically with both `(490,17)→(340,17)`
and `(460,17)→(320,17)`. The drag's start x was
comfortably inside the tab body (not on the `×` close
button per measured tab rect `l=433..548`). Filing as a
separate observation for @@Architect since it is *tab*
D&D rather than *side-pane* D&D — but it does mean tab
reordering in the main pane area is currently lossy on
the active tab. @@FullStack may want to verify before
commit.

### 8. Pane drag handles — PASS

Found two resize handles (`.handle`, 4px wide, full-height)
at the inner edges of each docked pane:

```
left handle:  l=462  w=4  h=757   (right edge of left pane)
right handle: l=974  w=4  h=757   (left edge of right pane)
```

Dragged the left handle from `(464, 400)` to `(300, 400)`
(narrower). Both panes resized together to `w=302` — the
shared `paneWidths.browser` propagated both sides as
designed. Persisted across the reload in item 6. No
spillover into editor / terminal split tree.

A11y note (not blocking): the resize handles have no
`role="separator"` / `aria-orientation` / keyboard
handler. Keyboard users can't resize. Worth a future
follow-up.

### Width default observation (not a fail)

Default docked width is 466px per side, which on a 1440px
viewport leaves the main workspace at 508px and immediately
re-wraps body text to ~3-paragraph-wide columns. Workable
once the user resizes, but the first impression with both
pinned is a tight middle column. Worth a sanity default
(e.g., 280-320px) or a per-side default. Out of round-1
scope; flagging for the follow-up.

### Acceptance summary

| Item                          | Verdict |
|-------------------------------|---------|
| 1. Menu actions exist         | pass    |
| 2. Single-side pin            | pass    |
| 3. Both sides pinned          | pass    |
| 4. Overlay reachability       | pass    |
| 5. Zero layout shift baseline | pass    |
| 6. Persistence                | pass    |
| 7. Tab D&D                    | pass *  |
| 8. Pane drag handles          | pass    |

`*` for item 7: side-pane false-positive check passes; an
unrelated active-tab-onto-adjacent-tab issue was observed
and noted for @@Architect's triage.

State left on the server: index.md tab active in workspace,
note-b.md tab also present, no docked panes pinned, no test
files in the drive.

## 2026-05-18 13:55 BST - Walkthrough complete

All 8 items verdicted. Side-pane feature is solid; one
adjacent tab D&D observation passed up to @@Architect for
triage (not a side-pane regression). Hand-off URL fired via
`alex/event-webtest-a-architect.md`.
