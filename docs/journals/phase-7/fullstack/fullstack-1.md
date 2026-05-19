# fullstack-1: file browser side panes

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Add a docked, vertical file-browser pane that sticks to the
left or right edge of the screen, top to bottom, *outside* the
main pane area where editor and terminal tabs live. The user
can pin one on each side simultaneously. The current overlay
file-browser stays available.

## Relevant links

* [../request.md](../request.md) Enhancements (top of the
  Enhancements list).
* Visual reference (GitHub-style left tree):
  `../image.png#w=250` in the request thread.
* [../../agents/fullstack.md](../../agents/fullstack.md)
  for skill context.

## Acceptance criteria

* File-browser surface gains two new toggles: "Stick to left"
  and "Stick to right". Each is independent.
* When stuck, the file browser renders as a vertical pane
  attached to the screen edge, full window height, persistent
  across editor/terminal tab switches. Width is resizable.
* Both sides can be stuck simultaneously; the main pane area
  shrinks to fit between them.
* The overlay file browser (current behavior) is still callable
  even when one or both side panes are active.
* Look-and-feel stays consistent with the rest of chan's UI;
  GitHub's left tree informs *layout*, not skin.
* Side-pane pinned state survives reload (persisted in
  preferences via `crates/chan-server/src/preferences.rs` or
  the equivalent client-side store).
* No regression in pane drag-and-drop or tab management inside
  the main pane area.

## Out of scope

* Find buffer fixes (separate task — fullstack-3).
* Style toolbar parity (separate task — fullstack-2).
* Any other file-browser content changes (sort, group, etc.).

## How to start

1. Read the existing file-browser components under
   `web/src/components/` (likely `FileBrowser*`, plus the
   overlay host).
2. Decide whether the side pane is a sibling layout
   component (preferred) or a state mode on the existing
   component.
3. Propose the layout in an append to this task file before
   coding if the choice is non-trivial; @@Architect signs off
   via event.

## Hand-off

When ready for walkthrough, append a "Specialist review
requested" entry and file `alex/event-fullstack-architect.md`
(type `poke`) with a link to your latest append.

## 2026-05-18 11:29 — Layout proposal

Inspected `web/src/App.svelte`, `web/src/components/FileBrowserOverlay.svelte`,
`web/src/components/FileTree.svelte`, `web/src/components/Workspace.svelte`, and
`web/src/state/store.svelte.ts`.

Proposal:

* Extract the reusable browser body from `FileBrowserOverlay.svelte` into a
  shared component that can render either as overlay chrome or as dock chrome.
  Keep `FileTree`, the details inspector, find bar, context menu, create/search/
  graph actions, and selection behavior shared so overlay and docked panes do
  not drift.
* Add `browserSidePanes` state with independent `left` and `right` booleans in
  `store.svelte.ts`; persist it through server preferences alongside existing
  pane widths. Reuse `paneWidths.browser` for the first pass unless Architect
  wants independent left/right widths.
* Render the docked panes as siblings of `<main><Workspace /></main>` in
  `App.svelte`, inside a new full-height app layout:
  `left browser | main workspace | right browser`. This keeps the panes outside
  the editor/terminal split tree and naturally shrinks the main pane area.
* Add "Stick to left" and "Stick to right" menu items to the file browser
  surface. The overlay remains openable regardless of dock state. Docked panes
  get compact header controls to unstick that side and can still open the
  overlay for the full inspector workflow.
* Preserve drag/drop and tab behavior by leaving `Workspace.svelte` and
  `tabs.svelte.ts` unchanged; only its containing flex area changes width.

Open question: should left and right docked browsers have independent widths, or
is the existing global browser width acceptable for both sides in round 1?

## 2026-05-18 — @@Architect sign-off

Layout proposal approved as drafted. Proceed with implementation.

Decision on the open question: **single global browser width** for round 1
(reuse `paneWidths.browser` for both sides). Reasoning:

* Round-1 scope is "ship the side-pane affordance"; independent widths add a
  second preferences slot + a second resize handle without buying enough.
* If side-by-side asymmetry becomes a real ergonomic complaint after we click
  around, that's a follow-up enhancement, not a blocker.

Two soft constraints to keep in mind:

* The overlay file browser must remain reachable even with both sides pinned.
  The existing keyboard shortcut / menu entry stays; verify on a walkthrough.
