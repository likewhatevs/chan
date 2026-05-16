# @@Frontend task 9: Scope-aware depth-slider cap for the Graph overlay

Owner: @@Frontend. Status: REVIEW. Depends on: nothing
(backend-3, frontend-7, backend-4 already in REVIEW so the
graph data the slider reads is stable).

Relates to journal work item **G3** in
[[chan-pre-release-phase-2/journal.md]].

## Goal

Cap the Graph overlay's depth slider at a value that makes sense for
the active scope, so the user can't ask for hops or sub-tree levels
that would expand to nothing.

## Relevant links

- [[chan-pre-release-phase-2/request.md]] Graph section item 1
  ("On scope of single file, the depth slider shouldn't go beyond
  1; on scope of N files, shouldn't go beyond N; on folders,
  depends on how many sub-folders; whole drive should know the
  max-depth").
- [[chan-pre-release-phase-2/journal.md]] decision 6 (cap source
  is the existing fs-graph payload / loaded nodes; no new
  backend route).
- `web/src/components/GraphPanel.svelte` (`DEPTH_MAX = 10`,
  slider mounting, `currentScope`).
- `web/src/components/GraphCanvas.svelte` (BFS that consumes
  `graphOverlay.depth`).
- `crates/chan-server/src/routes/fs_graph.rs` (`MAX_DEPTH = 6`,
  `truncated`).

## Behavior to implement

The slider's `max` attribute is dynamic:

* **file scope** (`currentScope.kind === "file"`): max = 1. The
  slider stays enabled so the user sees the affordance, but it
  pins to 1.
* **group scope** (`kind === "group"`): max = the number of files
  in the group (`currentScope.paths.length`), clamped to the
  current `DEPTH_MAX = 10` ceiling.
* **dir scope** (`kind === "dir"`): max = the deepest sub-folder
  level reachable from this directory. Read this from the
  fs-graph payload GraphPanel already loads for filesystem mode
  (`graphOverlay.mode === "filesystem"`); for content mode,
  derive the same depth client-side by walking the loaded
  `nodes` and counting the maximum number of `/`-segments past
  the dir path.
* **drive scope** (`kind === "drive"`) and **global scope**
  (`kind === "global"`): max = the drive's max folder nesting,
  derived once per overlay open by calling
  `/api/fs-graph?scope=folder&path=&depth=6` (the existing route
  caps depth at 6; the response's `truncated` flag tells us when
  the cap was hit, in which case the slider's max becomes 6).
* **tag scope** and **git_repo scope**: keep current behaviour
  for now (slider disabled at drive / global today; tag / repo
  still mean "BFS this many hops"). The current `DEPTH_MAX = 10`
  ceiling stays.

The slider's current value is clamped to the new max whenever the
scope changes. Lowering the cap should reduce the value to fit,
not refuse the scope change.

## Acceptance criteria

1. file scope: slider max is 1; value pins to 1 on scope entry.
2. group of N files: slider max is min(N, 10); value clamps to
   that max on scope entry.
3. dir scope: slider max equals the deepest sub-folder under the
   directory (1 means "directory has no sub-folders"); value
   clamps to that max on scope entry.
4. drive scope: slider max equals the drive's max folder nesting
   from the fs-graph probe; if the probe response is `truncated:
   true`, max is the route's `MAX_DEPTH` (6).
5. tag / git_repo scopes: unchanged behaviour from today.
6. The slider stays visible in every scope so the affordance does
   not disappear; the `disabled` state mirrors today's rule
   (drive / global remain disabled because their BFS does not
   key on hops).

## Test expectations

* Add Vitest coverage for a pure helper that returns the cap
  given a scope + the loaded nodes + the optional fs-graph
  probe response.
* `cd web && npm test -- --run`.
* `cd web && npm run check`.

## Hardening / review expectations

* @@Architect reviews the cap derivation rules against the
  request text before commit.
* @@Webtest extends the CDP smoke (see
  [[chan-pre-release-phase-2/webtest-2.md]]) to verify the
  slider max changes across at least file, group, dir, drive
  scopes.

## Progress notes

* 2026-05-16 @@Frontend: implemented scope-aware cap derivation in
  `web/src/graph/depth.ts` with Vitest coverage.
* 2026-05-16 @@Frontend: wired `GraphPanel` slider `max` and
  value clamping to the derived cap. Drive / global scopes now use
  a one-shot `fsGraph(folder, "", depth=6)` probe and keep the
  existing disabled affordance.

## Completion notes

* Files changed: `web/src/components/GraphPanel.svelte`,
  `web/src/graph/depth.ts`, `web/src/graph/depth.test.ts`,
  `chan-pre-release-phase-2/frontend-9.md`.
* Verification:
  * `cd web && npm test -- --run depth`
  * `cd web && npm test -- --run`
  * `cd web && npm run check`
* @@Webtest browser smoke:
  * [[chan-pre-release-phase-2/webtest-smoke.mjs]] now verifies depth caps for
    file, group, directory, and drive scopes.
  * `node chan-pre-release-phase-2/webtest-smoke.mjs`: pass.
