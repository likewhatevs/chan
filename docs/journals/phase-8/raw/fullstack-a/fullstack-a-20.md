# fullstack-a-20: Cmd+Enter in wysiwyg mode double-dispatches text to the terminal

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Wysiwyg-mode Cmd+Enter from the rich prompt must dispatch
the buffer **once** to the focused terminal. Today it
dispatches twice (`pwd` arrives as `pwdpwd`) because the
fix in `fullstack-a-18` introduced a double-trigger path.

## Background

Regression from `fullstack-a-18` (committed and currently
in the wave-3 working-tree set, not yet pushed). @@Alex
caught it the same day: typing `pwd` (no Enter) then
pressing Cmd+Enter results in `pwdpwd` showing in the
terminal grid.

### Root cause

Two handlers both fire on Cmd+Enter when the rich prompt
is in wysiwyg mode:

1. **Wysiwyg's CM6 keymap** at `web/src/editor/Wysiwyg.svelte`
   ~line 415-420 has
   `{ key: "Mod-Enter", run: () => { onSubmit?.(); return true; } }`.
   `-a-18` threaded `onSubmit={submit}` from the wrapper,
   so this now calls the real `submit()`. CM6's
   `runHandlers` returns true → calls `event.preventDefault()`
   on the DOM event. **It does NOT call `event.stopPropagation()`.**
2. **Wrapper's `onKeydown` at `TerminalRichPrompt.svelte:118-122`**
   tests `e.key === "Enter" && (e.metaKey || e.ctrlKey) && !e.altKey
   && !e.shiftKey`, calls `e.preventDefault()` + `e.stopPropagation()`,
   then calls `submit()`. **It does NOT check
   `e.defaultPrevented`.**

The wysiwyg-mode pre-`-a-18` state worked the same way:
keymap fired with `onSubmit=undefined` (no-op) + `return true`
→ preventDefault → bubbled to wrapper → wrapper called
submit(). Net result: one dispatch. The wrapper was
silently covering the broken thread.

When `-a-18` connected `onSubmit={submit}`, both layers
now invoke the real `submit()` and we get the double
dispatch.

### Source mode is unaffected

`Source.svelte`'s keymap has no Mod-Enter binding, so the
chord bubbles cleanly to the wrapper. One dispatch. The
asymmetry is the wysiwyg-only Mod-Enter handler.

## Fix

Single-line: add `if (e.defaultPrevented) return;` at the
top of `onKeydown` in `TerminalRichPrompt.svelte`, BEFORE
the Escape branch. Standard "respect what children
already handled" event-handling discipline.

```typescript
function onKeydown(e: KeyboardEvent): void {
  if (e.defaultPrevented) return;     // NEW
  if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    onClose();
    return;
  }
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey) {
    e.preventDefault();
    e.stopPropagation();
    submit();
  }
}
```

After the guard:

* **Wysiwyg mode**: Wysiwyg keymap runs `submit()` once,
  returns true → preventDefault → wrapper sees
  `defaultPrevented` → bails. **One dispatch.** ✓
* **Source mode**: Source has no Mod-Enter, event reaches
  wrapper with `defaultPrevented === false`, wrapper
  handles it. **One dispatch.** ✓
* **Escape**: similarly, if some child component cancels
  Escape (and calls preventDefault) the wrapper respects
  that. Likely no regression on Escape since no child
  currently does this, but the guard is correct discipline.

## Acceptance criteria

* In wysiwyg mode, typing `pwd` then Cmd+Enter results in
  `pwd` in the terminal grid (single occurrence, NOT
  `pwdpwd`).
* In source mode, same behaviour as today: `pwd` →
  Cmd+Enter → `pwd` in terminal. No regression.
* Multi-line buffers dispatch once in both modes.
* `fullstack-a-18`'s wysiwyg-mode "Cmd+Enter dispatches"
  acceptance still holds — the dispatch still fires, just
  no longer twice.
* Test pin: `TerminalRichPrompt.test.ts` (or wherever the
  wrapper-level keydown test lives) gets a new case that
  asserts a `defaultPrevented` Cmd+Enter event does NOT
  call `submit()`.

## How to start

1. Open `web/src/components/TerminalRichPrompt.svelte`,
   add the `if (e.defaultPrevented) return;` line at the
   top of `onKeydown`.
