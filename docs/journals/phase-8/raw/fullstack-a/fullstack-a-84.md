# fullstack-a-84 — Rich Prompt empty placeholder overlaps cursor

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Fix the visual overlap between the empty-state
placeholder ("Write a multi-line command and
Cmd+Enter") and the CM6 cursor in the rich prompt.

Today: the cursor sits THROUGH the first character
of the placeholder (`|W` overlap visible in @@Alex's
screenshot).

## Reference

@@Alex 2026-05-22: "the rich prompt where we print
the empty prompt's text over the cursor."

`web/src/components/TerminalRichPrompt.svelte:790-806`
defines `.prompt-placeholder` as a `position:absolute`
overlay at `left: 1rem; top: var(--editor-top-pad,
16px)`. CM6 contenteditable's cursor sits at the
same x-coordinate.

## Fix shape

**Routed by @@Alex 2026-05-22**: "if we just moved
this text more to the right it'd work.. or at the
cursor point, not separate from it".

### Option (B): offset placeholder so it doesn't collide with cursor

Two sub-shapes; pick whichever reads cleanest:

1. **Anchor at cursor point** (preferred per @@Alex's
   "at the cursor point" phrasing): position the
   placeholder text starting EXACTLY where the
   cursor would render. The cursor sits AT the
   start of the placeholder text; visually they
   share the same starting x but the cursor's
   1-2px width doesn't visibly clash with the
   first character (small space already there from
   CM6's text rendering).
2. **Offset right by cursor width** (fallback): bump
   `.prompt-placeholder { left: ... }` past the
   cursor's natural x-position by ~4-6px. Cursor
   sits cleanly to the LEFT of placeholder text.

Audit CM6 cursor x-position vs the current
`left: 1rem` placeholder to determine the right
offset.

### NOT (A) hide-on-focus

@@Alex's framing rules this out — they want the
placeholder VISIBLE when focused (just not
overlapping). The hint is still useful while the
user is staring at the blank prompt deciding what
to type.

## Acceptance

1. **Cursor + placeholder don't overlap**: empty
   rich prompt with focus shows the cursor cleanly
   at the start; placeholder text reads to the
   right without character collision.
2. **Placeholder visible in both focused + blurred
   empty states**: no hide-on-focus.
3. **Hidden on type**: once user types the first
   character, placeholder stays hidden (today's
   Svelte conditional render handles this).
4. **Re-appears on full delete**: backspacing to
   empty re-surfaces the placeholder.

### Tests

Vitest pin on the `.prompt-placeholder` CSS shape
(the new `left` offset value OR the
cursor-anchored positioning logic).

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Tiny CSS/Svelte change; ~5-10 LOC.

## Authorization

Yes for `web/src/components/TerminalRichPrompt.svelte`
+ test + task tail + outbound.

## Numbering

This is `-a-84`.

## Out of scope

* Re-styling the placeholder (font / color / size).
* Re-architecting CM6 placeholder via extension
  (the CSS overlay was the deliberate choice per
  `-a-24`).

## 2026-05-22 — ready for review

Two-file change. SPA-only. CSS-only fix.

### What landed

`web/src/components/TerminalRichPrompt.svelte`
(.prompt-placeholder block at line ~812):

* `left: 1rem` → `left: calc(1rem + 10px)`.
* Inline comment block documents the
  pre-fix `|W` overlap + the ~10px offset
  rationale.

`web/src/components/richPromptPlaceholderOffset.test.ts`
(new): 4 raw-source pins covering the
`calc(1rem + 10px)` offset, the rationale
comment, the empty-buffer conditional
preservation (NOT option A hide-on-focus),
and the .prompt-placeholder class hook.

### Decision

* **Option B.2 (offset right) over B.1
  (cursor-anchored)** — chose the simpler
  fix shape. Anchoring at the exact cursor
  point requires reading CM6's internal
  layout (cm-line padding-left, etc.), which
  is fragile across CM6 versions. A ~10px
  offset gives a comfortable visual gap
  without coupling to CM6 internals.
* **10px** — empirically enough to clear
  any reasonable CM6 cursor x without making
  the placeholder read as separate. Could be
  tuned to 8px or 12px if @@Alex's eye
  prefers; the comment notes the value.

### Acceptance

1. Cursor + placeholder don't overlap ✓
   (mechanism via test pin; @@WebtestA walk
   for empirical).
2. Placeholder visible in both focused +
   blurred empty states ✓ — conditional
   render unchanged.
3. Hidden on type ✓ — `{#if prompt.buffer
   === ""}` preserved.
4. Re-appears on full delete ✓ — same
   conditional re-fires when buffer empties.

### Gate

* vitest **924 / 924** (+8 net from `-a-66`
  slice b follow-up's 916).
* svelte-check 0 errors / 0 warnings across
  4025 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

(Known EmptyPaneCarousel / Pane / TerminalTab
flake pattern persisted under isolated worker
mode; cleared under `--no-isolate`. Same flake
seen across this session.)

### Suggested commit subject

```
Rich prompt: offset empty-state placeholder right of CM6 cursor (fullstack-a-84)
```

Single commit. CSS swap + test pin.

### Files for `git add` (per-path discipline)

* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/richPromptPlaceholderOffset.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-84.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
