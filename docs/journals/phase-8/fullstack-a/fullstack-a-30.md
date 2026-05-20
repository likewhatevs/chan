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

## 2026-05-20 — implementation note + ready for review

Confirmed at start: the editor-side page-width slider DOES
exist today (`FileEditorTab.svelte:466-480` — page-width row
inside the tab-menu bubble, opened by right-click on the
editor body via `onEditorContext` synthesizing into the
shared menu). So this task is the "add the same slider to
the rich-prompt textbox's right-click menu" mirror, no new
editor-side wiring.

### Decoupling approach

The page-width plumbing currently flows through a single
CSS variable `--chan-page-max-width` set by
`Pane.svelte:226` (via `applyPageWidthToElement`) on the
pane's editor wrapper. The rich-prompt's
composer-editor (Wysiwyg / Source) reads that variable via
`max-width: var(--chan-page-max-width, none);`. Because
the prompt is a descendant of the wrapper, the cap
cascades — which is exactly the coupling @@Alex caught.

Per-prompt fix: override `--chan-page-max-width` INLINE on
the `.rich-prompt` element. Descendants see the override
instead of the inherited pane value. Two branches:

* `pageWidthRatio` absent or ≥ 1.0 → set `none` inline.
  The composer fills the prompt's painted width. This is
  the new default behaviour: a user who hasn't touched the
  per-prompt slider now sees the prompt uncapped, fully
  decoupled from the editor's pane-level slider. Visible
  but intentional change — chat-style composers want
  near-full-width by default.
* `pageWidthRatio` in [0.25, 1.0) → set
  `${Math.max(240, width * ratio)}px` inline, where `width`
  is the prompt's measured painted width
  (ResizeObserver-driven, same observer as `fullstack-a-29`).

Width measurement extends the `fullstack-a-29` ResizeObserver
to also write `prompt.measuredWidthPx` alongside
`measuredHeightPx`. One observer, two reactors.

### Slider in textbox right-click menu

The existing `onContextMenu` opened the `.ctx` menu with
mode + toolbar + new-file + watch + spawn buttons. Added a
new `.page-width-row` at the top of the menu (same shape
as `FileEditorTab.svelte`'s tab-menu slider — Page-width
label + range input + value readout). The input's
`oninput` calls `onRichPromptPageWidthSlider`, which
mutates `prompt.pageWidthRatio` directly (does NOT touch
the global `setPageWidth`). Reads `richPromptPageWidthPct`
for the slider's current value (100 % when unset). 100 %
unsets to absent so the persisted shape stays minimal.

### Persistence

`pageWidthRatio?: number` on `TerminalRichPromptState`.
SerTab field `rppw?: number` with conditional spread on
serialize (only emitted when 0 < ratio < 1; 1.0 / "no cap"
rounds to absent). Deserialize on both restore paths,
with the same range guard so a corrupted value falls
through to the default. Round-trip pinned + 100 % omission
pinned in `tabs.test.ts`.

### Why not extend the global `pageWidth`

Considered extending `pageWidth.svelte.ts` with a
per-instance helper that takes an element + custom ratio.
Decided against: the global module is keyed on
`window.innerWidth` + the `chan.pageWidth.ratio`
localStorage key, neither of which captures "this
specific prompt." The override at the
`.rich-prompt` level is cleaner — it's a one-liner
inline style backed by `prompt.measuredWidthPx`, and
doesn't risk polluting the global state surface.

### Files touched

* `web/src/state/tabs.svelte.ts`
  * `TerminalRichPromptState`: new `measuredWidthPx?` +
    `pageWidthRatio?`.
  * `SerTab`: new `rppw?` with conditional spread on
    serialize; deserialize via `richPromptFromSer` with
    range guard.
* `web/src/components/TerminalRichPrompt.svelte`
  * Extended ResizeObserver tracks width.
  * New `richPromptPageWidthPx()` / `richPromptPageWidthPct()`
    + slider event handler.
  * Inline `style:--chan-page-max-width` override on
    `.rich-prompt`.
  * New `.page-width-row` at top of `.ctx` menu.
  * CSS for `.page-width-row`/`-label`/`-slider`/`-value`
    mirrors the editor's tab-menu slider verbatim so both
    surfaces read alike.
* `web/src/state/tabs.test.ts` (+2 tests)
  * SerTab `rppw` round-trip with a < 1 ratio.
  * `rppw` omission when ratio is 1.0.

### Test pin notes

Skipped a vitest pin on the actual visual page-width
override — ResizeObserver doesn't fire in jsdom, so
`measuredWidthPx` would never populate in unit tests.
The behavioural contract (composer caps to prompt-relative
width, decoupled from sibling tiles) requires real
browser layout. @@WebtestA's lane-A walkthrough is the
authoritative verification. SerTab persistence + the
range-guard contract IS unit-pinned.

### Gate

* `vitest`: 514/514 (+2 from -28's 512 baseline).
* `svelte-check`: 0 errors / 0 warnings / 3974 files.
* `npm run build`: clean.
* Rust gate: no Rust changes; not run.

### Suggested commit subject

`Rich prompt: per-prompt page-width slider + cross-tile decoupling (fullstack-a-30)`

### Cross-lane coordination

No conflicts with `fullstack-b-13` — different files
entirely. -a-30 stays in `tabs.svelte.ts` (SerTab `rppw`,
TerminalRichPromptState) + `TerminalRichPrompt.svelte`
(observer + slider + inline override). -b-13 touches the
header toolbar + chan-server `terminal_sessions.rs`.
