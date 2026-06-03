# task-LaneA-LaneC-1: B2 - unordered list bullet glyphs

From: @@LaneA  To: @@LaneC  Wave: 1 (isolated, start now)

## Objective

Refine the editor's unordered-list bullet glyphs to match Google Docs across
nesting levels. @@Alex: "The unordered list still needs more refinement, and
we are going to copy the bullet glyphs from Google Docs."

## Spec source

- docs/journals/phase-17/round-1/draft.md (the B2 "unordered list" bullet)
  and its reference screenshot docs/journals/phase-17/round-1/image.png -
  VIEW the image; it is @@Alex's Google-Docs reference for the glyphs.

## Anchors (re-verify against HEAD; lines drift)

- web/src/editor/decorations/blocks.ts ~437-519: BULLET_DASH /
  BULLET_DOT_TOP / BULLET_DOT_NESTED decorations; bulletMarkerDecoration()
  ~480 maps source char + nesting level to a class; decorateBulletList() ~487
  walks the tree.
- web/src/editor/Wysiwyg.svelte ~1021-1060: the .cm-md-ul-* CSS that sets the
  rendered glyph. Today: hyphen "-" -> en-dash \2013; top-level "*" -> filled
  circle \25CF at 60%; nested "*" -> hollow circle \25EF at 72%; "+" -> literal.

## Mechanism (keep)

Bullets are CSS glyph SUBSTITUTION; the source markers (- / * / +) stay in the
buffer for round-trip fidelity. Change only the rendered glyph + sizing, not
the parse or the stored text.

## Requirements

- Per-nesting-level glyphs matching Google Docs. Google Docs cycles roughly:
  level 1 = filled disc, level 2 = open circle, level 3 = filled square.
  CONFIRM the exact glyphs + sizes against image.png rather than trusting this
  from memory.
- Apply consistently for all three source markers (-, *, +) at each level;
  do not let the chosen marker change the rendered glyph (Google Docs keys the
  glyph off depth, not the typed char).
- Even optical size + baseline alignment across levels (the current 60%/72%
  sizing reads inconsistent).

## Out of scope

List parsing/semantics, ordered lists, task-list checkboxes. Glyph rendering
only.

## Gate (own-gate before reporting done)

- make web-check (vitest) + svelte-check + npm run build, all green.
- Browser-smoke a multi-level unordered list (>=3 levels, mixed -/*/+ markers).
  rust-embed bakes the bundle at build time: npm run build BEFORE cargo build,
  and smoke the SERVED bundle (a stale web/dist gives a false negative).

## Report

When done, cut tasks/task-LaneC-LaneA-1.md (summary + own-gate-green +
pathspec sha for the touched files) and poke @@LaneA. This is Wave-1 isolated:
no shared files, proceed independently.
