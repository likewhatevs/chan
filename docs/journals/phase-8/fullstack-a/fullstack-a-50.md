# fullstack-a-50 — Graph overhaul G3: directory nodes + FB-style inspector with aggregated reports stats

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 + Tasks B/C/E/F + systacean-15 + Task F)

## Goal

Make directory nodes first-class graph entities. Clicking
a directory opens an FB-style inspector body with
aggregated chan-reports stats for that directory.

## Background

Locked design at
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
§"Architecture overhaul" — G3. @@Alex 2026-05-21:

> whatever we are plotting, we always start from a
> parent directory which if we click we get an
> inspector for that directory like we get in the file
> browser, with the directory aggregated stats for
> chan-reports

The FB-style inspector is the same shape as the file-
browser side panel today (directory name + path + a
body of metadata + actions). Body contents for the v1
inspector:

* Directory path + name.
* File / subdirectory count.
* Aggregated stats from chan-report (file count by
  bucket markdown / source / binary / media; SLOC by
  language; COCOMO summary scoped to this directory).
* "Graph from here" action (re-root the graph at this
  directory).

## Acceptance criteria

* Directory nodes are clickable + open the inspector
  body.
* Inspector renders the aggregated stats from
  `systacean-15`'s new chan-report endpoint.
* "Graph from here" action on a directory inspector
  re-roots the graph at that directory.
* Tests cover: inspector mount on directory click;
  aggregated stats render correctly; "Graph from
  here" re-rooting behaviour.
* Pre-push gate green.

## How to start

1. Confirm `systacean-15` cross-dir aggregation API is
   live + accessible from the SPA.
