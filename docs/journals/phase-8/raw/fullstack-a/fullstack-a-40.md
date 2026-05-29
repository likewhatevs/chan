# fullstack-a-40: Wysiwyg outline-style dotted numbering (CSS counters)

Owner: @@FullStackA
Date: 2026-05-21

## Goal

Render nested numbered lists in chan's wysiwyg editor using
outline-style dotted numbering (`1. → 1.1. → 1.1.1.` etc.)
instead of independent per-depth counters (`1. → 1. → 1.`).
Source markdown stays standard (interoperable with GitHub /
Obsidian / other markdown tools).

## Background

Bug entry:
[`../phase-8-bugs.md`](../phase-8-bugs.md) — "Markdown
wysiwyg enumerated-list nested numbering: want outline-style
dotted notation" (filed 2026-05-21).

@@Alex confirmed implementation shape in A.7:
[`../architect/round-2-open-questions.md`](../architect/round-2-open-questions.md):

> agree A

Option A = **pure visual / CSS counters**. Source stays
standard markdown (`1. text\n   1. sub-item`); chan's
renderer applies CSS `counter-reset` + `counter-increment`
+ `::marker content: counters(...)` to produce the dotted
display. Other markdown tools (GitHub, Obsidian) continue
rendering with independent per-depth counters.

Why this shape vs option B (source change): source
portability matters — chan's docs in `docs/journals/` are
read by agents via filesystem; standard markdown stays
interoperable. The dotted display is a chan-specific
render-time convention.

## Authorization

**Authorization: yes**, covers:

* `web/src/styles/*.css` (or wherever the wysiwyg renderer's
  CSS lives) — `counter-reset` + `counter-increment` +
  `::marker` declarations for nested `<ol>` levels.
* `web/src/editor/Wysiwyg.svelte` (or whichever component
  hosts the markdown renderer) — possible HTML structure
  tweaks to support the CSS counters.

@@FullStackA may proceed without further @@Alex confirmation.

## Acceptance criteria

* Nested `<ol>` lists in wysiwyg render with outline-style
  dotted numbering:
  ```
  1. item
  1.1. sub-item
  1.2. another sub
  2. another item
  2.1. sub
  ```
* Multi-level nesting follows the same pattern: depth-3 →
  `1.1.1.`, `1.1.2.`, etc.
* Source-mode view continues to show STANDARD markdown
  (`1. / \t1. / 2.`); the dotted display is rendering-only.
* Pasting from another markdown source (per `-a-34` paste
  handler) continues to work; pasted lists render with the
  chan-specific dotted display.
* Theme-aware: marker colour respects the current theme
  tokens (light / dark / per-Hybrid override per `-b-5`
  while it still exists).
* Pre-push gate: clean.
* (Optional) Vitest pin: render a synthetic nested
  `<ol>` + assert the `::marker` computed style produces
  the expected dotted string. Pure CSS — may not be
  testable without jsdom CSS-counter support. Defer if
  jsdom doesn't render counters.

## How to start

1. Grep the wysiwyg CSS for any existing `::marker`
   declarations on `<ol>`. The renderer may already use
   `list-style-type` — that conflicts with
   `counter-reset` approach; need to override to `none`.
2. Add `counter-reset: section` + `counter-increment:
   section` declarations on nested `<ol>` rules. Compose
   `::marker { content: counters(section, ".") "."; }`
   for the dotted display.
3. Verify the renderer outputs nested `<ol>` (not nested
   `<ul>` with custom markers). If it doesn't, may need
   to wire the markdown-it / commonmark / similar parser
   to preserve nested `<ol>` semantics.
4. Test against a doc with nested numbered lists.
5. Confirm source-mode view is unaffected (the CSS only
   applies to the wysiwyg render).
6. Append commit-readiness.

## Coordination

* Independent of other v0.11.2 tasks (CSS + renderer-only).
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Parallelisable.
* Composes with the deferred Round-2 wave-2 work on
  source-mode list intervention (`-a-41`) — both touch
  list rendering but at different layers (this is CSS-only
  for wysiwyg; -a-41 is keymap stripping in source mode).

