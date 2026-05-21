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
