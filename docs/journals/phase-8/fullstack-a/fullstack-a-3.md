# fullstack-a-3: Cmd+K cluster (label, flashing H, immediate commit for 1/2/3)

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Three related Hybrid-NAV / Cmd+K behaviours:

1. **Status-bar label on Cmd+K** today prints `pane mode`. It
   should print: `Hybrid ☯ Enter commit, Esc discard, H help`.
2. **Remove the flashing "H" in the middle of the screen** when
   Hybrid NAV opens. The PaneModeHelp cheat-sheet already covers
   the H affordance; the mid-screen flash is noise.
3. **Cmd+K → 1 / 2 / 3 commits immediately.** Slots 1-3 are the
   "common" Hybrid actions; pressing one of those numbers should
   exit Hybrid mode and execute, without waiting for Enter.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under the
`cmd+k` and `commands 1, 2, 3` items.

Phase-7 reference: `fullstack-61` flashed the H, `fullstack-62`
renamed "Pane Mode" → "Hybrid NAV", `fullstack-76` bumped the
flash duration. This task removes the mid-screen flash entirely
and amends the status-bar copy to match.

## Acceptance criteria

* Pressing Cmd+K shows the exact status-bar label
  `Hybrid ☯ Enter commit, Esc discard, H help`.
* No flashing H glyph anywhere on screen on Hybrid entry.
* Cmd+K followed by `1`, `2`, or `3` commits immediately
  (matches the existing Enter-to-commit path, just skipped).
* `H` still opens the help overlay (PaneModeHelp.svelte
  unchanged).

## How to start

Status-bar copy: likely `web/src/components/PaneModeStatus.svelte`
or wherever the Hybrid status string is rendered. Flash-H glyph:
look for the `fullstack-61` / `fullstack-76` work to find the
component. Immediate-commit: the Hybrid keymap dispatch table.

## 2026-05-19 — implementation note

Three independent edits:

1. **Status-bar label**: the Hybrid pill in
   `web/src/components/AppStatusBar.svelte` (`paneMode.spawnIntent`
   block) was rendering `pane mode ... Enter commit · Esc discard`
   as two separate spans. Replaced with the single inline text
   `Hybrid ☯ Enter commit, Esc discard, H help`. The
   spawn-intent chip (`→ stage ${kind}`) stays since 1/2/3 still
   stage briefly before commit; the chip is visible only between
   `paneModeStageSpawn(...)` and `commitPaneMode()` (now in the
   same case, so effectively invisible on real keypresses).

2. **Mid-screen H flash removed**: dropped all of
   `paneModeFlashVisible`, `paneModeFlashKey`,
   `paneModeWasActive`, `paneModeFlashTimer`,
   `PANE_MODE_FLASH_MS`, the `$effect` that drove the flash,
   the `onDestroy` cleanup, the DOM block (`pane-mode-flash`
   plus `pane-mode-flash-key` / `pane-mode-flash-text`), and the
   matching CSS / reduced-motion @keyframes from `App.svelte`.
   The status bar's Hybrid pill already telegraphs `H help` and
   `PaneModeHelp.svelte` covers the cheatsheet on press.

3. **Cmd+K → 1/2/3 commit immediately**: in `App.svelte`'s
   `handlePaneModeKey`, cases `"1"`, `"2"`, `"3"` now call
   `paneModeStageSpawn(...)` followed by `commitPaneMode()` +
   `scheduleSessionSave()` + `paneModeHelpVisible = false`
   inside the same case (matching the `"4"` / new-file
   commit-first pattern). The `"2"` case also primes
   `revealAndSelect(ctx.file/dir)` before commit so the new FB
   tab lands already expanded to the contextual node — same
   prime the Enter handler does for any other staged browser
   intent. The Enter handler's intent-peek stays as defensive
   code for forward-compat (UI affordances that might stage
   without committing).

Test updates in
`web/src/components/paneModeKeymap.test.ts`: the three
spawn-staging tests now assert the stage-and-commit shape; the
Pane-Mode-flash describe block was replaced with negative
assertions that the `pane-mode-flash*` symbols no longer appear
in `App.svelte`.

Files touched:

* `web/src/App.svelte` — flash state + DOM + CSS removed;
  cases 1/2/3 commit immediately.
* `web/src/components/AppStatusBar.svelte` — Hybrid label
  rewritten.
* `web/src/components/paneModeKeymap.test.ts` — assertions
  updated to match new behaviour.

Pre-push gate (SPA portion): vitest 445/445 green
(`Pane Mode entry flash` block went from 2 tests to 1 negative
test → -1 from the prior 446);
`npm run check` 0 errors / 0 warnings; `npm run build` clean.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Three-part edit lands cleanly:

1. Hybrid pill copy updated to the exact spec string.
2. Mid-screen H flash entirely removed (state, DOM, CSS,
   reduced-motion keyframes, $effect, onDestroy cleanup). Net
   simplification — the status-bar pill + `PaneModeHelp` cover
   the affordance.
3. Cases 1/2/3 commit immediately via the
   stage-and-commit-in-same-case pattern, matching the existing
   `"4"` shape. The Enter handler's intent-peek stays as
   forward-compat defensive code — right call, doesn't hurt
   and protects future staging UI.

Test diff is honest: -1 from removing the flash describe block,
+ new negative assertions for the now-absent flash symbols.
Gate green.

**Commit clearance**: approved. Commit `fullstack-a-3` as a
standalone change. Suggested subject:

```
Hybrid cluster: status-bar copy + drop flash + immediate-commit on 1/2/3 (fullstack-a-3)
```

Push waits for Round-1 close.

**Coordination note for `fullstack-a-7` (Cmd+K → Cmd+. swap)**:
the Hybrid pill copy lands first via this commit with the Cmd+K
wording. When you cut `-7`, the same `AppStatusBar.svelte` pill
gets the Cmd+. wording — single-line edit then. No duplication
of the label copy across surfaces.

Pick up `fullstack-a-4` next (rich prompt cluster: cursor
focus, overlay-bottom, spawn-agent dialog, Cmd+Enter stay).
