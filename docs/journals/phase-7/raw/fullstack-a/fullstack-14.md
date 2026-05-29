# fullstack-14: Phase 1 — Graph + File Browser as first-class tabs

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Migrate the Graph overlay and the File Browser overlay
from OverlayShells to first-class tabs in the existing
tab system. Each tab carries its own inspector. The
left/right docked side-pane File Browser stays as-is
(it's a separate UI surface inspired by GitHub's tree;
docked panes are unaffected).

Search stays as an OverlayShell. Settings stays as an
OverlayShell. Only Graph and File Browser migrate this
round.

## Relevant links

* @@Alex's UI exploration:
  [../ui-exploration.md](../ui-exploration.md) — Phase 1
  section.

## Acceptance criteria

* Graph tab and File Browser tab are first-class tab
  types alongside Editor and Terminal.
* New tab button / menu / shortcut exposes the new tab
  types (e.g. doc tab menu's "Show in file browser"
  opens a File Browser tab in the current pane, not the
  overlay).
* "Graph from here" actions across various surfaces
  (right-click on directory, file context menu, etc.)
  spawn a Graph tab rather than the overlay.
* Each migrated tab carries its own inspector / detail
  affordance — Graph keeps the Details panel; File
  Browser keeps the path / metadata panel; both reuse
  the existing inspector machinery.
* The OverlayShell variants for Graph + File Browser are
  removed (or kept gated behind a transitional flag
  briefly — your call; prefer removal if reachable).
* The docked left/right side-pane File Browser is
  untouched.
* Search and Settings OverlayShells unchanged.
* Existing "Show File" / "Graph from here" cross-nav
  flows land on a sensible target pane (focused pane,
  new pane if focused is editing, your call).

## Out of scope

* Phase 2 Hybrid pane model (separate task,
  `fullstack-15`).
* New Graph/FileBrowser features beyond what the
  overlays already shipped.

## How to start

1. Inventory the OverlayShell entry points for Graph and
   File Browser. There's likely a single OverlayShell
   component and per-overlay content components.
2. Create new tab types in the tab registry. Reuse the
   existing Editor / Terminal tab machinery — they're
   already polymorphic enough; you'll add new variants.
3. Migrate the OverlayShell-content components to
   tab-content components. The inspector / detail panels
   travel with them.
4. Update all callers ("Graph from here", "Show File",
   tab-creation menus, file-tree right-click) to spawn
   tabs instead of opening the overlay.
5. Smoke test all the entry points; @@WebtestA will run
   the proper walkthrough.

## Hand-off

Standard. Pre-push gate green. Coordinate with @@WebtestA
on the entry-point matrix (file-tree right-click, doc tab
"Show in file browser", graph triggers across surfaces).
Ping via `alex/event-fullstack-architect.md`.

## 2026-05-18 20:45 BST — implementation slice

Implemented the Phase 1 migration locally:

* Added first-class `graph` and `browser` tab variants to the pane
  layout model, including labels, clone/reopen behavior, URL/session
  serialization, and restore.
* Rendered Graph and File Browser surfaces inside panes, with tab-local
  inspector state for each surface.
* Retargeted Graph/File Browser open helpers and app shortcuts/commands
  from OverlayShell flags to tab creation in the active pane.
* Left docked left/right File Browser side panes mounted and unchanged.
* Removed the globally-mounted Graph and File Browser OverlayShell entry
  points from `App.svelte`; Search and Settings overlays remain mounted.

Verification so far:

* `npm run test -- store tabs`
* `npm run check`
* `npm run build`

Known follow-up before landing: run the full gate and smoke the main
entry-point matrix once more before commit.

## 2026-05-18 20:45 BST — commit readiness

Ready to land.

Changed files:

* `web/src/state/tabs.svelte.ts`
* `web/src/components/Pane.svelte`
* `web/src/components/GraphPanel.svelte`
* `web/src/components/FileBrowserSurface.svelte`
* `web/src/App.svelte`
* `web/src/state/store.svelte.ts`
* `web/src/state/tabs.test.ts`
* `web/src/state/store.test.ts`
* `docs/journals/phase-7/fullstack-a/fullstack-14.md`

Verification:

* `npm run test -- store tabs`
* `npm run check`
* `npm run build`
* `scripts/pre-push`

Known risk: the migrated surfaces reuse the existing Graph and File
Browser body components; @@WebtestA should still walk the full
entry-point matrix for focus/menu polish after landing.

Proposed commit: `Migrate graph and file browser into tabs
(fullstack-14)`.
