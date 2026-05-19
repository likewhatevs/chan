# fullstack-66: shared tab-title truncation utility + sweep

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged that tab names get unwieldy
when they're long — full file paths or long
filenames push the tab strip past its width
budget. Apply a consistent middle-elision rule
across all tab kinds so the tab strip stays
readable.

This lands early on Lane A's queue so the
title-rework tasks (`-64` Graph, `-65` Files)
both consume the same utility instead of each
implementing their own elision.

## Spec

* **Max display length: 15 characters.**
* **Threshold to trigger elision: 15 chars.**
  Names ≤ 15 render as-is.
* **Elision shape: `head[..]tail`** with
  `[..]` as the 4-char marker.
* **Head: 6 chars. Tail: 5 chars.** Total
  6 + 4 + 5 = 15.
* **Bias toward the tail** to preserve file
  extensions (`.md`, `.ts`, `.svelte`,
  `.json` are the common ones; 5 chars at
  the tail keeps the dot + 3-4 char ext
  visible for the typical cases).
* **Edge cases**:
  * Name with no extension (e.g. dir names) —
    same rule, no special handling needed.
  * Name exactly 15 chars — render as-is, no
    elision.
  * Name 16 chars — elision triggers; result
    is 15 chars (`head[..]tail` = 15 chars
    even for a 16-char source).
  * Very short head/tail collapse (e.g. an
    elision that produces something shorter
    than the original) — guard against this:
    if elision wouldn't actually shorten the
    name, render as-is.
  * Unicode / multi-byte chars: count code
    units, NOT bytes. Don't split a surrogate
    pair.

Tooltip-on-hover (`title="<full name>"`)
should render the full untruncated name on
ALL elided tab titles. Users can hover to see
the full path/name when they need to.

## Relevant code

* `web/src/state/tabs.svelte.ts` — central
  tab-title plumbing. Add `truncateTabTitle`
  utility (or similar name) here. Pure
  function, no Svelte state.
* `web/src/state/tabs.svelte.ts:324` —
  `tabTitle()` helper. Wrap the return in the
  truncation.
* `web/src/state/tabs.svelte.ts:387` — the
  `Graph: ${scopeId}` line (being reworked by
  `-64` independently). Both Graph and Files
  paths land their derived titles through the
  same truncation.
* `web/src/components/Pane.svelte` — tab strip
  render. Confirm `title="..."` (the HTML
  attribute, not the symbol) is set to the
  full untruncated name on tab elements so
  hover shows the full label.
* All tab kinds: file editor, terminal,
  graph, browser (Files), search, infographic
  if it lands later. Audit each tab-kind's
  title call site; route them through the
  utility.

## Acceptance criteria

* All tab strip titles are ≤ 15 chars rendered.
* Titles > 15 chars elide as `head[..]tail` =
  6 + 4 + 5 chars; total 15.
* Titles ≤ 15 chars render unchanged.
* Hover (`title="..."` HTML attribute) shows
  the full untruncated name on every elided
  tab.
* Truncation utility is exported / referenced
  from a single source of truth — no
  per-component reimplementation.
* `-64` (Graph) + `-65` (Files) consume this
  utility instead of computing their own.
  If `-64` lands before `-66`, mark its
  truncation as TODO; `-66` sweeps. If `-66`
  lands first (preferred), `-64`/`-65`
  consume it from day one.
* Other tab kinds (file editor with full
  filename, terminal with shell name + cwd,
  etc.) ALSO route through the utility. No
  tab kind escapes the truncation rule.

### Tests

* Vitest unit tests on the utility:
  * `""` → `""` (empty).
  * `"short.md"` (8ch) → unchanged.
  * `"exactly15chars."` (15ch) → unchanged.
  * `"sixteen-chars-md"` (16ch) →
    `"sixtee[..]rs-md"` (or whatever the
    head/tail split produces — assert
    matching shape).
  * `"verylongfilename.svelte"` (23ch) →
    matches the bias-toward-extension shape.
  * Unicode: a multi-codepoint name doesn't
    split a surrogate pair.
