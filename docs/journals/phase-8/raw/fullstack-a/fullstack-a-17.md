# fullstack-a-17: Cmd+K p (spawn terminal) steals rich-prompt focus to xterm-helper-textarea

Owner: @@FullStackA
Date: 2026-05-20

## Goal

When the user triggers Cmd+K → p to spawn a new terminal,
the newly-mounted xterm-helper-textarea must not steal focus
from the rich prompt input if the rich prompt is the
currently focused surface. The chord opens the terminal in a
pane; focus rules from `fullstack-a-4` still apply (rich
prompt stays focused if the user was typing there).

## Background

Side observation from @@WebtestA's Round-1 sweep on
2026-05-20:

> Cmd+K → p path: the newly-spawned terminal's
> `xterm-helper-textarea` steals focus from the rich prompt
> input.

Filed in [`../phase-8-bugs.md`](../phase-8-bugs.md). Family
of the `fullstack-a-4` rich-prompt focus rules: "consistent
UX — opening a surface doesn't steal focus from the rich
prompt input when the user is typing there."

## Acceptance criteria

* User has rich prompt focused + typing. Triggers Cmd+K → p.
  A new terminal pane appears. Focus stays in the rich
  prompt input.
* User has the editor focused. Triggers Cmd+K → p. The
  new terminal takes focus as today (no regression on the
  primary-spawn-when-no-prompt case).
* The spawned terminal's xterm.js instance is functional
  the moment the user clicks into it (no input loss from
  delayed focus).

## How to start

1. Reproduce on the lane-A test server (URL in
   `event-architect-alex.md` 2026-05-20).
2. Find the Cmd+K → p handler. Likely in
   `web/src/state/shortcuts.ts` and the pane-mode dispatch
   in `web/src/App.svelte`.
3. Find the terminal-mount code path that calls
   `xtermHelperTextarea.focus()` (or equivalent). It is
   probably unconditional today. Gate it on "was the rich
   prompt focused at spawn-time?" and skip the focus call
   in that case.
4. Cross-reference `fullstack-a-4` — the rich prompt's
   open-effect already reads focus state; mirror the same
   pattern for the spawn-time decision.
5. Pin with a small test if a testable seam exists;
   otherwise visual verification on the lane-A server.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.

## 2026-05-20 — implementation note

Root cause traced into `TerminalTab.svelte`'s focus effect (line
170 pre-fix):

```ts
$effect(() => {
  if (!focused) return;
  queueFit();
  setTerminalActivity(tab, false);
  sendFocusState();
  queueMicrotask(() => term?.focus());
});
```

This fires whenever the terminal tab transitions into the
focused state (active tab on the focused pane). On Cmd+K p
against a pane without a terminal:

1. `showOrSpawnRichPromptInFocusedPane` spawns a new terminal
   tab via `openTerminalInPane` and sets `activeTabId` to it.
2. Then calls `openActiveTerminalRichPrompt`, which sets
   `tab.richPrompt = { open: true, focusNonce: 1, ... }` (or
   bumps focusNonce on an existing one).
3. Svelte renders. `TerminalTab` mounts. The effect above
   fires (`focused` is true). `queueMicrotask(term?.focus())`
   schedules an xterm focus.
4. Inside `TerminalTab`, `{#if tab.richPrompt?.open}` mounts
   `TerminalRichPrompt`. Its open-effect (driven by
   `focusNonce`) schedules an editor focus via `tick()`.
5. Both focus calls land in the same microtask drain. xterm's
   focus and the editor's focus race; the user-reported
   outcome is xterm wins because `xterm.focus()` synchronously
   focuses `xterm-helper-textarea`, whereas the editor focus
   path waits a `tick()` first.

`fullstack-b-8`'s `blurTerminalHelperTextarea()` in
`openActiveTerminalRichPrompt` does NOT cover this case:
it blurs the CURRENT active element, but at the time it runs
xterm hasn't mounted yet (it mounts on the next Svelte tick).
There's nothing to blur up front.

Fix: gate the xterm focus on `tab.richPrompt?.open`. When the
rich prompt is open, bump `focusNonce` instead of focusing
xterm. The rich prompt's open-effect re-runs and lands the
caret on the editor.

Two cases this addresses cleanly:

1. **Cmd+K p races a fresh terminal mount** (the reported
   bug). Rich prompt is open at the moment this effect runs
   → bump focusNonce → editor focused. xterm doesn't grab the
   keystrokes.
2. **User clicks back to a pane whose rich prompt was already
   open**. `focused` transitions to true again; without this
   gate the existing code would `queueMicrotask(term?.focus())`
   and silently steal focus from the editor. With the gate,
   the same focusNonce bump re-focuses the editor. Bonus
   coverage: the task only flagged the Cmd+K p race, but the
   pane-switch-return regression mode was latent on the same
   path.

The `queueMicrotask` boundary keeps the `tab.richPrompt?.open`
read out of the `$effect`'s reactive tracking, so changes to
`richPrompt.open` don't re-fire this effect — only `focused`
changes do (which is what the original effect tracked).

No regression on the "no rich prompt" path: with
`tab.richPrompt?.open` falsy, the `term?.focus()` call fires
as before.

Files touched:

* `web/src/components/TerminalTab.svelte` — gate the focus
  effect on `tab.richPrompt?.open`; bump focusNonce when the
  prompt has the floor.

Pre-push gate (SPA portion): vitest 480/480 green;
`npm run check` 0 errors / 0 warnings; `npm run build` clean.

To verify on the lane-A server (post-restart): focus a pane
that has no terminal (e.g. the file-browser pane), press
Cmd+K then p → new terminal spawns + rich prompt opens →
caret lands in the rich prompt editor, NOT in the xterm grid.
For the bonus case: open rich prompt on pane A, click pane B
to focus it, click pane A back → caret returns to the rich
prompt editor.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Sharp diagnosis. The previous fix (`fullstack-b-8`'s
`blurTerminalHelperTextarea`) covers the OPEN-side race
but doesn't help here because at the moment Cmd+K p fires
the xterm element hasn't mounted yet — there's nothing to
blur. Gating the xterm focus call on `tab.richPrompt?.open`
and bumping `focusNonce` instead is the right fix because
it cooperates with the rich prompt's open-effect rather
than fighting it.

The bonus catch (pane-switch-return regression on the same
path) is the kind of "while I'm in this code I noticed
there's a latent case" finding that the audit trail
captures cleanly. Glad the fix covers both.

The `queueMicrotask` boundary keeping `tab.richPrompt?.open`
out of the effect's reactive tracking is correct reasoning
— otherwise the effect would re-fire on every open/close,
which would compound the race instead of fixing it.

Pre-push gate green.

**Commit clearance**: approved. Suggested commit subject:

```
TerminalTab focus effect: gate xterm focus on rich prompt closed; bump focusNonce when open (fullstack-a-17)
```

Push waits for Round-1 close.