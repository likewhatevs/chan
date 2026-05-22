# fullstack-a-89 — Rich prompt placeholder: switch from CSS overlay to CM6 placeholder extension (architectural fix; supersedes -a-84/-a-87)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Replace the CSS-overlay placeholder pattern from
`-a-24` with CodeMirror 6's built-in `placeholder`
extension. The CM6 extension renders the placeholder
INSIDE the editor's first line right at the cursor
position — no manual coordinate math.

## Reference

@@Alex 2026-05-22 (after `-a-84` + `-a-87`): cursor
still visibly offset from placeholder text. "how come
we cannot print the text in front of the cursor?"

The CSS overlay was always going to fight CM6's
internal coordinate system. Two iterations of
alignment patches (`-a-84` 10px X-offset, `-a-87`
line-height 1.8 match) didn't close the empirical
gap because the placeholder lives in a parallel
positioning system, not CM6's.

`-a-24`'s original "deliberate CSS overlay" choice
was made to avoid threading a `placeholder` prop
through the Wysiwyg/Source composer. But the cost
(repeated alignment patches that don't fully close)
exceeds the benefit.

This task EXPLICITLY OVERRIDES the
out-of-scope clauses from `-a-84` + `-a-87` that
said "Re-architecting CM6 placeholder via extension
is out of scope — deliberate CSS overlay per -a-24."
@@Alex's empirical feedback supersedes that choice.

## Fix shape

### 1. Import CM6's placeholder extension

```ts
import { placeholder } from "@codemirror/view";
```

### 2. Add to the extension list when buffer is empty

In the Wysiwyg/Source composer used by the rich
prompt, add `placeholder("Write a multi-line command
and Cmd+Enter")` to the extension list. CM6 renders
it as a widget at the cursor position when the
content is empty + hides automatically when the
user types.

### 3. Remove the CSS overlay

* Delete `.prompt-placeholder` CSS rule from
  `TerminalRichPrompt.svelte` (lines around 790-806).
* Remove the `<div class="prompt-placeholder">`
  wrapper from the markup (line ~582).
* Remove `{#if prompt.buffer === ""}` conditional
  guard for that element.

### 4. Wire the placeholder string

The rich prompt's placeholder string lives where
the CSS overlay rendered it today. Pass it as a
prop or hardcoded value to the Wysiwyg/Source
composer. Threading a `placeholder` prop through
the composer is the cost we deferred in `-a-24` —
pay it now.

### 5. Other surfaces

Audit if any OTHER surfaces use the same CSS-overlay
pattern (e.g. the editor's empty-state text). If
they exist + would benefit from the same switch,
note in task tail but defer unless tightly coupled.

## Acceptance

1. **Cursor + placeholder share the exact same
   position**: empty rich prompt with focus shows
   the cursor at the START of the placeholder text
   (or just before the first character). They look
   like one unit visually — cursor blinks at
   placeholder's start; text reads as if the user
   has already typed it.
2. **Hide on type**: typing replaces the placeholder
   per CM6's standard behavior.
3. **Re-appears on full delete**: standard CM6
   placeholder behavior.
4. **No regression on Cmd+Enter submit**: extension
   doesn't interfere with key handling.
5. **No regression on the rich prompt's other
   features**: rendering modes (wysiwyg vs source),
   theming, etc.

### Tests

Vitest pins for:
* `placeholder` extension imported.
* Placeholder string passed through to the composer.
* CSS overlay code removed.
* The expected placeholder text present in the
  extension config.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* Medium scope — extension wire-up + threading the
  placeholder prop + removing CSS overlay. ~30-60
  LOC.

## Authorization

Yes for:
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/editor/Wysiwyg.svelte` / `Source.svelte`
  (if the placeholder prop threading lands here).
* Tests + task tail + outbound.

## Numbering

This is `-a-89`.

## Out of scope

* Placeholder text content changes.
* Re-styling beyond CM6's default placeholder render
  (color / opacity / font — CM6's defaults should be
  fine; tweak only if visually wrong).
* Replacing the placeholder pattern in OTHER
  components beyond the rich prompt's first surface.

## Supersedes

* `-a-84` (X-offset CSS hack).
* `-a-87` (line-height match CSS hack).

Both shipped under the CSS-overlay architecture
which is being replaced. The fixes were not wrong —
they were the best the architecture allowed. The
empirical alignment gap survives because the
architecture itself was misaligned.
