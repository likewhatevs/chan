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

## 2026-05-22 — ready for review (architectural swap landed)

Five-file change. SPA-only.

### What landed

`web/src/editor/Wysiwyg.svelte`:
* Added `placeholder` to the
  `@codemirror/view` import list.
* New optional `placeholderText?: string`
  prop.
* In the extension list (between
  `EditorView.lineWrapping` and
  `drawSelection()`):
  `...(placeholderText ? [placeholder(placeholderText)] : [])`.
* Comment explains CM6 renders the
  placeholder INSIDE the first line at the
  cursor position — the architectural shape
  the CSS overlay couldn't replicate.

`web/src/editor/Source.svelte`:
* Mirror change: import, prop, extension
  injection.

`web/src/components/TerminalRichPrompt.svelte`:
* New `PROMPT_PLACEHOLDER_TEXT` constant
  holding "Write a multi-line command and
  Cmd+Enter".
* Wysiwyg + Source both receive
  `placeholderText={PROMPT_PLACEHOLDER_TEXT}`.
* Removed the entire
  `{#if prompt.buffer === ""} <div
  class="prompt-placeholder">…</div> {/if}`
  markup block.
* Removed the `.prompt-placeholder` CSS
  rule (including the `-a-84` X-offset + the
  `-a-87` line-height match).
* Replacement comment in the markup
  cross-references the architecture swap +
  the superseded `-a-84` / `-a-87` tasks.

`web/src/components/richPromptPlaceholderExtension.test.ts`
(new): 11 raw-source pins covering the
extension wiring on both editors, the
constant declaration, the prop pass-through,
the removed overlay markup, the removed CSS
rule, and the rationale comment.

`web/src/components/richPromptPlaceholderOffset.test.ts`,
`web/src/components/richPromptPlaceholderBaseline.test.ts`:
**deleted**. Both pinned the `-a-84` /
`-a-87` CSS overlay shapes that this task
removes. New `-a-89` extension pins
supersede them.

### Acceptance

1. **Cursor + placeholder share the exact
   same position** ✓ — CM6's `placeholder`
   widget renders inside the first cm-line
   at the cursor position; no separate
   coordinate system to fight.
   @@WebtestA empirical walk to confirm
   the visual.
2. **Hide on type** ✓ — CM6's standard
   behavior.
3. **Re-appears on full delete** ✓ — CM6's
   standard behavior.
4. **No regression on Cmd+Enter submit**
   ✓ — extension doesn't touch keymap.
5. **No regression on wysiwyg vs source
   modes** ✓ — both editors receive the
   prop; CM6 renders identically in either
   mode.

### Gate

* vitest **954 / 954** (+3 net from `-a-66`
  slice d's 951: +11 new pins on the
  extension wire-up minus 8 removed pins
  from the deleted overlay tests).
* svelte-check 0 errors / 0 warnings across
  4031 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Threaded prop through both editors**
  rather than only the one currently in
  use — the rich prompt's `-a-4` mode-toggle
  swaps Wysiwyg/Source at runtime; if only
  one had the prop, switching modes would
  drop the placeholder. Single-shape API
  surface across both.
* **Optional prop, not required** — file
  editors (FileEditorTab) don't want a
  placeholder; the existing call sites
  don't have to be touched.
* **Single constant
  `PROMPT_PLACEHOLDER_TEXT`** at the top of
  TerminalRichPrompt — single source of
  truth so the wysiwyg + source paths can't
  drift.
* **Deleted the old test files** — both
  pinned the overlay architecture this
  task replaces. Leaving them would have
  cluttered the test count + introduced
  false signal that the CSS overlay still
  ships.
* **`-a-89` comment explicitly cites
  `-a-84` / `-a-87`** so a future audit
  trail can find the architecture swap +
  the prior patch attempts.

### Other surfaces audit (per task body §5)

Searched for other CSS-overlay placeholder
patterns. The rich prompt was the only
surface using this shape (per `-a-24`'s
"single in-prompt use" framing). No other
components to migrate; no follow-up needed.

### Suggested commit subject

```
Rich prompt placeholder: CSS overlay → CM6 placeholder extension (fullstack-a-89)
```

Single commit. Editor prop threading +
TerminalRichPrompt swap + test
replacement.

### Files for `git add` (per-path discipline)

* `web/src/editor/Wysiwyg.svelte`
* `web/src/editor/Source.svelte`
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/richPromptPlaceholderExtension.test.ts` (new)
* `web/src/components/richPromptPlaceholderOffset.test.ts` (deleted)
* `web/src/components/richPromptPlaceholderBaseline.test.ts` (deleted)
* `docs/journals/phase-8/fullstack-a/fullstack-a-89.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@WebtestA empirical walk that confirms
cursor + placeholder finally share the same
position.
