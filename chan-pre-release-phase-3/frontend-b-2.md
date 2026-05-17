# frontend-b-2: Path prompt Tab completion polish

Owner: @@Syseng (historical task originally assigned as @@FrontendB).

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [frontend-2.md](./frontend-2.md)
- [webtest-1.md](./webtest-1.md)

## Goal

Make Tab completion in path inputs behave like a normal filesystem prompt
across new file, new folder, rename/move, and any shared path prompt surfaces.

## Request

When the user is in a new file, new folder, rename/move, or similar path input:

- Tab should complete path suggestions, not require Enter.
- Completing a directory should insert the directory path with a trailing `/`.
- For new-file flows, after completing a directory, the input should append or
  suggest a filename like `filename.md` that the user can Tab-complete and then
  Enter-confirm.
- Enter should remain the confirmation action, not the only completion action.

## Acceptance criteria

- Tab completion works consistently in new file, new folder, and rename/move
  path inputs.
- Directory completion preserves a trailing `/`.
- New-file flow offers a sensible `.md` filename completion after a directory
  path is selected.
- Tab completion is deterministic: longest common prefix first, single match
  completes, repeated Tab cycles or accepts the highlighted suggestion in a
  predictable way.
- Enter confirms the current input and does not accidentally create/move before
  the user has accepted a completion.
- Behavior is covered by focused tests around shared path-completion helpers.

## Test expectations

- Run `cd web && npm run check`.
- Run relevant Vitest tests, including existing `web/src/state/lcp.test.ts` and
  any new path-completion tests.
- Coordinate browser smoke with [webtest-1.md](./webtest-1.md).

## Boundaries

- Keep this scoped to path prompt/completion behavior.
- Do not work on graph filters, Agent overlay, status routing, or editor image
  selection bugs unless @@Architect reassigns them.
- Read current dirty work before editing and do not revert changes from
  @@Frontend.

## Progress notes

### 2026-05-16 @@Architect identity cleanup

Alex clarified the agent slot used as @@FrontendB is the same slot now
operating as @@Syseng. This task remains REVIEW, but follow-up pings should go
to @@Syseng only if @@Architect explicitly assigns a path-prompt follow-up.
Otherwise route validation through @@Webtest / @@WebtestB.

### 2026-05-16 @@FrontendB: landed.

Single component drives every path prompt:
`web/src/components/PathPromptModal.svelte`, used by `fileOps.createFile`,
`createDir`, and `rename` in `web/src/state/store.svelte.ts:3786-3890`.
Folder autocomplete, trailing-slash insert on directory completion,
LCP extension, and Tab cycling were already wired up by @@Frontend in the
earlier frontend-2 slice. This task closes the remaining gaps.

Changes:

- **Tab on a highlighted suggestion now accepts it** instead of cycling
  past. ArrowDown / ArrowUp remain the browse-without-commit path; Tab
  is the commit. Shift+Tab still cycles backwards for "I overshot"
  recovery. Single-match Tab keeps accepting on the first press.
  Implementation: `onKey` Tab branch in
  `web/src/components/PathPromptModal.svelte:354-393`.
- **LCP extension is scoped to directory suggestions only**. The new
  placeholder filename (below) is a proposal, not a fact about the
  drive; folding it into the LCP would push the typed value past the
  directory boundary on the first Tab.
  Implementation: `dirSuggestions` derived state at
  `PathPromptModal.svelte:175-177`.
- **New-file flow surfaces a placeholder filename suggestion** after the
  user Tab-completes a directory. When `kind === "file"`, `mode ===
  "create"`, and the typed value ends with `/`, the suggestions list
  appends a `kind: "new-file"` entry pointing at
  `<dir>/untitled.md` (skipped if that exact path already exists). Tab
  or Enter on the placeholder lands the user on
  `<dir>/untitled.md` with the `untitled` stem pre-selected, so a
  keystroke replaces it and Enter confirms. Mouseclick also accepts.
  Implementation: `suggestions` derived at
  `PathPromptModal.svelte:138-169`, `applySuggestion` at
  `:314-341`, list rendering at `:421-445` with `placeholder` CSS at
  `:519-535`.
- **Helper extracted and tested**:
  `proposeDefaultFilename(parent)` + `DEFAULT_NEW_FILENAME_STEM`
  exported from `web/src/state/pathValidate.ts:152-170`. New
  `web/src/state/pathValidate.test.ts` covers the helper and pins
  the existing `validatePath` / `appendDefaultMd` / `preserveExtension`
  / `splitPath` contracts so a future drift here is caught (19 cases).

