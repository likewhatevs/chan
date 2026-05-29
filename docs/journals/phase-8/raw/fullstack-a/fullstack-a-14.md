# fullstack-a-14: Rich prompt re-open with bubble present focuses prompt input (should focus survey area)

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Make the bubble-present rule in `fullstack-a-4` ("if bubbles
present on open, caret lands in the survey area") survive
the re-open path. Today the cold-open works correctly but
re-opening the rich prompt while a bubble is still on
screen steals focus to the prompt input.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md)
rich-prompt cluster: "Rich prompt cursor focus on open".
`fullstack-a-4` set the rule (no-bubbles → prompt input;
bubbles present → survey area). @@WebtestA's lane-A sweep
on 2026-05-20 confirmed the cold-open path works on HEAD
but the re-open path doesn't:

> Open rich prompt; a survey bubble is present; close the
> rich prompt (without dismissing the bubble); re-open the
> rich prompt. The caret lands in the prompt input field
> instead of the survey area, contrary to the rule.

WebtestA's root-cause hypothesis (worth verifying):

> "The focus-effect grabs the prompt input before
> BubbleOverlay's bubbleCount catches up."

Likely a timing or ordering bug between the open-effect
focus call and the bubble-count derivation. Investigate.

## Acceptance criteria

* Cold-open path continues to work: no bubbles → caret in
  prompt input; bubbles present → caret in survey area.
* Re-open path obeys the same rule: bubbles still present
  → caret in survey area; bubbles dismissed in between →
  caret in prompt input.
* On dismiss of all bubbles, caret returns to the prompt
  input (the original rule).
* No regression on the Cmd+Enter retention from
  `fullstack-a-4` (caret stays after dispatch).

## How to start

1. Reproduce on the lane-A test server (URL in
   `event-architect-alex.md` 2026-05-20). Alt+Space to open,
   trigger a survey via the watcher events seeded in
   `/tmp/chan-test-phase8-wa/watcher-events/`, close + re-
   open without replying.
2. Look at the rich-prompt open-effect in
   `web/src/components/RichPrompt*` or wherever the open
   trigger lives. There is likely a `$effect` (Svelte 5) or
   `onMount` that calls `inputRef.focus()` unconditionally on
   open.
3. Either (a) make the focus decision wait for the bubble-
   count to settle (read it from the shared store at the
   moment of decision, not at mount-time), or (b) move the
   focus logic into the bubble-aware component so it sees
   the count synchronously.
4. Pin with a small SPA test if a focus-target assertion
   fits the shape; otherwise visual verification on the
   lane-A server is acceptable.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.

## 2026-05-20 — implementation note

Root cause confirmed by reading the focus stack against Svelte 5
mount order. The pre-fix flow on re-open with bubble present:

1. `openActiveTerminalRichPrompt` flips `prompt.open = true` and
   bumps `prompt.focusNonce`.
2. Svelte renders `TerminalRichPrompt` (the `{#if richPrompt?.open}`
   block remounts the component fresh).
3. Inside the prompt, the `{#key mode()}` block mounts a fresh
   `Wysiwyg` (or `Source`) child. The child's `onMount` calls
   `view.focus()` UNCONDITIONALLY (line 467 of `Wysiwyg.svelte`,
   line 177 of `Source.svelte`), grabbing DOM focus on the
   editor.
4. AFTER the child mounts, `TerminalRichPrompt`'s `$effect` runs.
   It reads `prompt.focusNonce` + `bubbleCount`. With
   `bubbleCount > 0` it early-returns — but the editor is
   ALREADY focused from step 3.
5. `BubbleOverlay.onWindowKeydown` bails on its `editableTarget`
   check (`closest("input,textarea,[contenteditable='true']")`
   matches the CM6 contenteditable host), so number keys type
   into the prompt buffer instead of replying the focused
   survey. This is exactly what @@WebtestA observed.

So the `bubbleCount > 0` gate in the rich-prompt effect doesn't
matter on re-open — the editor child has already stolen focus
by the time the gate runs. @@WebtestA's hypothesis ("the
focus-effect grabs the prompt input before BubbleOverlay's
bubbleCount catches up") was directionally right but pointed at
the wrong owner — it's the child editor's mount-time focus,
not the parent's effect, that wins the race.

Fix: gate the editor's mount-time focus on a new
`autoFocus?: boolean` prop (default true so all existing
callers — `FileEditorTab`'s file-editor case in particular —
keep their snap-to-focus-on-tab-open behaviour). The
`TerminalRichPrompt` passes `autoFocus={bubbleCount === 0}` to
both the `Wysiwyg` and `Source` child. Combined with the
existing `bubbleCount > 0 -> early return` in the parent's
focus effect, the bubble-present mount path now leaves the
editor unfocused, BubbleOverlay's window keydown takes over,
and the number reply lands on the focused survey.

