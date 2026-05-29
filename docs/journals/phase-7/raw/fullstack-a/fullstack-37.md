# fullstack-37: replace last `window.prompt` + enforce no native dialogs

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

@@Alex remembered the "Rebuild index" path used to call
`alert()`, which fails silently in Chan.app's Tauri
WKWebView. Audited the SPA — that's already gone, and
the in-house dialog system (`uiConfirm` /
`ConfirmModal` / `PromptModal`) is in place. **One last
native call remains:**

```
web/src/editor/commands/format.ts:476
  target = window.prompt("URL")?.trim() ?? "";
```

Replace it with the in-house `PromptModal` flow, and
add a small lint-style guard so future code can't
re-introduce `window.alert`, `window.confirm`, or
`window.prompt`.

## Acceptance criteria

### Replace the lone `window.prompt`

* `format.ts:476` (the "insert link — ask for URL"
  path) uses the existing in-page prompt modal flow
  (whatever `PromptModal` + `pathPromptState` /
  sibling state) instead of `window.prompt`.
* The link-insertion command becomes async: shows the
  modal, awaits the resolution, inserts the URL on
  confirm. Cancel = no-op.
* Existing tests for the format command stay green;
  add one that asserts no `window.prompt` is invoked
  during a link insertion.

### Enforce going forward

* Add a tiny ESLint / build-time guard (your call on
  the mechanism — `no-restricted-globals` rule
  targeting `alert`, `confirm`, `prompt`, or a
  pre-commit grep, or both) so future code can't
  re-add native dialogs.
* The guard should allow `window.prompt` etc. only in
  test files (sometimes useful for assertions about
  what didn't get called).

### Verify the "rebuild index" path

* Audit the rebuild-index trigger end-to-end —
  confirm it now uses an in-house dialog (or nothing
  at all). If something slipped through, fix it as
  part of this task; otherwise note the audit
  verdict in the hand-off.

## Out of scope

* Refactoring the in-house dialog components (they
  work).
* Replacing `console.log`/`console.warn` with
  anything else — that's a different concern.
* Tauri-side native dialog plugins (we already have
  `dialog:default` capability for the file-open
  dialog; this task is about replacing the JS
  blocking dialogs).

## How to start

1. Read `web/src/components/PromptModal.svelte` and
   `web/src/state/*prompt*` to find the right helper
   to call.
2. Replace the synchronous `window.prompt` in
   `format.ts:476` with the async modal flow.
3. Add the ESLint rule (preferred — runs as part of
   the existing gate) or a pre-push grep.
4. Run the "Rebuild index" UI affordance against
   current main, confirm no `alert` fires.

## Hand-off

Standard. Pre-push gate green. The guard you add will
catch future regressions automatically; mention in the
hand-off how it's wired so future agents know to
respect it. Ping via
`alex/event-fullstack-a-architect.md`.

## 2026-05-19 10:32 BST — @@FullStackA specialist review

### Replace the lone window.prompt

* `web/src/editor/commands/format.ts:toggleLink` now imports
  `uiPrompt` from `state/store.svelte`. The legacy
  `target = window.prompt("URL")?.trim() ?? "";` line is
  replaced by `const answer = await uiPrompt("URL");
  target = answer?.trim() ?? "";`. Function is now `async`;
  the modal flow is identical to existing in-house uses
  (returns string on confirm, null on cancel).
* Wysiwyg.svelte's `toggleLink` wrapper now calls
  `void fmt.toggleLink(view, url)` so the StyleToolbar
  click handler stays sync-call shaped.
* Defensive guard: after the await, we check
  `(view as unknown as { destroyed?: boolean }).destroyed`
  before dispatching. The cast lands on `unknown` because
  EditorView marks `destroyed` private; the runtime field
  still exists on the live view. If the editor unmounted
  while the modal was open, the dispatch is skipped.

### Enforce going forward — no_native_dialogs.test.ts

Tiny vitest test at `web/src/no_native_dialogs.test.ts`:

* Uses `import.meta.glob("./**/*.ts|.tsx|.js|.jsx|.svelte",
  { query: "?raw", eager: true })` so the scan ships with
  zero `node:fs` / @types/node footprint.
* Regex `/\bwindow\.(?:alert|confirm|prompt)\s*\(/g`
  scans each shipped source for the forbidden calls.
* Excludes:
  * `state/store.svelte.ts` and
    `state/confirm.svelte.ts` — explanatory references to
    the legacy `window.prompt()` / `window.confirm()`
    names; no invocations.
  * `components/PromptModal.svelte` and
    `components/ConfirmModal.svelte` — same reason.
  * Any `*.test.*` file — tests sometimes need to assert
    the natives WEREN'T called.
* Failure message names the offending sites and points
  callers at `uiPrompt` / `uiConfirm`.
* Built into the existing `npm test` gate, so pre-push
  catches regressions without adding a new lint step.

### Verify the "Rebuild index" path

`SearchStatusOverlay.svelte:rebuildIndex` already routes
through `api.indexRebuild()` with error captured into
`indexResetError` state — no native dialogs anywhere in
that path. `grep -rEn '\b(alert|prompt|confirm)\s*\('
web/src/` returns only comment/documentation matches.
Audit clean.

### Tests

* `web/src/editor/commands/format.test.ts` — two new
  tests under "toggleLink prompts via the in-house modal,
  never window.prompt":
  * Resolves the in-house prompt with a URL, asserts the
    URL was inserted around the selection AND
    `window.prompt` was never called.
  * Cancel path: in-house prompt resolves with null,
    document remains unchanged, window.prompt still
    untouched.
* `web/src/no_native_dialogs.test.ts` — the guard test
  described above.

### Gate

* `npm run test -- format` — 15 passed (was 13; +2 new).
* `npm run test -- no_native_dialogs` — 1 passed.
* `npm run test` — 31 files / 277 tests, all pass.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green.

### Proposed commit message

> Replace last window.prompt + lock down native dialogs (fullstack-37)
>
> The link-insertion command now asks for the URL through the
> in-house uiPrompt modal instead of window.prompt, which fails
> silently inside Chan.app's WKWebView. A tiny vitest guard scans
> every shipped source for window.alert / window.confirm /
> window.prompt invocations so future code cannot re-introduce
> them; tests + the in-house dialog modules are excluded by name.

Ready for commit + push under standing topic-level
clearance.
