# fullstack-9: markdown table rendering crash (B20)

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Fix the editor crash when rendering a markdown pipe table.
@@WebtestA caught this in `webtest-a-1` as the Lane A
headliner: opening a doc with a pipe table triggers
`RangeError: Block decorations may not be specified via
plugins` and leaves an empty area + chevrons + may break
content rendering below the table.

This is blocking @@Alex's normal authoring workflow because
multiple in-tree docs contain pipe tables (their own
`setup-1.md` Q3 reproduces it deterministically).

## Relevant links

* [../request.md](../request.md) Bugfixes — B20.
* [../alex/setup-1.md](../alex/setup-1.md) Q3 — live repro.
* [../webtest-a/webtest-a-1.md](../webtest-a/webtest-a-1.md)
  — original headliner with the exact stack trace.

## Acceptance criteria

* Pipe tables render as actual table elements (or at least
  as preformatted blocks that don't crash). No `RangeError`
  in the console; no empty area with stray chevrons; content
  below the table renders normally.
* Source view still shows the raw pipe-table markdown
  untouched.
* Switching source ↔ rendered with the cursor on a table
  row preserves position (covered by `fullstack-10`'s
  source/rendered mapping work; coordinate if both are in
  flight).
* Unit test on the table block: parse + render without
  throwing.

## Out of scope

* Editable tables (full WYSIWYG table grid). Out of scope
  for this task; render-only is fine.
* Other markdown extensions.

## How to start

1. Open one of the live-repro docs (e.g.
   `docs/journals/phase-7/alex/setup-1.md` or just
   `request.md`).
2. The error string `Block decorations may not be specified
   via plugins` is a CodeMirror 6 idiom. Search the editor
   extensions for any `Decoration.block` set via a
   `ViewPlugin` instead of via the `EditorView.decorations`
   field. That's the wrong attachment point for block
   decorations.
3. Likely fix: move the table block decoration from a
   `ViewPlugin` to a `StateField` (the right attachment
   point) or use `EditorView.decorations.of(...)` correctly.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-18 18:11 BST — implementation

Implemented the pipe-table crash fix.

Changed files:

* `web/src/editor/widgets/table.ts`
* `web/src/editor/widgets/table.test.ts`

What changed:

* Moved the table block replacement decorations out of a `ViewPlugin` and into
  a `StateField`.
* Exposed the field through `EditorView.decorations.from(field)`, which is the
  CM6-supported path for block decorations.
* Kept `EditorView.atomicRanges` wired to the same decoration set so caret
  movement still treats rendered tables as an atom.
* Added a jsdom CodeMirror test that renders a GFM pipe table without throwing,
  verifies a table DOM exists, and verifies the source markdown is unchanged.

Verification:

* `npm run test -- table` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

Notes:

* No manual browser console pass performed in this lane.
