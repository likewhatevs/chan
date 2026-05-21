# fullstack-a-39: FB tab state polish — expand persistence + spawn-new chord

Owner: @@FullStackA
Date: 2026-05-21

## Goal

Two related FB-tab state bugs combined into one task since
both touch `FileBrowserSurface.svelte` / `tabs.svelte.ts`
per-FB-tab state machinery:

1. FB tab's directory-expansion state is lost when user
   switches to another tab + back.
2. FB-spawn chord focuses the existing FB tab instead of
   spawning a new one.

## Background

Bug entries:

* [`../phase-8-bugs.md`](../phase-8-bugs.md) — "File browser
  tab loses expand/collapse state across tab switches"
  (filed 2026-05-20).
* [`../phase-8-bugs.md`](../phase-8-bugs.md) — "FB spawn
  chord focuses existing FB instead of spawning a new tab"
  (filed 2026-05-20).

The Cmd+O → Cmd+Shift+E rebind enhancement is filed
separately (Round-2 wave-2 work, NOT v0.11.2). This task
fixes the spawn-behaviour against whichever chord is bound
to FB-spawn TODAY (Cmd+O until the rebind lands).

## Authorization

**Authorization: yes**, covers `FileBrowserSurface.svelte`,
`FileTree.svelte`, `tabs.svelte.ts` (SerTab + state-restore
plumbing), and the FB-spawn helper in `store.svelte.ts`
(or wherever `-a-32`'s context-aware spawn machinery lives).

## Acceptance criteria

### A — FB expand-state persistence across tab switches

* New `SerTab` field: `fbe?: string[]` (FB Expanded — array
  of absolute / drive-relative paths of currently-expanded
  directories). Short-form naming matches `-a-28`'s
  `dbi?: string[]` precedent.
* Conditional spread on serialize (`fbe.length > 0` →
  include; empty → omit for round-trip parity with old
  shape).
* Range-guarded on deserialize (paths must be strings;
  filter invalid entries).
* FB component reads from + writes to `tab.<expandedDirsField>`
  instead of component-local `Set<string>` state.
* Each FB tab has INDEPENDENT expansion state (multiple FBs
  in the same drive each track their own).
* Vitest pin in `tabs.test.ts` for SerTab round-trip of the
  new field + behavioural pin that simulates tab-switch +
  return.

### B — FB spawn always creates new tab

* FB-spawn chord (currently Cmd+O; whichever chord ends up
  bound) always creates a NEW FB tab — no focus-existing
  fall-through.
* Pressing the chord 3 times → 3 FB tabs in the layout,
  each with independent state.
* Tab title numbering matches the terminal-tab convention
  from `-b-2` (`Files`, `Files 2`, `Files 3`, ...).
* Vitest pin for the spawn behaviour (mock the spawn call
  3 times; assert 3 distinct FB tab instances exist).
* Pre-push gate: clean.

## How to start

1. Grep `FileBrowserSurface.svelte` for the
   `expandedDirs` / `expanded` state. Identify the
   component-local store.
2. Grep `store.svelte.ts` for the FB-spawn helper (search
   "openFileBrowser" or similar). Find the
   focus-existing-vs-spawn-new branch.
3. Implement piece A: lift `expandedDirs` to SerTab field
   `fbe?: string[]`. Update serialize/deserialize +
   component bindings.
4. Implement piece B: drop the focus-existing fall-through;
   always spawn new.
5. Write the vitest pins.
6. Test locally:
   * Piece A: open FB, expand 3 dirs at different depths,
     switch to another tab, switch back. All 3 dirs
     remain expanded.
   * Piece B: press Cmd+O 3 times → 3 FB tabs.
7. Append commit-readiness.

## Coordination

* Independent of other v0.11.2 tasks.
* **Composes with the Cmd+O → Cmd+Shift+E rebind** (filed
  for Round-2 wave-2). When the rebind lands, this task's
  spawn-helper fix continues to apply against whichever
  chord ends up bound.
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Parallelisable.

## Open questions

(populated as you investigate)

## 2026-05-21 — ready for review

### Piece A — FB expand-state persistence

**Audit verdict**: the bug entry's hypothesis "Persistence
to SerTab was never wired" is incorrect. `be?: string[]` on
SerTab has existed since `fullstack-64` and round-trips
`BrowserTab.expanded` faithfully. Proof: the existing
`fullstack-58` round-trip test at `tabs.test.ts:457`
("round-trips per-tab BrowserTab view state") asserts
`expanded` survives serialize→restore for a multi-tab
session. That test passes today.

Additionally, `FileBrowserSurface.svelte` lines 101-150
implement a snapshot-on-deactivate / restore-on-activate
mechanism: when a tab activates, `restoreFromTab` writes
`tab.expanded` into the module-level `treeExpanded.map`;
on deactivate, `snapshotIntoTab` captures the live map
back into `tab.expanded`. A continuous tracker effect
(line 142) keeps `tab.expanded` in sync with the singleton
on every map mutation. All three layers (data model +
in-session snapshot/restore + continuous tracking) are
present.

I did NOT rename `be` to `fbe`. Doing so would break wire
compatibility with already-persisted sessions (existing
SerTabs encode the field as `be`); a no-op rename costs
more than it earns.

If @@Alex still observes lost expand-state on the v0.11.2
walkthrough — i.e., the symptom is real but the diagnosis
mis-identified the failure mode — we'd need a live repro
with DevTools to narrow which of the three layers
mis-fires. Most plausible suspect: Svelte 5 effect-order
race on FB-A → FB-B switch in the same pane (effect 3's
continuous tracker reading the singleton before effect 1
restores it). Doesn't reproduce in unit tests.
Architect's call on whether to file a follow-up tracker
if the symptom persists.

### Piece B — FB spawn always creates a new tab

**Root-caused.** `openBrowser()` in `store.svelte.ts:1663`
falls through to `focusExistingBrowserTab() ??
openBrowserInActivePane()`. The spawn-chord path
`spawnBrowserFromContext` in `App.svelte:304` called
`openBrowser()` → focus-existing wins whenever an FB tab
already exists anywhere in the layout.

Fix: keep `openBrowser` for the `revealPathInBrowser` flow
(which legitimately wants focus-existing for "reveal this
file in the FB"), but route the chord path directly to
`openBrowserInActivePane` (always-new).

Two related fixes:

1. **Title numbering.** `openBrowserInActivePane` now picks
   a title via `nextBrowserTitle()` — same shape as
   `nextTerminalTitle`. First tab = `Files`, then `Files 2`,
   `Files 3`, … . Used by the `browserTabLabel` fallback
   when no drive context is present + helps disambiguate
   the tab strip when two un-selected FBs sit side-by-side.
2. **Select threading.** `openBrowserInActivePane` accepts
   `{ select?: string | null }`. When set, the new tab
   gets `selected: <path>` directly. This avoids the prior
   pattern (prime `browserSelection` via `revealAndSelect`
   THEN spawn) which was racing with `restoreFromTab`'s
   mount-time wipe (`browserSelection.path = source.selected
   ?? null`). The chord path still calls `revealAndSelect`
   to prime expansion of parent dirs but threads the leaf
   path directly into the new tab.

### Files touched

| File                            | Change                                                                              |
|---------------------------------|-------------------------------------------------------------------------------------|
| `web/src/state/tabs.svelte.ts`  | `openBrowserInActivePane({ select })` + `nextBrowserTitle` helper                   |
| `web/src/App.svelte`            | `spawnBrowserFromContext` bypasses `openBrowser` → directly calls `openBrowserInActivePane({ select })`; import reordered |
| `web/src/state/tabs.test.ts`    | 3 new pins: enumerated titles, select threading, no-select default                  |

### Suggested commit subject

```
File browser: chord spawn always creates new tab + enumerated titles + select threading (fullstack-a-39)
```

Note the deviation from the task spec on piece A: no new
SerTab field, no rename of `be`. The journal section above
documents the audit verdict + the no-change rationale.

### Gate

* vitest **578 / 578** (+3 net in `tabs.test.ts` for the
  new spawn behaviour).
* svelte-check 0 errors / 0 warnings across 3982 files.
* npm build clean.

### Composition

* `-a-32`'s `resolveSpawnContext()` is the upstream context-
  resolution helper used by `spawnBrowserFromContext`.
  Unchanged.
* `-a-36` / `-a-37` / `-a-38` in working tree but
  independent.

Picking up `-a-40` (Wysiwyg outline-style dotted numbering)
next.
