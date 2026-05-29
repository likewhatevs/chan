# fullstack-a-18: Wysiwyg-mode Cmd+Enter from rich prompt silently does nothing

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Make Cmd+Enter (Mod-Enter) from the rich prompt's
**wysiwyg** mode dispatch the buffer to the focused
terminal, the same as source-mode does today. Right now
the chord is silently consumed in wysiwyg mode and the
user has no way to dispatch without flipping to source.

## Background

Caught by @@FullStackB during the `fullstack-b-8` (Cmd+Enter
first-char swallow) root-cause investigation:

> The original wysiwyg-mode dispatch path is independently
> broken: `TerminalRichPrompt` doesn't pass an `onSubmit`
> to the `<Wysiwyg>` child, so the Wysiwyg keymap's
> `{ key: "Mod-Enter", run: () => { onSubmit?.(); return true; } }`
> consumes Cmd+Enter without doing anything. The dispatch
> path works in source mode because Source's keymap has no
> Mod-Enter binding and the event bubbles to the wrapper.

So the chord registers (Wysiwyg's keymap returns `true`,
stopping bubble) but the `onSubmit` callback is `undefined`,
making the chord a no-op. In source mode the chord goes
unhandled at the editor level, bubbles to the wrapper, and
the wrapper's submit logic fires.

User-visible symptom: in wysiwyg mode (the default mode for
non-source rich-prompt entries), pressing Cmd+Enter does
nothing. Users haven't filed this as a bug yet because the
source-mode workaround works; once anyone leans on wysiwyg
mode for any length, the bug surfaces.

## Acceptance criteria

* Cmd+Enter from the rich prompt's wysiwyg mode dispatches
  the buffer to the focused terminal, identical to source
  mode's behaviour today.
* Cmd+Enter from source mode continues to work (no
  regression).
* The caret-retention rule from `fullstack-a-4` still
  applies after wysiwyg dispatch — caret stays in the
  prompt; user can continue typing.
* The autoFocus rule from `fullstack-a-14` is not
  affected — bubble-present means caret isn't grabbed by
  the prompt; once dismissed, caret returns.
* No new tests required if there's no testable seam; a
  vitest pin that asserts `<Wysiwyg>` receives the
  `onSubmit` prop would be nice if the test harness
  supports it.

## How to start

1. Open `web/src/components/TerminalRichPrompt.svelte`.
   Find the `<Wysiwyg>` instantiation site (probably near
   the `{#if mode() === "wysiwyg"}` block).
2. Add `onSubmit={dispatchToTerminal}` (or whatever the
   wrapper-level dispatch function is named — likely
   `submit` / `onSubmit` / `dispatch` in the same file).
3. Verify the source-mode call site already threads the
   same callback; if it doesn't, the wrapper's
   keydown-handler does the work. Either way, parity
   between modes is the goal.
4. Visual verification on @@WebtestA's lane-A test server:
   open the rich prompt, flip to wysiwyg mode if not
   already there, type a command, Cmd+Enter, observe the
   terminal grid receive the command.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.
* Coordinate with `fullstack-a-14` if its commit hasn't
  landed yet — the autoFocus prop change touched the
  same Wysiwyg child instantiation in
  TerminalRichPrompt.

## 2026-05-20 — implementation note

Confirmed the root cause exactly as @@FullStackB described.
`Wysiwyg.svelte`'s extension stack includes a high-precedence
`Mod-Enter` entry (around line 415-420):

```ts
{ key: "Mod-Enter", run: () => { onSubmit?.(); return true; } }
```

Returning `true` from a CM6 keymap command consumes the
event, so it doesn't bubble. With no `onSubmit` prop passed,
`onSubmit?.()` is a no-op — the chord registers, consumes the
event, and does nothing visible.

`Source.svelte`'s editor doesn't have a Mod-Enter handler, so
the keystroke bubbles up to the wrapper, whose `onKeydown`
(line 118-122 of `TerminalRichPrompt.svelte`) catches Cmd+Enter
and calls `submit()`. That's why source mode works while
wysiwyg silently drops.

Fix: thread the wrapper's local `submit` function as
`onSubmit={submit}` on the `<Wysiwyg>` instantiation site.
`submit()` is the one-liner that calls the prop's `onSubmit`
with `prompt.buffer`, which TerminalTab wires to
`submitRichPrompt` (which sends the buffer to the PTY +
bumps focusNonce so the caret retention from `fullstack-a-4`
still applies).

No regression on the source-mode path — that goes through
the wrapper's `onKeydown` and remains unchanged.

`fullstack-a-14`'s `autoFocus={bubbleCount === 0}` on the
same Wysiwyg site is preserved alongside the new prop. The
two props compose without interaction (autoFocus gates
mount-time focus, onSubmit threads the Cmd+Enter dispatch).

Wysiwyg's existing test (in particular `richPromptAutoFocus`
string-match) does not reference `onSubmit` so no test edit
needed. The wrapper-side keydown test for Cmd+Enter
(`TerminalRichPrompt.test.ts` already covers the source-mode
path; the wysiwyg-mode path would need a Wysiwyg keymap-level
test that the existing harness doesn't yet support — visual
verification on lane-A is the practical bar).

Files touched:

* `web/src/components/TerminalRichPrompt.svelte` — single
  prop addition (`onSubmit={submit}` on the Wysiwyg child).

Pre-push gate (SPA portion): vitest 480/480 green;
`npm run check` 0 errors / 0 warnings; `npm run build` clean.

To verify on the lane-A server (post-restart): open the rich
prompt (default mode is wysiwyg), type `echo hello`, press
Cmd+Enter. Terminal grid above shows the command typed in +
output. Then flip the prompt to source mode (right-click →
"Show source code"), repeat — also works as before.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

One-line fix. The root-cause confirmation against
@@FullStackB's diagnosis is clean, and the why-source-works-
but-wysiwyg-doesn't trace through the CM6 keymap precedence
+ event bubbling is the right shape for the audit trail.
The two props (`autoFocus` from -14 and `onSubmit` from
this task) compose without interaction — explicit
documentation of that helps future readers.

Pre-push gate green (vitest 480/480, including the new
+5 tests that landed in -b-8 alongside).

**Commit clearance**: approved. Suggested commit subject:

```
TerminalRichPrompt: thread submit() to Wysiwyg so Cmd+Enter dispatches in both modes (fullstack-a-18)
```

Push waits for Round-1 close.

Carry on with the wave-3 chord-table drift cleanup, cut
next as `fullstack-a-19`.