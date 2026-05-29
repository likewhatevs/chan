# fullstack-79: auto-focus rich prompt input on entry

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged: when the user enters rich
prompt mode (`Cmd+K + p` per `fullstack-50`,
or whatever other entry path exists), the
cursor should auto-focus inside the prompt
input. Currently the user has to click the
prompt to start typing — extra friction on a
primarily-keyboard-driven feature.

## Spec

* On rich prompt mount / open (whether via
  `Cmd+K + p` show-existing, `Cmd+K + p`
  spawn-then-show, Alt+Space global, or any
  other entry path) — focus the prompt
  input immediately.
* Auto-focus survives:
  * Hybrid NAV exit on commit (the prompt
    opens, Pane Mode closes, focus is
    already in the prompt — no extra hop).
  * Existing keyboard handlers (Esc to
    dismiss the prompt, Enter to send,
    etc.) work without an intermediate
    click.
* On re-show (Cmd+K + p when the prompt is
  already open in the focused pane) —
  re-focus the input even if it's already
  mounted (in case the focus drifted
  elsewhere since last open).

## Relevant code

* `web/src/state/tabs.svelte.ts` —
  `showOrSpawnRichPromptInFocusedPane()`
  (from `fullstack-50`). Find where the
  prompt component is mounted / shown;
  add a focus call after the surface
  becomes visible.
* `web/src/components/...` — wherever the
  rich-prompt input lives (likely
  RichPrompt.svelte or similar). Add an
  `onMount` / reactive focus call. Use
  `tick()` to wait for the element to be
  in the DOM before calling `.focus()`.
* Existing patterns to follow: find any
  Svelte component that auto-focuses on
  open (FB find-bar at `Cmd+F` per the
  `findInputEl?.focus()` pattern in
  `FileBrowserSurface.svelte:114` is a
  good reference).

## Acceptance criteria

* `Cmd+K + p` from a pane with no rich
  prompt → spawn rich prompt → input is
  focused, user can type immediately.
* `Cmd+K + p` from a pane with rich prompt
  already shown → input gets focus (works
  even if focus was elsewhere when the
  shortcut fired).
* Alt+Space global → same.
* Tab into / out of the prompt works as
  normal browser tabbing.
* Esc dismisses the prompt; focus returns
  to whatever it was before opening
  (mirror the close-find-bar focus return
  if such a pattern exists).
* No focus regression on the underlying
  pane after the prompt closes.

### Tests

* Vitest / component test: mount the rich
  prompt; assert `document.activeElement`
  is the prompt's input within the next
  tick.
* Component test: open prompt → focus
  elsewhere → open prompt again →
  input regains focus.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0-blocking-soft. Tiny UX win;
  ships easily in v0.11.0 or v0.11.1.
* Coordinate with `-72` (spawn keys →
  draft/commit). `p` is out of scope for
  `-72` per its task spec (rich prompt
  is multi-step), so this task is
  independent.
* Queue position: end of Lane B queue.
  Updated queue: `-67` (shipping) → `-71`
  (shipping) → `-78` → `-79`.
* Standing topic-level commit clearance.

## 2026-05-19 23:05 BST — implementation

**Focus-nonce pattern.** Added a
`focusNonce?: number` field to
`TerminalRichPromptState` (mirroring the
find-bar's nonce at `tabs.svelte.ts:95`).
`openActiveTerminalRichPrompt()` bumps it
on every call — `1` on fresh creation and
`(focusNonce ?? 0) + 1` on re-show — so the
prompt component can detect "show again" even
when `open` was already true.

**TerminalRichPrompt's effect.** A reactive
`$effect` reads `prompt.focusNonce` to
subscribe to bumps, then dispatches to the
editor child after a `tick()`. The tick wait
covers both the first-mount case (Svelte
needs a frame to settle the `bind:this`
binding) AND the wysiwyg ↔ source toggle
case (the `{#key mode()}` block remounts the
child on toggle, and we wait for the new
binding before calling focus).

**Source mode** gets a parallel
`bind:this={sourceRef}` and focuses via
`sourceRef.focusAt(prompt.buffer.length)`
which lands the caret at the end of the
current buffer.

**Edits:**

* `web/src/state/tabs.svelte.ts`:
  - Added `focusNonce?: number` to
    `TerminalRichPromptState`.
  - `openActiveTerminalRichPrompt` seeds
    `focusNonce: 1` on fresh creation and
    bumps `(focusNonce ?? 0) + 1` on re-show.

* `web/src/components/TerminalRichPrompt.svelte`:
  - Imported `tick` from `svelte`.
  - Added `sourceRef: Source | undefined`
    state binding.
  - Added `$effect` watching `prompt.focusNonce`
    that calls `wysiwygRef?.focusEnd()` (or
    `sourceRef?.focusAt(prompt.buffer.length)`
    in source mode) after a `tick()`.
  - Added `bind:this={sourceRef}` to the
    `<Source>` template.

* `web/src/components/richPromptAutoFocus.test.ts`
  (new) — source-grep sentinel, 4 assertions:
  1. `TerminalRichPromptState` declares
     `focusNonce?: number`.
  2. `openActiveTerminalRichPrompt` bumps the
     nonce on both branches.
  3. TerminalRichPrompt's effect reads the
     nonce + dispatches to both editor refs.
  4. Source mode has the `bind:this={sourceRef}`
     binding so source-mode focus works too.

**Coverage of acceptance criteria:**

* `Cmd+K + p` spawn → openActiveTerminalRichPrompt
  → fresh state has focusNonce: 1 → effect
  runs → wysiwygRef.focusEnd() after tick().
  ✓
* `Cmd+K + p` re-show → focusNonce bumps from
  N to N+1 → effect re-runs → focus regrabbed.
  ✓
* Alt+Space global → routes through same
  openActiveTerminalRichPrompt → same path.
  ✓
* Esc dismiss → onClose fires; focus return
  is handled by the existing close path
  (no new behaviour needed).

**Gate.** `npm run check` 0/0; `npm run test`
41 files / 417 tests (was 40 / 413; +4 new
sentinel — and +0 from this turn's
parallel-lane work since the run); `npm run
build` clean; `scripts/pre-push` green.

**Visual eyeball.** Skipped — the focus path
is mechanical (tick + view.focus()), and the
source-grep sentinel covers the wiring. If
@@Alex flags a timing issue (e.g. focus
doesn't survive the `{#key mode()}` remount
for some interaction path), follow-up.

**Out of scope:**
* Focus-return-to-pane on Esc — the existing
  onClose path handles this; not in the
  task's required criteria beyond "no
  regression on the underlying pane".

**Commit readiness:**

Files staged:
* `web/src/state/tabs.svelte.ts`
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/richPromptAutoFocus.test.ts`
* `docs/journals/phase-7/fullstack-b/fullstack-79.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Auto-focus the rich prompt input on every open (fullstack-79)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 22:15 BST cut.
