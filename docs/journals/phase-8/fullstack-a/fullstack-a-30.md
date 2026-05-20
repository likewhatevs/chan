# fullstack-a-30: Per-prompt page-width + slider in textbox right-click menu

Owner: @@FullStackA
Date: 2026-05-20

## Goal

The rich prompt's composer inherits / shares the markdown
editor's page-width (CodeMirror max-content-width) setting.
In a tiled layout, narrowing the editor's page width in one
tile cascades onto the rich prompt in a sibling tile,
producing the awkward rendering @@Alex caught 2026-05-20
(screenshot in bug entry).

Two changes:

1. **Per-prompt page width**: each terminal's rich prompt
   has its own page-width value, independent from the
   editor's and independent from sibling tiles. Persists
   per-prompt-session as a new SerTab field (suggest
   `rppw?: number` for "rich prompt page width"; conditional
   spread on serialize so the empty case doesn't change the
   round-tripped SerTab shape).
2. **Slider in the rich-prompt textbox right-click context
   menu**: symmetric to the editor's existing surface.
   Reaches the user from whichever surface they're in.

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md) "Rich
prompt page-width is shared/inherited from the editor;
breaks under tiling".

Assumption to verify before designing: @@Alex's framing
("add the slider … **as well**") implies the slider exists
on the editor's right-click menu today. Grep the editor
components (likely `FileEditorTab.svelte` /
`HybridEditor.svelte` or wherever the markdown editor's
right-click menu is wired) to confirm. If the editor-side
slider doesn't exist, wiring it as part of this task is in
scope — the user-visible end state is "both surfaces expose
the same slider."

Relevant code:
* `web/src/components/TerminalRichPrompt.svelte` — the
  composer; per-instance CSS scoping needs to isolate the
  page-width style from the editor's.
* `web/src/state/tabs.svelte.ts` — SerTab field add +
  conditional spread.
* Editor right-click context menu (file TBD at grep time).
* Wysiwyg.svelte / Source.svelte CodeMirror page-width
  extension (line-length cap).

## Acceptance criteria

* Setting the editor's page width in one tile does not
  affect any sibling tile's rich prompt's page width.
* Setting the rich prompt's page width via the new slider
  affects ONLY that prompt; persists across reload
  (SerTab); does not affect the editor in the same pane or
  in any sibling pane.
* Right-clicking the rich prompt's textbox surfaces the
  slider with the same UX shape the editor's slider uses
  (or a faithful equivalent).
* If the editor-side slider didn't exist before this task,
  it's added in the same commit alongside the rich-prompt
  one — symmetric affordances.
* `vitest` green; pin the SerTab round-trip + the
  per-prompt-only scope (a regression test would set
  prompt A's width, set prompt B's width, assert each
  retains its own value).

## How to start

1. Reproduce: tile two panes, narrow the editor's page
   width in one, observe the rich prompt's composer in the
   other being affected.
2. Grep for the page-width setting in
   `web/src/state/tabs.svelte.ts` + the editor components.
3. Identify whether the constraint is shared via a global
   store / Svelte context / CSS variable on `:root` — that
   informs the scoping approach.
4. Pick a CSS-variable-per-instance approach if possible
   (cleanest: set `--rich-prompt-page-width: <value>` on
   the prompt's root element; the CodeMirror extension
   reads that variable). Verify the existing
   editor-side mechanism's shape first; mirror it.

## Coordination

* Sits in the rich-prompt surface alongside
  [`fullstack-a-28`](fullstack-a-28.md) (BubbleOverlay) +
  [`fullstack-a-29`](fullstack-a-29.md) (collapse dead
  space). Sequence: -28 → -29 → -30 keeps the audit trail
  clean; or pick whatever order fits.
* Composes with the future rich-prompt session-evolution
  work (history backlog + cwd preflight + team conductor) —
  the conductor bands need their own page-width handling
  that doesn't cascade across tiles either; this task's
  scoping pattern should generalise.
* @@WebtestA verifies on lane-A once landed.
