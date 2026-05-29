# fullstack-a-4: Rich prompt cluster (cursor focus, overlay-bottom, spawn-agent dialog)

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Four related rich-prompt UX fixes:

1. **Cursor focus on rich-prompt open** — if no bubbles, focus
   the prompt input; if bubbles present, focus the survey area
   (so numbered keystrokes reply immediately).
2. **Cursor stays in rich prompt after Cmd+Enter** — today it
   drops out; per @@Alex it should remain so consecutive prompts
   are fluid.
3. **Rich-prompt overlay obscures bottom of terminal** — push
   the terminal up (or resize cleanly) so the last rendered
   terminal line is still visible above the overlay.
4. **'Spawn agent' from rich prompt** — clicking the button
   currently dims the screen but no dialog appears. Restore the
   spawn dialog (or wire it for the first time if it never
   landed).

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under
"Rich prompt", "spawn agent", and the overlay/cursor items.

`spawn-agent` ties into phase-7 wave-B fullstack-20 (spawn from
rich prompt + pre-flight survey). Confirm whether that work
landed; if it shipped but the dialog regressed, this is a
fix; if it never landed, this is the wire-up cut.

## Acceptance criteria

* Opening the rich prompt with no bubbles → caret in the prompt
  input.
* Opening with at least one bubble → caret in the survey area;
  pressing `1`/`2`/`3` etc. replies the focused bubble.
* After dismissing all bubbles → caret returns to the prompt
  input.
* Cmd+Enter submits the prompt and leaves the caret in the rich
  prompt area for the next entry.
* Rich-prompt overlay no longer paints over the bottom terminal
  line. Resize is acceptable; covering is not.
* `Spawn agent` button opens an actual dialog (pre-flight survey
  + agent profile selector).

## How to start

Likely files:
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/BubbleOverlay.svelte`
* SPA terminal layout for the resize/push behaviour.

## 2026-05-19 — implementation note

Four edits land here:

1. **Cursor focus on rich-prompt open** — `TerminalRichPrompt`
   gained a `bubbleCount?: number` prop. The auto-focus
   `$effect` early-returns when `bubbleCount > 0`, so the
   editor doesn't steal keystrokes while surveys are present.
   The same effect re-runs when `bubbleCount` drops back to 0
   and refocuses the editor — caret returns to the prompt
   input once the user has answered every bubble.
   `TerminalTab.svelte` derives `bubbleCount` from
   `tab.watcher.events` filtered to non-`survey-reply` items
   (matching `BubbleOverlay`'s `visibleEvents` rule) and passes
   it down. BubbleOverlay's `onWindowKeydown` already routes
   `1`/`2`/`3` to its focused survey when the active target is
   not editable, so just keeping the editor unfocused is
   enough — no plumbing into BubbleOverlay required.

2. **Caret stays after Cmd+Enter** — `submitRichPrompt` in
   `TerminalTab.svelte` previously called `term?.focus()`
   after sending the buffer, kicking focus back into the
   terminal grid. Replaced with a bump of
   `tab.richPrompt.focusNonce`, which re-triggers the rich
   prompt's auto-focus effect and lands the caret back in the
   editor for the next entry.

3. **Rich-prompt overlay obscures terminal bottom** — the
   `.terminal-host` div in `TerminalTab.svelte` gets an inline
   `margin-bottom = richPrompt.heightPx + 12px` when the
   prompt is open. xterm's `ResizeObserver` picks the smaller
   box up and `fit()`s the cell grid; the bottom-most rendered
   line is preserved above the prompt. Margin clears when the
   prompt closes.

4. **Spawn agent dialog visibility** — the dialog used to
   mount inside `<TerminalRichPrompt>` and was sensitive to
   every ancestor stacking context (`.pane { overflow: hidden;
   box-shadow }`, Hybrid NAV's `filter: saturate(0.8)` on
   unfocused panes, `.rich-prompt { position: absolute;
   z-index: 20 }`). Lifted to a state-driven singleton:
   - New `web/src/state/spawnDialog.svelte.ts` exposes
     `spawnDialogState`, `openSpawnDialog(...)`,
     `closeSpawnDialog()`.
   - `SpawnDialog.svelte` rewritten to read from the
     singleton instead of taking `open` as a bindable prop.
   - `TerminalRichPrompt.svelte`'s "Spawn agent" button now
     calls `openGlobalSpawnDialog({ orchestratorSessionId,
     onSpawned })` and no longer renders the dialog locally.
   - `App.svelte` mounts a single `<SpawnDialog />` at the
     root next to `ConfirmModal` / `PathPromptModal` so the
     fixed-position backdrop is anchored to the viewport for
     real, with no ancestor stacking context in the way.
   `TerminalRichPrompt.test.ts` updated: it now mounts the
   SpawnDialog into a sibling host since the test target is
   not the App root, and resets `closeSpawnDialog()` in
   `afterEach` so module-level state doesn't leak between
   tests.

Files touched:

* `web/src/components/TerminalRichPrompt.svelte` — bubbleCount
  prop, focus effect, spawn dialog signal.
* `web/src/components/TerminalTab.svelte` — bubbleCount derive,
  submitRichPrompt focus, terminal-host margin-bottom.
* `web/src/components/SpawnDialog.svelte` — state-driven.
* `web/src/state/spawnDialog.svelte.ts` — new singleton.
* `web/src/App.svelte` — mounts `<SpawnDialog />`.
* `web/src/components/TerminalRichPrompt.test.ts` — sibling
  mount + afterEach reset.

Pre-push gate (SPA portion): vitest 445/445 green;
`npm run check` 0 errors / 0 warnings; `npm run build` clean.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Four-part edit lands cleanly. The lift of SpawnDialog to a
state-driven singleton mounted at App root is the right call —
fixed-position backdrops anchored to the viewport were exactly
what the stacking-context maze was breaking. `bubbleCount` prop
threaded through `TerminalTab` → `TerminalRichPrompt` is a
minimal, focused shape.

**Commit clearance**: approved. Suggested subject:

```
Rich prompt: caret focus rules + Cmd+Enter retain + push terminal up + spawn dialog singleton (fullstack-a-4)
```

Push waits for Round-1 close. Pick up `fullstack-a-5` next
(editor cluster).
