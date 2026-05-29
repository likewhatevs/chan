# fullstack-64: drop Graph tab maximize + scope selector, title from selected element

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged two pieces of useless chrome on
the Graph tab plus a title quality issue:

1. **Maximize button** — overlay-only affordance
   from when Graph lived as an overlay. With
   Graph as a first-class tab now, maximize is
   meaningless.
2. **Scope selector dropdown** — `scopeOptions`-
   driven dropdown that lets the user pick a
   scope. Discoverability dead end; the
   canonical way to set a graph's scope is the
   "Graph from here" inspector action +
   `Cmd+K 3` context-aware spawn. The dropdown
   is chrome no one uses.
3. **Tab title** — currently renders
   `Graph: <scopeId>` (e.g. `Graph: file:foo/bar.md`
   or `Graph: drive`). Should instead be the
   basename of whatever the user clicked to
   open the graph — file basename, dir name,
   contact name, hashtag, etc.

This also **supersedes `fullstack-57`** (the
GraphPanel scope-reset deferral bug). That bug
existed because `GraphPanel.svelte` validated
`scopeId` against `scopeOptions` on mount. With
the selector gone, the validation goes too,
which removes the reset path entirely.
@@FullStackA: skip `-57` from your queue;
this task covers the cleanup.

## Relevant code

* `web/src/components/GraphPanel.svelte:1019-1027`
  — maximize button. Drop the whole `<button>`
  block. Drop `Maximize2` import + the
  `overlayMaximized` / `setOverlayMaximized`
  imports if no other consumer remains in this
  file (audit; the maximize state may also be
  read elsewhere for overlay sizing — leave
  the state machinery alone, just drop this
  consumer).
* `web/src/components/GraphPanel.svelte:1031-1040`
  — scope selector `<select class="scope-select">`
  + its options loop. Drop.
* `web/src/components/GraphPanel.svelte:77-80`
  — `scopeOptions` derived + `selectedScope`
  derived. Drop if unreferenced after the
  selector goes.
* `web/src/components/GraphPanel.svelte` —
  any mount effect that resets `scopeId` based
  on `scopeOptions` membership (the
  `fullstack-57` bug surface). Drop the
  validation; `scopeId` stays whatever the
  caller set it to.
* `web/src/components/GraphPanel.svelte:1368-1379`
  — `.scope-select` CSS. Drop.
* `web/src/components/GraphPanel.svelte:1380+`
  — maximize chrome CSS comment. Drop the
  related rules.
* `web/src/state/tabs.svelte.ts:799` —
  `graphTitle(mode, scopeId)` helper. Currently
  computes `Graph: <scopeId>`. Replace with a
  basename-extracting helper (see Title
  resolution below).
* `web/src/state/tabs.svelte.ts:245-260` —
  `GraphTab` type. May need a `selectedLabel`
  or `originName` field if the caller needs to
  pass through the human-readable name
  separately from `scopeId`. Or compute from
  `scopeId` (extract basename of file: / dir:
  prefixes).

## Title resolution

Given a `scopeId`, derive the displayed title
as:

* `scopeId === "drive"` → `"drive"` (or
  `"Drive Graph"` if "drive" alone reads too
  thin; your call).
* `scopeId.startsWith("file:")` → basename of
  the file path (e.g. `file:foo/bar/baz.md`
  → `baz.md`).
* `scopeId.startsWith("dir:")` → basename of
  the dir path (`dir:foo/bar` → `bar`, or
  `bar/` to signal directory-ness).
* `scopeId.startsWith("contact:")` → contact
  name (everything after `contact:`).
* `scopeId.startsWith("tag:")` → `#<tag>`
  (everything after `tag:`, prepend `#`).
* Other prefixes — extract the human-readable
  payload after the first `:`. Fallback to
  the raw `scopeId` if no prefix matches.

If a caller wants to override the derived
title (e.g. `paneModeOpenGraph` passing
`opts.title` already), let that win — keep
the explicit `title` field on `GraphTab` as
the override path.

Internally, the existing call sites
(`paneModeOpenGraph`, the `openGraphInPane`
helpers) should keep passing `scopeId`; the
title-rendering layer derives from it on the
fly OR caches into `tab.title` at spawn time.
Pick whichever is less code churn — if the
existing pattern always sets `title` at spawn,
keep that pattern and just compute from
`scopeId` at spawn instead of from the old
`graphTitle()` helper.

## Acceptance criteria

* Graph tab header has no maximize button.
* Graph tab header has no scope-selector
  dropdown.
* Graph tab header still has whatever else
  belongs there (filter chips, find, hamburger,
  etc.) — only the two flagged items go.
* Graph tab title in the tab strip + in any
  inspector / window title contexts reads the
  basename of the scope:
  * Drive scope → `drive` (or whatever short
    label you pick).
  * File scope → file basename.
  * Dir scope → dir basename (with trailing
    `/` if you want).
  * Contact / tag scopes → human-readable name
    with `#` prefix for tags.
* Existing `scopeId` plumbing still works —
  the graph still renders the correct
  subgraph; only the UI surfaces around it
  change.
* `paneModeOpenGraph` context-aware spawn
  (per `fullstack-43`) still works:
  * Spawning from a doc → graph titled after
    the doc basename, scope `file:<path>`,
    inspector pops on mount.
  * Spawning from a FB selection → graph
    titled after the selection's basename.
  * Spawning from a focused graph → carries
    the existing scope, titled accordingly.
* No regression on existing graph behaviour —
  filter chips, find, double-click navigation,
  inspector all unchanged.

### `fullstack-57` supersession

