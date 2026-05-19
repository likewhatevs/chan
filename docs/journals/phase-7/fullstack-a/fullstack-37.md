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
