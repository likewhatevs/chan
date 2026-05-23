# fullstack-a-89b — Rich prompt cursor / placeholder visible Y-misalignment (3rd-round follow-up; empirical-first)

Owner: @@FullStackA
Cut: 2026-05-23 by @@Architect
Status: dispatched
Priority: HIGH (3rd round on user-flagged UX bug)

## Goal

@@Alex 2026-05-23 (exact spec — literal):

```
{cursor}{space}{default-text}
```

* Cursor on the left.
* Single space after cursor.
* Default placeholder text immediately after the
  space.
* All on the SAME ROW.

Behavior:
* If user types → space + default text disappear in
  favor of what they typed.
* If user deletes the input whole → default text
  appears again.

This is canonical CM6 placeholder behavior +
should be a 1-3 line CSS fix to undo whatever's
pushing the placeholder to a separate row.

## Reference

@@Alex 2026-05-23: "im still very dissatisfied with
this cursor for the rich prompt that we cannot get
right" — 3rd round of feedback.

Screenshot shows the cursor `|` sitting visibly
HIGHER than the placeholder text baseline. The
cursor's top edge is ABOVE the placeholder text;
the cursor's bottom is at roughly placeholder
baseline.

### Saga history (all shipped, none closed visually)

* `-a-84` (3869a07): CSS overlay X-offset 10px.
* `-a-87` (0bcfbe7): CSS overlay line-height
  1.5 → 1.8 to match cm-line.
* `-a-89` (5845fa0): switched from CSS overlay
  to CM6 `placeholder` extension. @@WebtestA
  walked HOLD because cursor + placeholder
  matched at x-axis BUT y-axis still showed a
  ~12px gap (per their reported coords:
  cursor y=464.5, placeholder y=476).

## Empirical-first directive

Per the architect-side lesson from the Drafts saga
(EMPIRICAL > code-review when feature doesn't
visibly work):

### Mandatory diagnostic steps

1. **Open chan in a browser**. Build fresh
   binary first (pkill + cargo build + npm build
   + restart per `feedback_fresh_binary_rewalks`).
2. **Open browser devtools**. Inspect the empty
   rich prompt.
3. **Capture exact pixel positions** for:
   * `.cm-cursor` (or `.cm-cursor-primary`):
     top, left, width, height.
   * `.cm-placeholder`: top, left, width, height,
     line-height, baseline.
   * `.cm-line` (the line containing both):
     top, height, padding-top, padding-bottom,
     line-height.
4. **Identify the exact pixel delta** between
   cursor visual top edge and placeholder text
   visual top edge.
5. **Identify the root cause** from the pixel
   measurements. Candidates:
   * cm-line padding pushing the placeholder text
     below the cursor's top edge.
   * Placeholder font-size + line-height creates
     a smaller box than cm-line's natural height.
   * CM6's default cursor styling extends beyond
     the typeable area.
   * CSS reset / inherited line-height conflict.
6. **Fix the actual cause** identified empirically.

### Fix shape (TBD per audit)

Likely one of:
* `.cm-cursor { height: 1em; vertical-align:
  baseline }` to shrink cursor to text height.
* `.cm-line { padding-top: 0 }` to remove top
  padding causing the offset.
* `.cm-placeholder { line-height: <match cm-line> }`
  with explicit pixel value matching cursor height.
* A wrapping element style change.

## Acceptance

1. **Cursor + placeholder share visible Y-axis
   baseline** — measured empirically in browser
   devtools post-fix. Cursor top edge aligns
   (within 1px) with placeholder text top edge,
   OR cursor baseline aligns with placeholder
   text baseline.
2. **Visually**: cursor + placeholder text look
   like ONE unit — the cursor blinks at the
   exact starting position of the placeholder
   text, as if the placeholder were already-
   typed content.
3. **Screenshot confirmation** required: take a
   screenshot in browser at 1x DPI and verify
   visually before declaring HOLD.
4. **No regression** on hide-on-type / re-show
   on full delete.

### Tests

Vitest pin on whatever CSS landed + a brief
empirical-walk note in the task tail with the
pixel measurements before + after.

### Gate

`npm test` / `check` / `build` green + browser
visual verification.

## Coordination

* @@FullStackA. SPA-only.
* 3rd round on this UX bug — go DEEP empirically
  before committing the fix. Don't trust audit-
  grep alone.
* @@WebtestA can pair on the empirical
  measurement step if useful.

## Authorization

Yes for `web/src/components/TerminalRichPrompt.svelte`
+ `web/src/editor/*.svelte` if needed + CSS module
+ tests + task tail + outbound.

## Numbering

This is `-a-89b` (filename for clarity; conceptual
slice 2 of `-a-89`).

## Out of scope

* Re-architecting away from CM6 placeholder extension
  (`-a-89` is correct; we just need to close the
  visual gap).
* Re-styling beyond the alignment fix.

## Supersedes

* Nothing — additive on `-a-89`.

## Reverse: what NOT to do this round

* DON'T guess at line-height / top / padding
  values without measuring first.
* DON'T ship without a browser screenshot
  verifying the visual.
* DON'T claim HOLD until the visual matches
  the "cursor at the start of text" intent.

## 2026-05-23 — Empirical-first investigation + fix landed

Per the architect's mandatory empirical-first
directive. Built fresh binary, spun up
`/tmp/chan-89b` test drive at port 8787, opened
the rich prompt in Chrome, measured pixel
positions via `getBoundingClientRect()` +
`getComputedStyle()`.

### Pre-fix pixel measurements

Captured against the running binary (commit
`0f3a489` baseline):

