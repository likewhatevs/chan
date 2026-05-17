# syseng-frontend-3: File Browser find navigation and Esc handling

Owner: @@Syseng.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [webtest-1.md](./webtest-1.md)
- [webtest-2.md](./webtest-2.md)
- [frontend-2.md](./frontend-2.md)

## Role

Frontend implementation/support lane. Load the frontend/webdev skill before
editing.

## Goal

Fix the File Browser find regressions found by @@Webtest / @@WebtestB.

## Bugs

1. BUG-FE2-A: File Browser find next/previous never advances.
   - Repro: open File Browser, expand `binary/`, Cmd+F, type `sample`.
   - Counter shows `1 of 3`.
   - Press Enter or click next/previous.
   - Expected: current match advances.
   - Actual: current stays on `sample.bin` / `1 of 3`.
   - Hypothesis in [webtest-2.md](./webtest-2.md): `findCurrentIndex` reset is
     tied to derived `findMatchPaths` array identity and re-runs after selection
     side effects.
2. Esc inside overlay find bars closes the whole overlay.
   - Repro: open File Browser, Cmd+F, press Esc.
   - Expected: close find bar only.
   - Actual: Esc also bubbles to overlay Esc handler and closes the File Browser.
   - WebtestB noted similar risk in `InlineAssist.svelte::onFindKeydown`.

## Acceptance criteria

- File Browser next/previous advances through all visible matches.
- Enter and Shift+Enter in the File Browser find input step forward/back.
- Esc in the File Browser find input closes only the find bar and leaves the
  File Browser overlay open.
- Esc in the Agent find input closes only the Agent find bar and leaves the
  Agent overlay open.
- Add focused tests if practical for helper/state behavior.
- Browser validation goes through @@Webtest / @@WebtestB.

## Boundaries

- Keep this scoped to find navigation / Esc propagation.
- Do not edit backend layout config or graph filter code.
- Do not restart webtest services.

## Progress notes

- 2026-05-16 @@Syseng: Started. Loaded webdev skill. Inspecting FileTree find
  navigation state and File Browser / Agent find-bar Esc propagation before
  editing.
- 2026-05-16 @@Syseng: Fixed BUG-FE2-A by replacing FileTree's cursor-
  management effect. The earlier signature-string dedupe still relied on
  one effect both seeding the cursor (on query change) and reacting to
  every findMatchPaths re-derivation, which left subtle races where
  `findStep`'s in-range cursor could be rewritten back to 0. New shape:
  non-reactive `lastSeededQuery` cache; the effect branches on
  `findQueryState !== lastSeededQuery` to either seed-from-zero or
  clamp-only (in-range cursor preserved). Either branch always
  republishes the count via `findOnCount`, so findStep + arrow / mouse
  selection keep the host counter in sync regardless of which path ran.
  Implementation: `web/src/components/FileTree.svelte:540-602`.
- 2026-05-16 @@Syseng: Esc propagation: confirmed both
  `FileBrowserOverlay.svelte::onFindKeydown` and
  `InlineAssist.svelte::onFindKeydown` already call `e.stopPropagation()`
  alongside `e.preventDefault()` in the dirty tree, which prevents the
  document-level `App.svelte::onWindowKey` Esc handler (overlay-stack
  pop) from firing. No additional edits needed for bug 2.
- 2026-05-16 @@Syseng: Picked up WebtestB's follow-up regression from
  [webtest-2.md](./webtest-2.md): Agent find mounted but immediately hit
  Svelte `effect_update_depth_exceeded`, so Esc / close-button interactions
  could not settle. Fixed by splitting Agent find scanning and DOM highlight
  painting into separate effects; the scan effect writes `findMatches` /
  `findCurrentIdx`, while the paint effect reads them.

## Files changed

- `web/src/components/FileTree.svelte`
- `web/src/components/FileBrowserOverlay.svelte`
- `web/src/components/InlineAssist.svelte`

## Tests run

- `cd web && npm run check` — pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run` — pass, 14 files / 168 tests.
- 2026-05-16 follow-up after Agent find loop fix:
  - `cd web && npm run check` — pass, 0 errors / 0 warnings.
  - `cd web && npm test -- --run` — pass, 14 files / 168 tests.

## Browser validation needed

- File Browser: expand `binary/`, Cmd+F, type `sample`; Enter and the next /
  previous buttons should cycle across all three matches and update the counter.
- File Browser: Esc in the find input should close only the find bar and leave
  the File Browser overlay open.
- Agent overlay: Cmd+F, then Esc in the find input should close only the Agent
  find bar and leave the Agent overlay open.
- Agent overlay: Cmd+F should not emit `effect_update_depth_exceeded`; close
  button, Esc, Enter, and Shift+Enter should remain responsive.

## Commit readiness notes

- Ready for review; browser validation still owed by @@Webtest / @@WebtestB.
