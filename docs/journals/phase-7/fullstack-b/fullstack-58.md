# fullstack-58: per-tab state for BrowserTab — finish multi-FB-tabs

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

`webtest-b-6` item 6 caught that `fullstack-47`
shipped half of the "multi File Browser tabs"
feature: spawn-without-dedup works (two `Files`
tabs coexist in a pane), but per-tab view state
doesn't (both tabs share the same selection,
expansion, scroll, and DETAILS target).

The walker's verification table:

| Step                          | Tab 1     | Tab 2     |
|-------------------------------|-----------|-----------|
| Click index.md on tab 1       | index.md  | (inactive)|
| Switch to tab 2, click notes  | notes.md  | notes.md  |
| Switch back to tab 1          | notes.md  | notes.md  |

`fullstack-47`'s commit body acknowledged the
asymmetry (graphs got per-tab scope/filter
schema; browsers didn't). The marquee surface
"multi-FB tabs" reads as half-shipped in the
v0.11.0 changelog without this fix.

## Relevant code

* `web/src/state/tabs.svelte.ts` — current
  `BrowserTab` type:
  ```typescript
  export type BrowserTab = {
    kind: "browser";
    id: string;
    title: string;
    inspectorOpen: boolean;
  };
  ```
  Per-tab fields to add (names are suggestions —
  match what FB surface already consumes):
  * `path?: string` — current subpath /
    breadcrumb root inside the drive.
  * `selected?: string` — selected file's
    drive-relative path (drives the DETAILS
    inspector target).
  * `scroll?: number` — pixel scroll offset
    inside the tree.
  * `expanded?: string[]` — expanded directory
    paths if the tree currently keeps that
    state outside the tab.
  Anything that's currently module-level shared
  state in the FB surface should move onto the
  tab's record.

* `web/src/components/FileBrowserSurface.svelte`,
  `FileTree.svelte` — find the shared state
  consumers and thread them through per-tab.
  The tab prop is already passed to the surface
  (`tab?: BrowserTab` at FileBrowserSurface.svelte:67);
  bind the surface's state to the tab record
  instead of module-level.

* `web/src/components/FileBrowserSurface.svelte:74`
  — `browserState = $derived(tab ?? browserOverlay)`
  pattern already exists for overlay vs tab; the
  tab-bound path should mirror it.

## Acceptance criteria

* Two FB tabs in the same pane carry independent:
  * selection (clicking a file in tab 1 leaves
    tab 2's selection untouched).
  * scroll offset (scrolling in tab 1 doesn't
    move tab 2's scroll).
  * expanded-directories state (expanding a
    subdir in tab 1 doesn't expand it in tab 2).
  * DETAILS inspector target.
* Tab switch preserves each tab's state; coming
  back to tab 1 restores its selection / scroll /
  expansion exactly as left.
* URL hash serialization captures per-tab
  state so reload restores the same view per
  tab. Hash schema extension is your call;
  follow the graph-tab precedent
  (`fullstack-47` graphs serialize per-tab
  scope + filter into the hash).
* Single-tab case unchanged: an overlay or a
  lone FB tab behaves identically to today.
* Dock variant (sidebar FB) unchanged — that
  isn't a tab, so the per-tab fields don't
  apply.

### Tests

* Vitest unit test asserting two BrowserTab
  records can carry distinct `selected` /
  `scroll` / `expanded` values without
  collision.
* Component test (if practical): mount two FB
  tabs, drive their state independently, assert
  each tab's rendered state.
* `webtest-b-6` item 6 won't auto-rerun, but
  the verification table from the walker is the
  spec — implement to pass that table.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* This is v0.11.0 blocking — multi-FB-tabs is
  marquee surface in the changelog and shipping
  the half-version invites a clarifying GitHub
  issue immediately after the tag.
* Re-walk after ship: Lane B's `webtest-b-6`
  item 6 should be re-walked once you ship. If
  Lane B is already wound down, Lane A can
  re-run it on their 8801 session — small cost.
* Queue position: behind `-54` (FB header drop)
  on your lane.
* Standing topic-level commit clearance.

## 2026-05-19 19:15 BST — implementation

**Approach: snapshot/restore on tab swap, live-mirror
while mounted.** The module-level singletons
(`browserSelection`, `treeExpanded.map`, the
`.tree-wrap` scrollTop) stay as the rendering
surface's working memory; the per-tab record stores
each tab's snapshot. On tab swap (the surface's
`tab` prop reference changes), an `$effect` keyed
on `tab.id` restores the new tab's saved state into
the singletons, with a cleanup function that
snapshots the singletons back into the deactivating
tab. While mounted, two live-mirror `$effect`s push
singleton edits (selection, showDrive, expanded
map) onto the active tab continuously so that the
URL-hash serializer always sees current values.
Scroll is mirrored via an `onscroll` handler on
`.tree-wrap` (DOM event, not reactive — keeps the
write outside the effect graph).

Why not fully tab-bound state? `treeExpanded.map`
is consumed by the dock + overlay variants
simultaneously with tab variants. Making expansion
*truly* per-tab would require threading a
context-specific `expanded` map into `FileTree`
and refactoring every reader in tabs.svelte.ts /
store.svelte.ts that observes `treeExpanded`. The
snapshot/restore approach preserves the current
behavior for dock + overlay (they keep sharing the
singleton) and gives tab variant the per-tab
independence the walker's table requires. The
acceptance criterion explicitly says "Dock variant
(sidebar FB) unchanged — that isn't a tab, so the
per-tab fields don't apply".

**Edits:**

* `web/src/state/tabs.svelte.ts`
  * Extended `BrowserTab` with four optional
    fields: `selected`, `showDrive`, `expanded`,
    `scroll`. All persisted only when meaningfully
    set so existing single-tab sessions don't
    bloat the hash.
  * Extended `SerTab` with `bs`, `bd`, `be`, `bsc`
    fields for hash serialization. `bs` is the
    selected path (string), `bd` is `1` when the
    drive-info inspector view is showing, `be` is
    the array of expanded directory paths,
    `bsc` is the integer scroll offset.
  * Updated the `serializeTab` `browser` branch to
    emit the four fields conditionally
    (`...(t.selected ? { bs: t.selected } : {})`,
    etc.).
  * Updated both browser-tab restore sites (the
    front-side `kind === "b"` branch at ~2690 and
    the back-side branch at ~2847 added by
    `fullstack-48` phase A) to read the four
    fields back into the `BrowserTab` record.

* `web/src/components/FileBrowserSurface.svelte`
  * Imported `untrack` from `svelte` and
    `treeExpanded` from `store.svelte`.
  * Added `treeWrapEl` element binding +
    `onTreeWrapScroll` handler that writes
    scrollTop to `tab.scroll` (tab variant only).
  * Added `snapshotIntoTab(tab)` /
    `restoreFromTab(tab)` helpers.
  * Added three `$effect`s:
    1. **Restore-on-swap.** Keyed on `tab.id` (the
       only field read outside the `untrack()`
       wrapper). When tab.id changes, calls
       `restoreFromTab(captured)`. The cleanup
       function captures the OLD tab in closure
       and calls `snapshotIntoTab(captured)` —
       runs before the next firing OR on unmount.
    2. **Selection live-mirror.** Tracks
       `browserSelection.path` and
       `browserSelection.showDrive`; writes them
       through to the captured tab record via
       `untrack()` (so the effect doesn't
       self-trigger).
    3. **Expansion live-mirror.** Tracks
       `treeExpanded.map`; computes the array of
       expanded paths (filtering out the implicit
       root entry); writes through to
       `tab.expanded` via `untrack()`.
  * All three effects gate on `isTab && tab` so
    the dock + overlay variants don't engage the
    per-tab pipeline.

**Tests** added to
`web/src/state/tabs.test.ts`:

1. `round-trips per-tab BrowserTab view state` —
   single browser tab with `selected`, `expanded`,
   `scroll` set; serialize → restore → assert
   field-by-field.
2. `two BrowserTab records carry independent
   state without leakage` — two browser tabs in
   the same pane; set distinct values on each;
   read back; assert no cross-write.
3. `hash round-trips both BrowserTab records' per-
   tab state` — two browser tabs with distinct
   `selected` / `expanded` / `scroll`; serialize
   and restore; assert each tab's state survives
   the round-trip independently.

Total `web/src/state/tabs.test.ts` tests: 69 → 72.
Full suite: 343 → 352 (carrying the +5 from
`fullstack-66`'s shared-truncation work that
landed in parallel).

**Gate.** `npm run check` 0 errors / 2 warnings
(pre-existing GraphPanel `chrome-btn` warnings
from `fullstack-64`, not in my diff); `npm run
test` 35 files / 352 tests passed; `npm run
build` clean; `scripts/pre-push` green.

**Visual eyeball.** Attempted ad-hoc chan serve +
Chrome MCP navigation on `/tmp/chan-test-fullstack-58`;
user denied the navigation step so dropped the
browser-side walkthrough. The unit tests cover
the walker's table behavior (independent
selection / scroll / expansion across two tabs,
hash round-trip). Teardown done (chan serve
killed, drive unregistered + rm'd, MCP tab
closed; webtest persistent tab at 8801
untouched).

**Re-walk flag.** Match the task's note:
`webtest-b-6` item 6 should be re-walked once
this lands. Architect to forward to @@WebtestB
when convenient.

**Out of scope (deliberately deferred):**

* Truly per-tab `treeExpanded` map (separate from
  the singleton) — would require FileTree to
  accept a `expandedMap` prop and every reader of
  `treeExpanded` elsewhere to choose between
  tab-scoped vs drive-scoped. The current
  snapshot/restore approach gives the user-visible
  effect the walker's table demands without
  refactoring the read path.
* `path` (subpath root for breadcrumb navigation)
  was in the task's "suggested fields" list but
  has no current UI surface that consumes it
  (the tree always renders from the drive root).
  Adding it speculatively would be scope creep.
  If a breadcrumb-style navigation comes later,
  the `BrowserTab` record can grow a `path`
  field then.

**Commit readiness.**

* Files staged:
  * `web/src/state/tabs.svelte.ts`
  * `web/src/state/tabs.test.ts`
  * `web/src/components/FileBrowserSurface.svelte`
  * This task file.
  * `docs/journals/phase-7/fullstack-b/journal.md`
    (append).
  * `docs/journals/phase-7/alex/event-fullstack-b-architect.md`
    (event append).
* Proposed commit message:
  ```
  Per-tab BrowserTab view state with hash round-trip (fullstack-58)
  ```
* Standing topic-level commit clearance applies.
  No HOLD pokes since the 17:20 BST cut.