The bubble-drop-to-zero path is unchanged: the parent's
`$effect` re-runs when `bubbleCount` changes, sees `0`, and
dispatches `wysiwygRef?.focusEnd()` (or `sourceRef?.focusAt`).
Caret returns to the prompt input — same behaviour as
`fullstack-a-4` defined for the dismissal case.

Why a prop instead of an effect-level blur: the editor's
mount focus runs synchronously inside the child's `onMount`,
BEFORE the parent's `$effect` (Svelte 5 fires child mount
before parent effects). Any parent-level blur would run AFTER
the editor focused, producing a one-frame flicker where the
CM6 caret renders then vanishes. Gating at the child's mount
keeps the focus state correct from the first paint.

Audit of the other call sites:

* `FileEditorTab.svelte`'s `<Wysiwyg .../>` and `<Source .../>`
  do not pass `autoFocus`, so they inherit the `true` default
  and keep their snap-to-focus behaviour when a file tab
  opens. No regression.
* `richPromptAutoFocus.test.ts` is a string-match test that
  checks for `void prompt.focusNonce`, `wysiwygRef?.focusEnd()`,
  `sourceRef?.focusAt(prompt.buffer.length)`, and
  `bind:this={sourceRef}`. None of those strings changed; the
  test stays green.

Cold-open with bubbles: same code path as re-open (both go
through the same mount). The acceptance criterion ("cold-open
with bubbles → survey area") was already at risk under the
prior fix; the autoFocus prop closes both cases together.

Files touched:

* `web/src/editor/Wysiwyg.svelte` — `autoFocus?: boolean` prop
  with `true` default; `view.focus()` gated.
* `web/src/editor/Source.svelte` — same prop + gate.
* `web/src/components/TerminalRichPrompt.svelte` — pass
  `autoFocus={bubbleCount === 0}` to both children.

Pre-push gate (SPA portion): vitest 475/475 green
(including the existing `richPromptAutoFocus` string-match
tests); `npm run check` 0 errors / 0 warnings;
`npm run build` clean.

To verify on the lane-A server (post-restart): seed a survey
event in `/tmp/chan-test-phase8-wa/watcher-events/`, open the
rich prompt → survey is rendered, no caret in prompt input;
press `1` → reply lands (BubbleOverlay handles the keystroke).
Close the rich prompt (Esc) without dismissing the bubble;
re-open via Alt+Space → caret still NOT in prompt input,
pressing `1` still routes to the survey. Dismiss / reply the
bubble → caret returns to the prompt input.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Excellent root-causing. The Svelte 5 mount order — child
`onMount` fires synchronously BEFORE the parent's `$effect`
— is exactly the kind of "the framework's ordering quietly
defeats the gate you wrote" trap that's easy to miss
without tracing through the actual lifecycle. @@WebtestA's
"focus-effect grabs the prompt input before
BubbleOverlay's bubbleCount catches up" was directionally
right but blamed the wrong owner; your audit corrects the
owner cleanly.

The autoFocus prop approach is right for the right reason.
You called it out explicitly: a parent-level blur would
run AFTER the child focused, producing a one-frame
flicker. Gating at the child's mount keeps the focus
state correct from the first paint. This is the kind of
"why a prop instead of an effect" reasoning that belongs
in the audit trail — it'll save someone else from
re-deriving it.

The default-true prop preserves backwards-compat for
`FileEditorTab`'s snap-to-focus-on-tab-open behaviour. The
`richPromptAutoFocus.test.ts` string-match test stays
green because none of the strings it checks were
touched — clean unrelated-test invariant.

Bonus catch: the original `fullstack-a-4` cold-open-with-
bubbles path was at risk under the prior fix. -14 closes
both the re-open AND the cold-open cases together. Good
that the acceptance criteria explicitly cover both.

Pre-push gate green. Lane-A verification path is well-
specified.

**Commit clearance**: approved. Suggested commit subject:

```
Editor: autoFocus prop gates Wysiwyg/Source mount-time focus; rich prompt leaves caret for bubble (fullstack-a-14)
```

Push waits for Round-1 close.

Carry on with `fullstack-a-15` (`.md.md` double extension)
next per the queue. Then -16 / -17, then -18 (newly cut —
wysiwyg-mode Cmd+Enter dispatch dropping silently; flagged
by @@FullStackB in -b-8).