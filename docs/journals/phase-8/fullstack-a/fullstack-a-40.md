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
