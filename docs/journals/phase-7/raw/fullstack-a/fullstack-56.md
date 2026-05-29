# fullstack-56: drop Cmd+S and the Save action surface — autosave is canonical

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Remove the explicit Save shortcut and action.
Autosave already covers the write path (debounced
on idle, plus tab-close + visibility hooks); the
Cmd+S binding is muscle-memory legacy that doesn't
buy anything users don't already get for free.

@@Alex flagged "drop the Cmd+S and file→save
option, default is autosave". There's no actual
File→Save menu item in the codebase (audited:
keyboard-only surface today), so the user's mental
model collapses to the Cmd+S binding alone.

## Relevant code

* `web/src/state/shortcuts.ts:120-123` — the
  `app.save` entry. Drop the whole entry; nothing
  else in shortcuts.ts depends on it.
* `web/src/App.svelte:644-649` — `case "app.save":`
  handler in the global action switch. Drop the
  case.
* `web/src/components/Pane.svelte:377-400` —
  `async function onSave()` and the direct
  Cmd+S keystroke interception at line 392-400.
  Both go.
  * Note line 393-395 comment: "Plain Cmd/Ctrl+S
    only. Cmd/Ctrl+Shift+S is the editor's
    strikethrough toggle." With the meta-S branch
    gone, the strikethrough binding still
    survives via whatever editor-level handler
    owns it; verify by trying Cmd+Shift+S on a
    selection after the change.
* `web/src/components/Pane.svelte:25` — `saveTab`
  import. Drop if no remaining consumer in
  `Pane.svelte`. `saveTab` itself stays in
  `tabs.svelte.ts` (autosave + close hooks call
  it).

## Acceptance criteria

* Cmd+S in a focused editor pane: no chan action
  fires. Autosave still kicks in on idle / on
  blur / on tab close per its existing schedule.
* Cmd+Shift+S still toggles strikethrough (the
  editor owns this — not chan).
* No "Save" label rendered anywhere in the UI.
  (Audit: there isn't one today; this is a
  belt-and-braces check after the action goes
  away.)
* Help cheatsheets / Pane Mode help (`PaneModeHelp.svelte`)
  don't list Save. Confirmed not listed today;
  re-verify after the drop.

### Browser-default suppression (judgement call)

* In a chan-desktop Tauri shell, Cmd+S won't
  trigger "Save Page As" — WKWebView doesn't
  surface that gesture.
* In the SPA-running-in-a-regular-browser case
  (which is supported but secondary), Cmd+S
  WILL trigger the browser's Save Page As
  dialog if nothing intercepts.
* Pick one:
  1. Remove all Save code, accept that
     browser-SPA users get Save Page As on Cmd+S
     (small annoyance, matches every other web
     app).
  2. Keep a tiny `preventDefault` swallower in
     Pane.svelte's keystroke filter for plain
     Cmd+S so Save Page As is suppressed but no
     chan action fires.

Recommended: (1). Simpler, matches the "drop the
surface" framing. Note the choice in the
implementation log either way.

### Tests

* `tabs.svelte.ts` autosave tests stay green
  (none of this changes the autosave path).
* `paneModeKeymap.test.ts` and any keymap test
  asserting Cmd+S maps to save: flip / drop the
  assertion.
* If any test asserts the `app.save` shortcut
  definition exists, drop it.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Affects `webtest-a-8` indirectly — the Pane Mode
  + keymap walkthrough doesn't currently test
  Cmd+S, but if Lane A notices "Cmd+S does
  nothing" during the walk, that's now expected
  behavior, not a defect.
* No re-walk cost on the carousel / FB surfaces.
* Standing topic-level commit clearance.

## 2026-05-19 16:33 BST — @@FullStackA implementation note

Implementation:

* `shortcuts.ts`: removed the `app.save` SHORTCUTS
  entry (`Mod+S` / `Mod+S` / group "File"). Replaced
  with a WHY comment naming the task.
* `App.svelte`: removed the `case "app.save":` block
  in the global action switch + the `saveTab`
  import.
* `Pane.svelte`: removed the `onSave()` async helper
  and the plain Cmd/Ctrl+S keystroke interception
  inside `onKeyDown`. The strikethrough branch
  (Cmd+Shift+S, owned by the editor) is unaffected
  because the plain-S gate is gone. Dropped the
  `saveTab` import — autosave + close hooks still
  call `saveTab` from their own consumers in
  `tabs.svelte.ts`.

Judgement call: went with option (1) per the
task's recommendation. No `preventDefault` swallower
for the browser-SPA case — Cmd+S there will trigger
"Save Page As", which matches every other web app.
The Tauri shell already swallows that gesture, so
the desktop UX is clean.

Audit: `grep -rE "app\.save"` returns only my own
comment lines (`shortcuts.ts:118` + `App.svelte:643`).
No native bridge references. PaneModeHelp has no
Save row to drop.

Gate green:

* `npm run check` (0 errors / 0 warnings),
* `npm run test` (343 passed),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Proposed commit message:

> Drop Cmd+S and the Save action (fullstack-56)
>
> Remove the explicit Save shortcut + action surface;
> autosave (debounced on idle + tab-close + visibility
> hooks) is the canonical write path and the Cmd+S
> binding was muscle-memory legacy with no menu
> backing. Drops `app.save` from SHORTCUTS, the
> matching App.svelte case, Pane.svelte's `onSave()`
> + plain Cmd+S keystroke interception, plus the now-
> unused `saveTab` imports in App.svelte / Pane.svelte.
> Cmd+Shift+S strikethrough (owned by the editor)
> survives.