## Open questions

(populated as you investigate)

## 2026-05-21 — ready for review

### Spec adjustment: CM6 widget, not pure CSS counters

The spec recommended pure CSS counters + `::marker` declarations,
assuming chan's wysiwyg renders ordered lists as HTML `<ol><li>`.
It doesn't — chan's wysiwyg is CodeMirror 6, and ordered-list
markers are SOURCE TEXT inside `.cm-md-list-line` siblings (each
line is a separate DOM row; no nested `<ol>` semantics). Pure
CSS counters can't replace text content; `::marker` only works
on real list items.

The right CM6 idiom is a `Decoration.replace` Widget over the
`ListMark` range. Source stays standard markdown; the rendered
line shows the dotted chain.

### What landed

* `web/src/editor/decorations/blocks.ts`:
  * `OrderedMarkerWidget` — `WidgetType` rendering a span with
    class `cm-md-ol-marker` containing the dotted label.
  * `orderedMarkerLabel(prefix, index)` — exported pure helper
    that joins ancestor indices with the new index and adds a
    trailing dot. Test-pinnable independent of the widget
    wiring.
  * `ancestorOrderedList(node)` — predicate used to skip the
    decoration pass on nested OLs (the outer pass owns the
    chain so the nested handler doesn't double-decorate).
  * `decorateOrderedList(ctx, ol, prefix)` — walks every
    `ListItem` direct child, replaces its `ListMark` range
    with the widget at the computed dotted label, recurses
    into nested `OrderedList`s with the extended prefix.
  * `handleOrderedList` now invokes `decorateOrderedList` for
    top-level OLs; early-returns from the recursive case (the
    `ancestorOrderedList(ctx.node.node)` guard).
  * Header comment at the top of the file updated — the
    "OrderedList: no marker replacement" line was stale once
    the widget pass landed.
* `web/src/editor/Wysiwyg.svelte`:
  * Scoped CSS for `.cm-md-ol-marker` — inherits the line's
    text colour + font, `font-variant-numeric: tabular-nums`
    so the digits align across rows when several markers stack
    visually. Same theme-token shape as the existing
    `.cm-md-bullet` styling.
* `web/src/editor/decorations/blocks.test.ts`:
  * 3 new pins on `orderedMarkerLabel`: top-level segments,
    nested concatenation, deep nesting.

### Source portability

Source markdown is unchanged. A doc with `1. a\n   1. b\n   1.
c\n2. d`:

| Display (chan wysiwyg) | Source (preserved verbatim)           |
|------------------------|----------------------------------------|
| `1. a`                 | `1. a`                                 |
| `1.1. b`               | `   1. b`                              |
| `1.2. c`               | `   1. c`                              |
| `2. d`                 | `2. d`                                 |

External markdown renderers (GitHub, Obsidian, anything reading
the `.md` directly) see the standard form and render with their
own counter semantics. The dotted display is chan-specific.

### Source-mode unchanged

Source mode reads the raw doc — the widget only fires in the
wysiwyg rendering pipeline. Verified by reading the layer: the
decoration pass runs inside the wysiwyg's CM6 view plugin
stack; source-mode mounts a different editor without these
decorations.

### Suggested commit subject

```
Wysiwyg: outline-style dotted ordered-list markers (fullstack-a-40)
```

Single commit. The widget + walker + CSS + comment refresh +
tests are tightly coupled.

### Gate

* vitest **581 / 581** (+3 in `blocks.test.ts` for
  `orderedMarkerLabel`).
* svelte-check 0 errors / 0 warnings across 3982 files.
* npm build clean.

### Composition

* Composes with `-a-41` (source-mode list intervention) —
  this task lives in the wysiwyg pipeline, `-a-41` lives in
  the source-mode CM6 extension stack. No overlap.

Picking up `-a-41` (source-mode list intervention) next —
final task in the v0.11.2 wave on my lane.