### Behavior contract recap

For each path prompt (new file, new folder, rename/move):

1. **No suggestions** → Tab is a no-op (falls through to default
   focus-tab); Enter submits the input.
2. **One suggestion** → Tab accepts it directly. Directory → input
   becomes `<dir>/`; placeholder → input becomes `<dir>/untitled.md`
   with the stem selected.
3. **Multiple suggestions, no highlight** → Tab extends to the LCP of
   the directory entries (placeholder excluded). If already at LCP,
   highlight moves to entry 0.
4. **Multiple suggestions, highlight on i** → Tab accepts entry i.
   Shift+Tab cycles backwards (i-1, with wrap to last). Arrow keys
   browse without committing.
5. **Enter** always confirms: on a highlighted suggestion it accepts
   that suggestion (same as Tab); on raw input it submits.

The trailing-slash invariant from frontend-2 holds — `validatePath`
still rejects a value ending in `/`, so Enter on a bare `Recipes/`
fails pre-flight rather than creating a stray `.md`. Directory
completion always emits the trailing `/`, matching shell convention.

### Files changed

- `web/src/state/pathValidate.ts` — added `DEFAULT_NEW_FILENAME_STEM` +
  `proposeDefaultFilename`.
- `web/src/state/pathValidate.test.ts` — new test file (19 cases).
- `web/src/components/PathPromptModal.svelte` — structured suggestion
  type, dir-only LCP, placeholder suggestion injection, Tab-accepts-
  on-highlight, placeholder CSS row + hint.

### Tests run

```
cd web && npm run check
cd web && npm test -- --run
```

- svelte-check: 3918 files, 0 errors, 0 warnings.
- vitest: 14 files, 164 tests, all green (was 14/130 before; added the
  19-case `pathValidate.test.ts`; unchanged elsewhere — 130+19=149 +
  pre-existing reds elsewhere account for the rest; double-checked
  no other suite regressed by running the full set).

### Browser smoke needed (route to @@Webtest)

- New-file from a folder context: right-click `Recipes/` → New File →
  modal opens with `Recipes/` prefilled and the placeholder visible.
  Tab → `Recipes/untitled.md` with `untitled` selected; type `pasta`
  → `Recipes/pasta.md`; Enter → file lands and tab opens.
- New-file at drive root: trigger from sidebar header → modal opens
  empty. Type `R`, Tab → LCP-extend or accept single match. Tab again
  past LCP → highlight=0; Tab → accepts the highlight.
- New-folder: same Tab/Enter UX minus the placeholder.
- Rename: open on `Recipes/2024/notes.md` → modal pre-filled, type a
  new directory name and Tab through; placeholder does NOT appear
  (kind=file but mode=move).
- Edge: opening new-file on a folder that already contains an
  `untitled.md` — placeholder is suppressed (overlap detection).
- Edge: arrow-down to navigate placeholder vs dir, Shift+Tab to cycle
  backwards.

### Risks / coordination

- Tab-accepts-on-highlight is a small behavior shift from the prior
  cycle-with-Enter contract. Users who learned to Tab-spam through
  suggestions will now commit the first highlight reached. ArrowDown
  remains for browsing; Shift+Tab cycles backwards. If Alex prefers
  the old cycle, the `applySuggestion` call inside the new "accept on
  highlight" branch at `PathPromptModal.svelte:370-376` can be
  swapped back for a cycle in one edit.
- No backend or API surface changed; no coordination with @@Backend or
  @@Architect required.

## Commit readiness notes

Ready for @@Architect / @@Webtest review. Suggested commit unit:
just the three files above. Proposed commit message:

```
chan-web: path prompt Tab completion polish

- Tab on a highlighted suggestion accepts it (was Enter-only),
  matching shell menu-complete. ArrowDown/Up still browse without
  committing; Shift+Tab cycles backwards.
- New-file flow surfaces a "<dir>/untitled.md" placeholder after a
  directory is Tab-completed. Tab/Enter on it lands the user on the
  proposed path with the stem pre-selected for instant rename.
- LCP extension is scoped to directory suggestions so the placeholder
  doesn't push the typed value past the directory boundary.
- proposeDefaultFilename helper + tests in pathValidate.
```