| element        | top     | bottom    | height | line-height |
|----------------|---------|-----------|--------|-------------|
| .cm-cursor     | 717.5   | 736.5     | 19.0   | 22.4px (inherited) |
| .cm-placeholder| 713.0   | 741.8     | 28.8   | 28.8px (= .cm-line) |
| .cm-line       | 713.0   | 741.8     | 28.8   | 28.8px |

Delta: cursor top is **+4.5px below** placeholder
box top; cursor bottom is **−5.3px above**
placeholder box bottom. Cursor visually floats
INSIDE the line box, with its top edge **above**
the placeholder text top (cap-height ≈ +6.4px
from line top → text top ≈ 719.4; cursor top
717.5 → cursor sticks ~2px above the visible
"W" of "Write").

### Root cause

CM6's `coordsAtPos(0)` returns the rect at the
caret position based on the **font's natural
line-box** (font-size × ~1.2 = 19.2px for 16px
font). The cursor's height is that natural
line-box.

The placeholder is rendered as `<span
class="cm-placeholder">` inside `.cm-line`. The
span inherits `.cm-line`'s `line-height: 1.8`
(28.8px) and uses CM6 baseTheme's
`vertical-align: top`. The span's box is
28.8px tall, and the text inside sits at the
top of that box (because vertical-align: top).

Cursor extends from natural line-box top (slightly
above text top) to natural line-box bottom
(line baseline). Placeholder text sits at the
top of a 28.8px box, which is BELOW the cursor's
natural line-box top.

Hence the visible misalignment: cursor's top
sticks above the placeholder text's top.

### Fix shape

Two minimal changes:

1. **`PROMPT_PLACEHOLDER_TEXT`** prefixed with
   a single space (` Write a multi-line command
   and Cmd+Enter`). Satisfies @@Alex's literal
   `{cursor}{space}{default-text}` spec.
2. **CSS scoped to `.rich-prompt
   .cm-placeholder`**:
   ```css
   line-height: 1.2;
   vertical-align: middle;
   ```
   Collapses the placeholder's inline-block to
   match the cursor's natural line-box (1.2 ×
   16px = 19.2px) + centers the placeholder text
   vertically so its top aligns with the cursor
   top (sub-pixel: +0.10px top, −0.09px bottom).

### Post-fix pixel measurements

| element        | top     | bottom    | height |
|----------------|---------|-----------|--------|
| .cm-cursor     | 719.29  | 738.29    | 19.0   |
| .cm-placeholder| 719.19  | 738.38    | 19.20  |

Delta: +0.10px top, −0.09px bottom. **Sub-pixel
alignment**. Cursor + placeholder share the same
Y bounding box.

X-axis: cursor.left 46.08 < placeholder text
start (placeholder.left 41.63 + leading space
glyph). Visually `|<space>Write…` per spec.

### Visual confirmation

Zoomed screenshot of the rich prompt at 1x DPI:
cursor `|` blinks at the start of the line, a
visible space follows, then "Write a multi-line
command and Cmd+Enter" placeholder text. Cursor
and placeholder text share the same baseline +
top edge.

### Files touched

* `web/src/components/TerminalRichPrompt.svelte`
  * `PROMPT_PLACEHOLDER_TEXT` prefix.
  * `:global(.rich-prompt .cm-placeholder)` CSS
    rule with the empirical-measurement
    comment block (root-cause + before/after
    metrics).
* `web/src/components/richPromptCursorAlignment.test.ts`
  (new) — 7 architectural pins for the leading
  space + the scoped CSS rule + the metric-
  preserving comment block.
* `web/src/components/richPromptPlaceholderExtension.test.ts`
  * `PROMPT_PLACEHOLDER_TEXT` pin updated for
    the leading space.

### Decisions

* **Leading space in the constant**, not via
  CSS padding-left. Tested both: padding-left
  shifts the cursor INTO the padding area
  (cursor.left moves right by the padding
  amount because CM6 measures the caret at
  the placeholder's first-text position). The
  leading space character keeps the cursor at
  the line's natural left edge + pushes the
  visible glyphs right by one space-width.
  Cleanest reading of the spec literal
  `{cursor}{space}{default-text}`.
* **Scoped CSS rule** (`.rich-prompt
  .cm-placeholder`), not a global override.
  CM6's `placeholder` extension is also used
  in the file editor (Wysiwyg + Source); the
  rich prompt's font sizing + line-height
  context is different from the file
  editor's, so scoping prevents collateral
  visual changes in other CM6 surfaces.
* **`line-height: 1.2`** (relative) not
  `19.2px` (absolute). Relative scales with
  font-size if the user ever increases the
  rich prompt's font (e.g. accessibility zoom).
* **`vertical-align: middle`** not `top`.
  Tested both; `middle` gives sub-pixel
  alignment, `top` leaves a residual +2px
  cursor-above-text delta.

### Gate

* Empirical: cursor + placeholder share Y axis
  (deltas ≤ 0.1px). Visual screenshot
  confirms `{cursor}{space}{default-text}` per
  spec.
* `svelte-check` → 0/0.
* `vitest` → **1321 / 1321** (+7 new pins).
* Rust gate clean (no Rust delta).

### Suggested commit subject

```
Rich prompt: cursor/placeholder Y-alignment (fullstack-a-89b)
```

### Files (per-path)

* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/richPromptCursorAlignment.test.ts` (new)
* `web/src/components/richPromptPlaceholderExtension.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-89b.md`

Autonomous-commit mode. No clearance held.
Test drive torn down (`chan remove /tmp/chan-89b`).
Walk handed back to @@WebtestA for empirical
HOLD confirmation against the in-tree binary.