2. Confirm Task F's chan-reports toggle exists + can
   be ON (without the toggle ON, the aggregated stats
   aren't computed; inspector body shows a
   "Enable chan-reports in FB settings to see
   aggregated stats" placeholder).
3. Design the inspector body shape — mirror the FB
   side-panel rendering pattern.
4. Wire the "Graph from here" action to the existing
   re-rooting code path (from `-a-33`).
5. Tests.

## Coordination

* SPA-primary; consumes `systacean-15`'s API.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereqs

* `fullstack-a-43` (Task A) — directory node
  rendering depends on the back-side architecture
  shape.
* Tasks B/C/E/F (Hybrid back-side wave) — Task F
  brings the chan-reports toggle which gates
  aggregated stats.
* `systacean-15` (cross-dir aggregation API).
* G2 (`-a-49`) — directory nodes need the
  filesystem-hierarchy spine to render meaningfully.

## Numbering

This is `-a-50`. See `-a-45` for broader wave
numbering note.

## 2026-05-21 — ready for review

Six-file change. SPA-only; no Rust touched. All
prereqs in HEAD (`-a-43` Task A, `-a-45..-a-48`
Tasks B/C/E/F, `systacean-15` cross-dir
aggregation, `-a-49` G2 hierarchy spine).

### What landed

`web/src/api/client.ts`:

* New `api.reportDir(path)` calls `/api/report/dir`
  (the O(1) cache endpoint from `systacean-15`).
  Returns the same `ReportPrefix` shape as
  `reportPrefix` but reads from the maintained
  cache instead of walking the file map. 404 when
  the directory has no tracked files.

`web/src/components/InspectorBody.svelte`:

* `InspectorSelection` discriminated union extended
  with `{ kind: "directory"; path: string; label?: string }`.
* Dispatch branch added: directory selection
  routes to `<DirectoryInfoBody>` with `path`,
  `label`, `onSetAsScope`, `onClose` props.

`web/src/components/DirectoryInfoBody.svelte` (new):

* Fetches `api.reportDir(path)` on path change via
  `$effect`. Tracks `loading` / `error` / `report`
  state.
* 404 handling: treats `/404/.test(message)` (or
  `not found` regex) as "no chan-report data yet"
  → renders the empty-state hint mentioning the
  chan-reports toggle (`-a-48`) in the Hybrid FB
  back-side. Non-404 fetch failures surface the
  error message.
* Displays: kind chip (`DIR`), directory name
  (label OR path OR "Drive root"), monospaced
  path row, "Graph from here" button (when
  `onSetAsScope` is wired), Totals section
  (files / SLOC / comments / blanks), By-language
  table (name / files / SLOC), COCOMO section
  (effort / schedule / developers / cost).
* `formatNumber` + `formatCurrencyUSD` helpers
  use `Intl.NumberFormat` for tabular numerics.

`web/src/components/GraphPanel.svelte`:

* `inspectorSelection` derived: `selectedNode.kind
  === "folder"` maps to the new directory
  inspector selection. The kind is `"folder"`
  (not `"directory"`) because GraphPanel normalises
  chan-server's `"directory"` wire kind into
  `"folder"` at data-load time (the
  `RenderedNode` type is narrowed to
  `"file" | "tag" | "mention" | "language" | "folder"`).
  Both surfaces map to the same selection shape.
* `<InspectorBody onSetAsScope={...}>` re-wired for
  directory selections: when the user clicks
  "Graph from here" on a directory, the handler
  calls `rescopeFromHere(\`dir:${inspectorSelection.path}\`)`
  using the existing `-a-33` re-rooting code path.
  Non-directory selections still skip the prop
  (the breadcrumb covers them; matches the
  `-a-33` rule).

`web/src/components/DirectoryInfoBody.test.ts`
(new): 10 raw-source pins for the wiring shape
(api.reportDir route, InspectorSelection variant,
InspectorBody dispatch, fetch effect, all 3
sections rendered, Graph-from-here button gated
on onSetAsScope, 404 fallback, drive-root display
name, GraphPanel folder → directory mapping,
GraphPanel onSetAsScope wired to
rescopeFromHere).

`web/src/components/revealBrowserActions.test.ts`:

* Existing `-a-33` pin "GraphPanel does not pass
  onSetAsScope on any InspectorBody" rewritten:
  the inverse pin under `-a-50` asserts the
  directory-only `onSetAsScope` wiring exists.
  Non-directory selections still don't pass the
  prop. Comment block expanded to call out the
  `-a-33` → `-a-50` evolution.

### Decisions

* **Where the data comes from**: `api.reportDir`
  (the O(1) cache from `systacean-15`) rather
  than `api.reportPrefix` (walks the file map).
  The cache endpoint is the right call for
  on-click inspector body — it's faster + the
  data is fresh per the cache's maintained
  invariant.
* **404 handling**: directories with no tracked
  files surface as a "no chan-report data yet"
  affordance pointing the user at the
  `chan-reports` toggle in the Hybrid FB
  back-side. The toggle is opt-in (default ON
  per `-a-48` option B, but the chan-server
  indexer can still skip empty directories).
  Better UX than a hard 404 error.
* **`kind: "folder"` matched, not `"directory"`**:
  The SPA normalises chan-server's
  `"directory"` wire kind into `"folder"` at
  data load (see GraphPanel.svelte lines
  ~957/958 + ~1039/1041). The `RenderedNode`
  type narrows to `"folder"`. Matching
  `"folder"` is type-safe + covers the
  current data path.
* **Inspector header chip "DIR"**: simple text
  badge using the `--g-folder` background
  variable, matching the canvas folder colour.
  Other inspectors use `KindChip` (which has
  its own kind enum); the new "directory" kind
  doesn't have a `KindChip` mapping yet.
  Inline text chip avoids extending KindChip
  in this commit; flag as a future polish if
  the visual mismatch reads off.
* **"Graph from here" semantic**: uses the
  existing `rescopeFromHere(scopeId)` helper
  from `-a-33`. Scope id format
  `dir:<path>` matches the breadcrumb's
  derivation. Re-roots in place (depth resets
  to 1, selection clears) — same UX as
  clicking a breadcrumb ancestor.

### Gate

* vitest **668 / 668** (+10 net from `-a-49`'s
  658).
* svelte-check 0 errors / 0 warnings across
  3992 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Atomic-audit-commit discipline

Per the `feedback-atomic-audit-commit` memory
rule that emerged from the `5685be4` incident,
this commit will use a single bash invocation
chaining `git add` + `git diff --staged --stat`
+ `git commit -m "..."` + `git show --stat HEAD`.
No inter-command race window for peer agents to
inject staged stowaways.

### Suggested commit subject

```
Graph directory inspector + chan-reports aggregated stats (fullstack-a-50)
```

Single commit. API + InspectorSelection +
component + GraphPanel wiring + tests are
tightly coupled.

### Files for `git add`

* `web/src/api/client.ts`
* `web/src/components/DirectoryInfoBody.svelte`
* `web/src/components/DirectoryInfoBody.test.ts`
* `web/src/components/InspectorBody.svelte`
* `web/src/components/GraphPanel.svelte`
* `web/src/components/revealBrowserActions.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-50.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

Push held — multi-agent tree commit discipline.
Standing by for clearance.
