# frontend-12: graph overlay scope navigation polish

Owner: @@Frontend
Status: REVIEW for dir "Graph from here"; breadcrumb PARKS to phase 6.1 per
[architect-4](./architect-4.md).

## Goal

Originally two scope-navigation improvements; split during the
phase-6 wrap:

1. **Breadcrumb** to walk "back up" from a deep scope.
   **PARKED to phase 6.1**: useful UX add but not blocking the
   architectural close-out.
2. **"Graph from here" on directory nodes** in the inspector.
   **LANDS this phase**: this is a real scope-action gap (file
   variant works, directory variant doesn't), and it closes the
   "filesystem as primary graph layer" ask end-to-end.

## Background

`/api/graph` scope is rooted at `drive | directory | file` + `path`,
with `depth` controlling forward fan-out from there. There is no
"ancestor depth" param; going backward = re-pivoting the scope.

Today the user can re-scope by clicking a parent node in the
graph (if it happens to be rendered) or by jumping back through
the file tree. There is no first-class affordance in the overlay
for stepping up.

## Scope

* Render a breadcrumb in the graph overlay header (or just under
  the scope picker / chip filter row, wherever fits the existing
  layout). Shape:

  ```
  drive  /  notes  /  sub  /  deep
  ```

  Each segment is a clickable link that re-scopes to that path.
  `drive` always at the left; the current scope is the last
  segment and is not clickable (or is rendered with a distinct
  style).
* The breadcrumb only renders for scopes deeper than `drive`.
  At drive scope there is nothing to step up to.
* For `scope=file`: segments are the parent directories of the
  file plus the file basename. Clicking the file basename is a
  no-op (you're already there); clicking a parent re-scopes to
  that directory.
* When the user re-scopes via the breadcrumb, `depth` resets to
  the overlay's default (or the previous user-set depth, the
  frontend picks the less surprising option). Hash state updates
  so a refresh lands on the new scope.

### "Graph from here" on directory nodes

* Today: `web/src/components/GraphPanel.svelte` around line 1139
  gates `onSetAsScope` on `fsKind === "file"`, so a directory
  node in the graph inspector has no "Graph from here" button.
  The reasoning in the comment ("user is already inside the
  graph") doesn't hold when the user is at drive scope and
  wants to drill into a specific directory.
* Change: extend the gate to also fire for directory nodes,
  setting the scope to that directory (e.g.,
  `graphOverlay.scopeId = "dir:" + fsPath` or whatever the
  dir-scope id format is, matching the file tree row's
  "Graph from here" action). Depth resets to the overlay
  default on re-scope, same as the file variant.
* Keep the existing "Show Directory" button (it opens the dir
  in the file browser); the new "Graph from here" is additive.

## Out of scope

* New backend route surface; the breadcrumb + new button are
  frontend-only.
* Showing siblings in the breadcrumb (no dropdown per segment).

## Relevant links

* Request follow-up: Alex's clarification on graph depth model
  (forward fan-out only; ancestor traversal is re-scope).
* Backend route: [backsystacean-9](./backsystacean-9.md)
  (merged `/api/graph` scope params).
* Source: `web/src/components/GraphPanel.svelte`,
  `web/src/state/scope.svelte.ts`.

## Acceptance criteria

* Breadcrumb renders for scopes deeper than `drive`.
* Clicking a segment re-scopes the overlay to that ancestor
  path; the graph re-renders.
* Hash state reflects the new scope so reload lands there.
* Current-scope segment is visually distinct (not a link).
* Directory nodes in the graph inspector now show a
  "Graph from here" button alongside "Show Directory". Click
  re-scopes the graph to that directory; depth resets per the
  same rule as the file variant.

## Tests

* Vitest covering: segment derivation from a path, click handler
  emitting the right scope, hash state update.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.

## Review and hardening

* @@Frontend self-review for hash-state interaction with
  [frontend-4](./frontend-4.md)'s scope pivot work.
* @@Webtest live click-around: from a deep scope, walk the
  breadcrumb back to drive; verify the graph re-renders at each
  hop.

## Progress notes

* 2026-05-18: Landed the in-scope half. Filesystem graph inspector
  now offers "Graph from here" for directory nodes as well as files.
  The action switches filesystem graph scope to `dir:<path>` or
  `file:<path>`, resets depth to 1, and keeps the selected node
  pending for the refreshed graph.
* 2026-05-18: Added `scopeFsGraphFromHere` store helper with Vitest
  coverage for file and directory pivots.

## Completion notes

Ready for review for the phase-6 landing half. Breadcrumb remains
parked by plan. Validation:

* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build` (passes with existing Vite chunk-size,
  ineffective dynamic import, and plugin timing warnings)
