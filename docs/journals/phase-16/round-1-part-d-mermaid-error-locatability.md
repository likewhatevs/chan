# Design: mermaid error line/col locatability (@@LaneD, task-2)

DESIGN-FIRST per @@Lead. Posting for @@Lead/@@Host sign-off before
implementing. Grounded in the actual code, not intuition.

## Problem

When a mermaid block fails to render, the error face shows mermaid's
raw message, e.g.

    mermaid: Parsing failed: Lexer error on line 5, column 5:
    unexpected character: ->z<- at offset: 90, skipped 4 characters.

The "line 5, column 5" is relative to the mermaid SOURCE (the text
between the fences, 1-indexed). But the code block has no line-number
gutter, so the user can't find line 5 to fix it. @@Host's constraint
stands: do NOT add a global line-number gutter or restyle code blocks.

## What the code gives us (grounded)

- `mermaid_render.ts::renderMermaid` returns `{ok:false, error}` where
  `error` is mermaid's raw `err.message` - it CONTAINS "line N,
  column M" for Lexer/Parse errors. So it's parseable (your 07:50).
- It renders `mermaid.render(id, source.trim())` - the `.trim()` can
  drop leading blank lines, which would shift mermaid's line N vs the
  raw source. The mapping must add back the trimmed leading-line count.
- `widgets/mermaid.ts::mermaidSource` already knows the block's
  `openLine` (the ```mermaid fence line); source line 1 = openLine + 1.
- The error is only known after a render, which only happens when the
  cursor is OUTSIDE the block (the error FACE). When the cursor is
  INSIDE (editable source) there is no render - so we must CACHE the
  parsed error from the last render to use it while editing, exactly
  like the reverse-flip face cache already does.

## Proposal (your lean: a + c)

1. **Parse line/col** in `mermaid_render.ts`: extend `MermaidResult` to
   `{ok, svg?, error?, errorLine?, errorCol?}`; regex
   `/line (\d+)(?:,?\s*column (\d+))?/i` over the message. No match ->
   leave them undefined (message-only, no highlight). Map to a DOC line
   accounting for the `.trim()` leading-blank offset.

2. **Cache** the parsed error per source+theme (parallel to the face
   cache), populated on render in the widget's `toDOM` `.then`.

3. **(a) Highlight the failing source line**: a small decoration that,
   when the cursor is INSIDE a mermaid block AND there's a cached error
   for its CURRENT source, adds a `Decoration.line` error class on doc
   line `openLine + N` (+ optional column-M mark). Mermaid-blocks only -
   no global code-block chrome.

4. **(c) Actionable error face**: reformat the rendered error face to
   LEAD with the line - "Mermaid error - line N" + echo that source
   line's text + the short reason - so the user sees which line is
   wrong before going in (today it's a raw one-liner).

## Decisions for @@Lead / @@Host (1-3 options each)

**D1 - failing-line highlight style** (mermaid blocks only):
  (i) left-border accent on the line (amber/red bar, gutter-less
      linter feel) - RECOMMEND
  (ii) full-line subtle red background tint
  (iii) (i) PLUS a wavy underline on the column-M token

**D2 - live re-validation as you edit**:
  (A) cached-error-on-entry only: highlight comes from the last render;
      it clears once the source changes (you typed) until you leave +
      re-render. Simplest, no editor-time mermaid calls. RECOMMEND v1.
  (B) live debounced `mermaid.parse()` on each edit so the highlight
      tracks while typing. More moving parts; propose as a follow-up.

**D3 - actionable error face (c)**:
  (yes) lead with "line N" + echo the offending source line text -
        RECOMMEND
  (no)  keep the raw mermaid message as-is, ship only the highlight

## Scope / files (all mine)

`mermaid_render.ts` (parse line/col), `widgets/mermaid.ts` (cache +
error-line decoration + actionable face), `Wysiwyg.svelte` (the
mermaid-error-line CSS only - no code-block restyle). Browser-smokeable
on the `zzzz` fixture in mermaid.md (line 5 error).

Holding implementation for the D1/D2/D3 calls.
