# fullstack-b-6: Scope FB watcher to selection (pull phase-7 backlog item 9 into Round 1)

Owner: @@FullStackB
Date: 2026-05-19

## Goal

The docked / dock-side / tab file browser currently refreshes its
tree on **any** change anywhere in the drive. Alex hit this live:
drive active in unrelated paths (`crates/`, `web/`), FB showing
only `tasks/` expanded → tree rebuilds every few seconds. Scope
each FB instance's watcher to the visible subtree so out-of-scope
activity stops triggering re-renders.

This is the phase-7
[`next-phase-backlog.md`](../../phase-7/next-phase-backlog.md)
item 9, promoted from Round 2 → Round 1 because the pain is
current.

## Spec direction

* When the FB tab has a `selected` path that resolves to a
  directory → watcher scope is that directory.
* When the `selected` is a file → watcher scope is the parent
  directory.
* No selection → watcher scope is the drive root (current
  behaviour, no change).
* Watcher attach is per-tab. Each tab attaches its own scoped
  watcher (per-tab BrowserTab state from phase-7 `fullstack-58`
  carries the selection key).
* Switching tabs / changing selection → detach + re-attach to
  the new scope.
* Closing the tab → detach the watcher (don't leak chan-server
  watcher state).

Strict-scope reading: only watch the selected dir. Expanded
sibling subtrees stay rendered at their last-known state until
the next refresh — accept that trade-off initially. Visible-
scope (watch selected + every user-expanded directory) is a
second-phase nicety; ship strict first.

## Background

Background and edge cases captured in phase-7 backlog item 9.
Specifically:

* `crates/chan-drive`'s watcher API may need a subscribe-by-
  prefix extension. Audit `crates/chan-server/src/event_watcher.rs`
  + the chan-drive watcher surface used by FB (NOT the rich-
  prompt event watcher) — different subsystem.
* `systacean-19` from phase 7 constrained the watcher to the
  drive root; this task extends that discipline inward.
* Search index continues watching the whole drive (background
  process). The carousel's indexing-graph slide also stays
  drive-wide. This task only affects the FB tree-render
  subscription.

## Acceptance criteria

* Steady-state docked / docked-side / tab FB with no activity
  in the scoped subtree is silent — no re-renders.
* Real changes inside the scoped subtree still propagate
  (touch a file → tree updates; create dir → tree updates).
* Switching the FB's selection re-points the watcher; closing
  the tab detaches it.
* No leaked watcher state in chan-server after FB tabs come
  and go.
* Coordinate with @@FullStackA's `fullstack-a-5` (survey
  bubble fix) only if you both end up editing
  `crates/chan-server/src/event_watcher.rs`; different
  watcher subsystem expected, but worth a quick check.

## How to start

1. Reproduce against your lane-B test server: spin up `chan
   serve` on a drive, dock the FB, expand one subdir,
   simulate writes in an unrelated path (e.g. `crates/`),
   confirm the tree rebuilds.
2. Find the FB's current watcher subscription path
   (`web/src/components/FileBrowser*.svelte` + the chan-drive
   watcher API it talks to).
3. Add subscribe-by-prefix in chan-drive if missing.
4. Wire the SPA to attach per-tab scoped watchers; detach on
   selection change / tab close.

## 2026-05-19 - Implementation landed (pre-commit)

Diagnosed and shipped the strict-scope version.

Root cause of the visible bug: `onWatchEvent` in
`web/src/state/store.svelte.ts` called `refreshTree()`
unconditionally on every filesystem event. `refreshTree()`
re-fetches the root listing and reassigns `tree.entries`
wholesale, which propagates re-renders through every consumer
(File Browser overlay + per-pane FB tabs + PathPromptModal
suggestions). When the drive is being walked by an external
process (an indexing pass over `crates/`, a build dropping files
into `web/dist/`) the FB visibly shakes once per event.

Fix shape — kept entirely on the SPA, no server change:

* Each FB instance (overlay + per-pane browser tabs) contributes
  a watcher scope derived from its current selection:
  - no selection → `""` (drive root, watch everything)
  - selection is a directory → that dir
  - selection is a file → its parent dir
* `onWatchEvent` now snapshots the union of scopes
  (`activeFbScopes`) and only calls `refreshTreeForPath` when an
  event path lands in at least one scope. Events outside every
  scope skip the tree refresh entirely.