2. Pin with a test case in `TerminalRichPrompt.test.ts`.
3. Visual verification on the lane-A test server: open
   rich prompt (default wysiwyg mode), type `pwd`,
   Cmd+Enter, observe single `pwd` in the terminal grid.
   Then flip to source mode (right-click → "Show source
   code"), repeat, confirm same one-dispatch behaviour.
4. Pre-push gate.

## Coordination

* **Highest priority** — Alex-visible regression flagged
  during their return. Slot ahead of `fullstack-a-19`.
* @@WebtestA verifies on lane-A drive once landed.
* @@FullStackA: this regression is yours from -a-18; fix
  it on top of the in-tree -a-18 change (you have it
  uncommitted at the moment).

## 2026-05-20 — implementation note

Confirmed root cause exactly as the task spec described.
`-a-18` connected `onSubmit={submit}` to the Wysiwyg child;
Wysiwyg's CM6 Mod-Enter keymap entry now does real work
instead of a `?.()` no-op. CM's keymap runner calls
`event.preventDefault()` on a handler that returns true, but
NOT `stopPropagation()`, so the event still bubbles. The
wrapper's `onKeydown` didn't inspect `defaultPrevented` and
called `submit()` a second time.

Fix exactly as the spec sketched: `if (e.defaultPrevented)
return;` at the top of `onKeydown`. Resolves both modes
cleanly:

* Wysiwyg: CM keymap fires `submit()` once, preventDefault'd
  event bubbles to wrapper, wrapper bails on the guard. One
  dispatch.
* Source: no Mod-Enter binding on Source's keymap; the event
  reaches the wrapper unhandled, `defaultPrevented` is false,
  wrapper handles it. One dispatch.

Bonus correctness for Escape: if a child ever cancels Escape
via `preventDefault`, the wrapper now respects that
intent. No current consumer does, but the guard is the right
discipline.

Test pin: new case in `TerminalRichPrompt.test.ts`
("Cmd+Enter with defaultPrevented does NOT re-submit
(fullstack-a-20)") — mounts the prompt in wysiwyg mode,
dispatches a `keydown` with `preventDefault()` already
called on it, asserts `onSubmit` is NOT invoked. The
existing Cmd+Enter test (line 133) stays green; jsdom
doesn't mount the CM6 Wysiwyg deeply enough to actually
fire the CM keymap, so that test exercises the
wrapper-only path against `defaultPrevented=false`.

Files touched:

* `web/src/components/TerminalRichPrompt.svelte` —
  `onKeydown` adds the `defaultPrevented` guard with a
  comment recording the wysiwyg / source asymmetry.
* `web/src/components/TerminalRichPrompt.test.ts` — new
  regression test pinning the guard's behaviour.

Pre-push gate (SPA portion): vitest 481/481 green (+1 from
the new test); `npm run check` 0 errors / 0 warnings;
`npm run build` clean.

To verify on the lane-A server (post-restart): open the rich
prompt (default wysiwyg mode), type `pwd`, Cmd+Enter →
terminal grid shows ONE `pwd`, not `pwdpwd`. Flip to source
mode (right-click → "Show source code"), repeat → still one
`pwd`. Multi-line buffers also single-dispatch.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Single-line fix in exactly the right place. The
`defaultPrevented` guard at the top of `onKeydown` is the
correct event-handling discipline that was missing — once
a child consumes an event (CM6 keymap calling
preventDefault via `return true`), the wrapper has to
respect that. The post-fix table in the implementation
note (Wysiwyg / Source / Escape) covers the three paths
cleanly.

Bonus correctness for Escape is noted — no current child
cancels Escape via preventDefault, but if one ever does
(e.g. a future modal that wants to handle its own dismiss
without closing the rich prompt), the guard already
respects that intent.

The new test pin
("Cmd+Enter with defaultPrevented does NOT re-submit")
exercises the wrapper-only path against the
`defaultPrevented=true` case. The pre-existing line-133
test exercises the `defaultPrevented=false` (source-mode-
equivalent) path. The two together pin both branches of
the new guard.

Pre-push gate green (vitest 481/481, +1 from baseline;
check + build clean).

**Commit clearance**: approved. Suggested commit subject:

```
TerminalRichPrompt onKeydown: respect defaultPrevented to avoid double-dispatch on wysiwyg Cmd+Enter (fullstack-a-20)
```

Push waits until end of Round 2 (no Round-1 binary cut
per the restructure).

Commit -20 ahead of -19 in the wave-3 set since it fixes
the regression introduced by -18; ordering keeps the
git-log story linear (regression introduced + fixed before
any other wave-3 commit lands).