* `App.svelte` layout change should keep the workspace centered when neither
  side is pinned (zero layout shift in the default case).

Note on path: I renamed the phase dirs from
`chan-pre-release-phase-N` to `phase-N` after you bootstrapped. The paths in
your earlier append are stale; new paths under `docs/journals/phase-7/...`.

## 2026-05-18 11:38 — Specialist review requested

Implemented docked file-browser side panes for round-1 review.

Files changed:

* `crates/chan-server/src/preferences.rs`
* `crates/chan-server/src/routes/preferences.rs`
* `crates/chan-server/src/lib.rs`
* `web/src/App.svelte`
* `web/src/api/types.ts`
* `web/src/state/store.svelte.ts`
* `web/src/components/FileBrowserOverlay.svelte`
* `web/src/components/FileBrowserSurface.svelte`
* `web/src/components/FileBrowserSidePane.svelte`

Behavior:

* File browser menu now has independent "Stick to left" and "Stick to right"
  actions.
* Docked panes render outside the workspace split tree and shrink the central
  editor/terminal area. Both sides can be active together.
* Docked panes share the existing browser width (`paneWidths.browser`) per
  Architect decision and resize through the existing `ResizeHandle`.
* Dock state persists through `browser_side_panes` in server preferences.
* Overlay file browser remains available through the existing Files shortcut
  and through "Open overlay" from a docked pane.

Verification:

* `npm run check`
* `npm run build` (passes with existing chunk-size warnings)
* `npm run test`
* `cargo test -p chan-server preferences`

Known gaps:

* No manual browser walkthrough yet. Needs Webtest or Alex click-through to
  check exact menu placement, drag resizing, and tab drag/drop ergonomics with
  both sides pinned.

## 2026-05-18 — @@Architect review: APPROVED, walkthrough queued

Implementation accepted. Files-changed list and verification chain
(`npm run check` + build + test + cargo preferences test) read clean. The
shared `FileBrowserSurface` extraction is the right factoring — keeps overlay
and dock from drifting.

Walkthrough is being routed to @@WebtestA as a new task `webtest-a-2`. They
will exercise: menu placement, drag resize, both-sides-pinned tab D&D, overlay
reachability with one or both sides pinned, no-layout-shift when neither side
is pinned. Findings will append back here.

Hold a commit until the walkthrough is clean.

Proceed to `fullstack-2` (unified style toolbar) now. The walkthrough on
`fullstack-1` runs in parallel and won't block your next task.

## 2026-05-18 14:00 BST — @@Architect: walkthrough cleared, commit cleared (gated on @@Alex)

@@WebtestA finished `webtest-a-2`: all 8 acceptance items PASS. Detail at
[../webtest-a/webtest-a-2.md](../webtest-a/webtest-a-2.md). Highlights:

* Menu actions exist, single + both-side pin, overlay reachability with one
  or both sides pinned, zero layout shift baseline, persistence through
  reload, side-pane false-positive D&D rejection, shared resize handle —
  all green.

Two non-blocking follow-ups noted (NOT regressions of `fullstack-1`, file
later):

* Default docked width 466px on a 1440px viewport leaves a tight middle
  column. Worth a smaller default (~280-320px). Out of round-1 scope; I'll
  capture as a small @@FullStack follow-up.
* Resize handles lack `role="separator"` / `aria-orientation` / keyboard
  handler. A11y follow-up.

**Out-of-scope regression** surfaced during walkthrough — workspace tab
D&D (NOT side-pane): dragging an active tab onto an adjacent inactive
tab in the same tablist removes the active tab from the list. Repros
twice. Likely pre-existing or unrelated to `fullstack-1` (which didn't
touch `tabs.svelte.ts`). Cutting as `fullstack-5` — fix can land before
or after `fullstack-1`'s commit; not a hold.

### Commit clearance

**APPROVED from @@Architect's side.** Per project rule, only @@Alex
authorizes the commit. Hold the commit until @@Alex says go.

Proposed commit message (uses your earlier shape, refined):

```text
Add docked file-browser side panes

Add "Stick to left" / "Stick to right" menu actions on the file browser
surface, plus left/right dock containers outside the workspace split
tree. Both sides can be pinned simultaneously and share the existing
paneWidths.browser preference. Overlay remains reachable when one or
both sides are pinned. Dock state persists through server preferences.
```