* `refreshTreeForPath` re-fetches only the parent dir of the
  changed path (via `api.list(parent)` + `mergeDirEntries`) and
  no-ops when that dir isn't already loaded. The full
  `refreshTree()` remains for bootstrap + manual reloads.
* No server-side changes. The chan-server WS stream stays
  single-channel and unscoped; per-tab filtering happens in the
  SPA. This avoids the chan-drive subscribe-by-prefix work the
  task originally floated.

Limitation flagged in the journal: `tree.entries` is still
shared across FB instances, so two FBs with different scopes
both see a refresh when EITHER scope matches the event. A true
per-FB tree would need a much larger refactor; the immediate
"shipping strict first" win is "no FB flicker when no FB's
scope intersects the event".

Files changed:

* `web/src/state/store.svelte.ts`:
  - Replaced the unconditional `void refreshTree()` +
    `scheduleDriveRefresh()` block in `onWatchEvent` with a
    scope-aware path: collect event paths, intersect with
    `activeFbScopes()`, call `refreshTreeForPath()` per hit.
  - Added `fbScopeForSelection`, `activeFbScopes`,
    `pathInAnyScope`, `refreshTreeForPath` helpers. The
    last three are exported so tests can hit them directly.
* `web/src/state/watcherScope.test.ts` (new): ten unit tests
  covering scope-match semantics (drive root, dir, descendants,
  siblings, any-of), `activeFbScopes` across the overlay +
  per-pane tab paths, and the `refreshTreeForPath` no-op
  when the parent dir isn't loaded.

Acceptance criteria status:

| Criterion                                          | Status |
|----------------------------------------------------|--------|
| Steady-state FB silent under out-of-scope activity | done   |
| Real in-scope changes still propagate              | done [^1]|
| Selection change re-points the watcher             | done [^2]|
| Closing the tab detaches the watcher               | done [^3]|
| No leaked watcher state in chan-server             | done [^4]|

[^1]: `refreshTreeForPath` merges the parent dir's listing
      whenever the event path lives in an active scope. New
      files / renames / deletes inside scope still update the
      visible tree.
[^2]: Scope is derived live from the FB's selection on every
      event tick. No subscribe/unsubscribe wiring — the next
      event after a selection change sees the new scope
      automatically.
[^3]: Closed tabs drop out of the layout walk in
      `activeFbScopes`. No per-tab subscription state to leak.
[^4]: chan-server unchanged; the existing single WS subscription
      remains. The original "subscribe-by-prefix in chan-drive"
      direction in the task header turned out unnecessary once
      the filtering moved to the SPA.

Gate status:

* `cargo fmt --check` — clean (no Rust changes).
* `cargo clippy --all-targets -- -D warnings` — clean.
* `npm run check` — 0 errors, 0 warnings.
* `npm run build` — green.
* `npx vitest run` — 474/474 green (10 new tests).

WebtestB walkthrough plan:

1. `chan serve` on a sample drive with multiple top-level dirs
   (e.g. `notes/`, `crates/`, `web/`).
2. Open the File Browser; expand only `notes/`; click into
   `notes/<some-file>.md` so the selection is inside `notes/`.
3. From outside chan, touch a file under `crates/` (e.g.
   `touch crates/x` followed by `rm crates/x`).
4. Expect: no tree flicker. The status pill might show "watch"
   activity briefly but the FB doesn't re-render.
5. Now `touch notes/y.md` from outside. Expect: the tree picks
   up the new file under `notes/`.
6. Open a second FB tab, navigate it to `crates/`. Re-do step
   3 — both FBs now refresh the visible subtree (the shared
   tree state means cross-FB events are still observed; this
   is the documented limitation of the "ship strict first"
   variant).

Held for commit clearance from @@Architect. Picking up
`fullstack-b-5` next (per-Hybrid theme propagation, last in the
queue).

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Strong call to skip the chan-drive subscribe-by-prefix
extension and filter SPA-side instead — the WS stream stays
single-channel and unscoped, and the per-FB scope derivation
on each event is cheap. The strict-scope shape (ship first,
visible-scope as a later refinement) matches the task spec
exactly. Limitation flagged honestly (cross-FB shared
`tree.entries`); that's the known cost of shipping strict
first.

10 new unit tests in `watcherScope.test.ts` pin the
intersection semantics across every meaningful selection
state.

**Commit clearance**: approved. Suggested subject:

```
Scope FB watcher to selection (drops out-of-scope drive activity) (fullstack-b-6)
```

Push waits for Round-1 close.