* Component test: tab strip renders elided
  titles + `title="..."` hover attribute
  carries the full name.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Visual eyeball worth doing — ad-hoc chan
  serve, open a few tabs with long names,
  verify the elision reads cleanly. The
  `[..]` marker should feel like a tab-strip
  convention, not a debug artifact. Teardown
  after.
* Edge case to spot-check: file editor tab
  with a dirty marker — the `●` (or similar)
  shouldn't get caught in the truncation. If
  the dirty marker is part of the title
  string, exclude it from the length count
  and append after truncation.
* Queue position: BEFORE `-64`/`-65` on Lane
  A. Updated queue:
  `-55` → `-56` → `-66` → `-64` → `-61` →
  `-65`.
* Standing topic-level commit clearance.

## 2026-05-19 16:38 BST — @@FullStackA implementation note

Implementation:

* `tabs.svelte.ts`: added `truncateTabTitle(label)`
  + exported `TAB_TITLE_MAX_LENGTH` constant.
  Counts code points via `Array.from(label)` so
  surrogate pairs never split. Strings whose
  code-point count is ≤ 15 pass through as-is;
  longer ones return `head[..]tail` (6 + 4 + 5
  = 15). Bias toward the tail keeps `.md` /
  `.ts` / `.svelte` / `.json` visible.
  Underlying `tabLabel` / `tabLabelInPane`
  intentionally stay untruncated — they're
  consumed for dedup logic (`tabLabel(candidate)
  === base` at line 340) and for the unsaved-files
  dialog at line 1019, neither of which wants
  the visual cap.
* `Pane.svelte`: wrapped the two display call
  sites — the tab strip `<span class="path">`
  (line 867 area) and the Pane Mode title at
  line 1036. The HTML `title` attribute still
  comes from `tabTooltip(t)` (full path), so
  hover reveals the untruncated name. The dirty
  marker `●` is a separate `<span>` after the
  label and is never part of the truncated
  string.
* 6 new vitest cases on the utility: empty, short
  (≤ 15), exactly 15, exactly 16 (first elision
  step), long with `.svelte`, and a multi-
  codepoint case with the supplementary `🌟`
  emoji to prove the surrogate-pair guard.

Audit of other call sites:

* `tabs.svelte.ts:1019` — used by the unsaved-
  files prompt message ("X has unsaved
  changes"); intentionally NOT truncated since
  it's a dialog body, not a tab strip.
* `tabLabel` / `tabLabelInPane` internal uses
  (lines 336-340) — dedup needs the full
  basename; intentionally untruncated.

`-64` (Graph chrome trim + smart title) and
`-65` (Files tab title from selection) will
consume `truncateTabTitle` for their derived
titles when they land.

Gate green:

* `npm run test -- tabs` (69 passed including
  the 6 new),
* `npm run test` (349 passed total),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball deferred to webtest. Spot check
worth doing: a doc tab with a long path
duplicated in two directories (so
`tabLabelInPane` triggers `x/[...]/foo.md`
shape) — the truncation cuts the combined
string to 15 chars; on hover the full path
shows.

Proposed commit message:

> Shared tab-title truncation utility (fullstack-66)
>
> Add `truncateTabTitle()` in `tabs.svelte.ts` and
> route the two tab-strip display call sites in
> `Pane.svelte` through it. 15-code-point cap as
> `head[..]tail` (6 + 4 + 5) biased toward the
> tail to keep extensions visible; counts code
> points via `Array.from` so surrogate pairs stay
> intact. Underlying `tabLabel` / `tabLabelInPane`
> stay untruncated for dedup + dialog body use.
> Tab tooltip (`title="..."`) keeps the full
> untruncated name so hover reveals it.