Confirm at PR / commit message: this task
covers what `-57` was meant to fix.
GraphPanel no longer resets `scopeId` because
the `scopeOptions` consumer (the dropdown) is
gone. `-57` task file stays in the repo as
audit trail; mark it as **superseded by `-64`**
in your implementation note here.

### Tests

* Vitest: title derivation function tested
  per scope prefix shape (drive / file: / dir:
  / contact: / tag: / unknown).
* Component test: rendered Graph tab does NOT
  contain a `.scope-select` element or a
  maximize button.
* Regression test: opening a graph with
  `scopeId = "file:foo.md"` and pre-existing
  `pendingSelectId` still pops the inspector
  on mount (the `fullstack-57` test surface,
  same expectation).

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0 blocking — Graph is marquee surface
  (carousel slide 3, context-aware spawn,
  multi-Graph tabs, etc.) and the chrome
  trim + better title both feed the release
  story.
* Re-walk cost: light. The Graph tab chrome
  was covered in `webtest-a-9` item 1 (multi
  Graph tabs) — that walk's verdict stands;
  this task removes chrome not behaviour.
  Re-walk if Lane A is still up; otherwise
  spot-check at tag time.
* Queue position: takes `-57`'s slot on Lane
  A; updated queue is
  `-55` → `-56` → `-64` → `-61`. `-57` is
  superseded; don't process it.
* Standing topic-level commit clearance.

## 2026-05-19 16:47 BST — @@FullStackA implementation note

Implementation:

* `tabs.svelte.ts` `graphTitle()` reshaped to
  return the basename of the selected element
  per the title-resolution map. Drive / global
  → `drive`. `file:foo/bar.md` → `bar.md`.
  `dir:foo/bar` → `bar/`. `tag:#foo` → `#foo`
  (and `tag:foo` → `#foo` for missing-prefix
  edge case). `contact:alice` → `alice`.
  `git_repo:project/chan` → `chan`. Unknown
  prefixes peel the payload after the first
  colon. `mode === "language"` keeps its
  dedicated `Languages` label since it's a
  top-level lens rather than a per-scope view.
  New `graphScopeBasename` helper kept private
  to the module; `graphTitle` now exported so
  tests can hit it directly.
* `GraphPanel.svelte`: dropped the maximize
  button (lines 1018-1028 in pre-edit, the
  whole `<button class="chrome-btn">` block)
  and the scope-selector dropdown (lines
  1029-1042 — `<span class="scope-label">` +
  `<select class="scope-select">`).
  `Maximize2` + `Minimize2` imports gone;
  `overlayMaximized` + `setOverlayMaximized`
  imports gone; `doToggleOverlayMaximized`
  helper gone.
* `GraphPanel.svelte`: dropped the
  `$effect` that snapped `scopeId` to
  `defaultScopeId()` when `currentScope` was
  null. That's the `fullstack-57` bug surface
  — context-aware spawn (`fullstack-43`) lands
  `file:`/`dir:` scopes that aren't always in
  `availableGraphScopes()`, and the snap-back
  clobbered them. Replaced with a
  `synthesizeScope(scopeId)` fallback inside
  `currentScope`'s derivation: when the
  options-listing lookup misses, synthesize a
  matching `ScopeOption` from the prefix
  (file:/dir:/tag:/git_repo: + drive / global)
  so `currentScope.kind` still resolves and
  the downstream BFS / chip logic carries on.
  Group scopes (no canonical synthesis) fall
  through to whatever `scopeOptions` returns.
* CSS: dropped `.scope-label`, `.scope-select`,
  `.scope-select:focus`, and the orphaned
  `.chrome-btn` / `.chrome-btn:hover` rules
  (the only consumer was the dropped maximize
  button; the file-browser surface dropped its
  `chrome-btn.close` in `fullstack-54`).
* `onGraphContextMenu`: removed the
  `.scope-select` selector from the
  "don't hijack right-click" gate (the element
  is gone).

`fullstack-57` **superseded by this commit**.
The validator-against-`scopeOptions` snap-back
is removed; context-aware spawn's
`file:foo.md` survives the mount.

Tests added:

* `tabs.test.ts`: 8 `graphTitle` cases (drive,
  file:, dir:, tag:, contact:, git_repo:,
  language mode override, unknown prefix
  peel + raw fallback).
* `revealBrowserActions.test.ts`: GraphPanel
  source asserts — no `<Maximize2`, no
  `<Minimize2`, no `doToggleOverlayMaximized`,
  no `class="scope-select"` / `class="scope-label"`,
  no `graphState.scopeId = defaultScopeId()`,
  and yes the new `synthesizeScope(...)` call.

Gate green:

* `npm run test -- revealBrowserActions tabs`
  (95 passed),
* `npm run test` (363 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: open a file →
Cmd+K 3 → graph spawns with `file:<path>`
scope and pre-selected node; chrome bar
should show only the filter chips + the
hamburger; tab label should be the file
basename (e.g. `foo.md`); inspector opens on
mount per `fullstack-32`.

Proposed commit message:

> Trim Graph chrome + basename-derived title (fullstack-64)
>
> Drop the overlay-era maximize button + the scope-
> selector dropdown from the Graph tab chrome. Graph
> tab title now derives from the scope's selected
> element: file basename, dir basename + slash,
> #tag, contact name, git_repo basename, language
> mode keeps the dedicated label. Adds a
> synthesizeScope() fallback in GraphPanel so a
> context-aware spawn's file:/dir: scope survives
> mount even when not in availableGraphScopes() —
> supersedes fullstack-57 by removing the snap-back
> $effect entirely. CSS for the dropped chrome
> elements swept.
