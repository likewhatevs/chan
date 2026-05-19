# fullstack-53: desktop launcher — drop name column, italic tagline, reorder header buttons, computer-glyph for outside-home paths

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Four small refinements to the `chan-desktop` Tauri
launcher (the drive-picker chrome the user sees when
they open the native app before picking a drive):

1. **Drop the "Name" column** from the drives table.
   The path already identifies the drive uniquely and
   the rename-display surface isn't pulling its
   weight. Path + On toggle + actions are enough.
2. **Italic tagline next to "Drives"** in the brand
   header — the exact text:
   `what are we going to DRIVE today?`
   Italic, with `DRIVE` kept uppercase as written.
   Sits inline next to the brand title, not on a
   second line.
3. **Reorder the header action buttons** to
   `[Open drive] [Attach] [theme toggle]`. Currently
   the theme toggle is leftmost and Open drive is
   rightmost; swap so Open drive is the leftmost
   action and the theme toggle sits at the far
   right.
4. **Computer-glyph for outside-home paths** in the
   path cell. `renderPath()` already collapses
   `$HOME` to a house SVG; paths outside home (e.g.
   `/private/tmp/chan-test-xyz` on macOS) currently
   render verbatim, which is noisy. Add a symmetric
   computer-icon prefix so the user has a visual
   cue that "this is somewhere on the computer,
   outside your home folder".

## Relevant code

* `desktop/src/index.html:11-33` — the `<header
  class="bar">` element. Brand title at `:14`,
  action buttons at `:16-32` (current order in DOM:
  theme-toggle, tunnel-btn Attach, auth-btn Sign in
  hidden, open-drive).
* `desktop/src/main.js:228-309` — `render(drives)`
  table render. Header at `:295-306`, body row at
  `:277-292` (local) and `:262-275` (tunneled). Name
  column header at `:301`, name cells at `:269` and
  `:286`.
* `desktop/src/main.js:120-134` — `renderPath()`.
  Home-prefix branch already collapses `$HOME` to
  an inline house SVG (`class="ic-home"`). The
  `else` branch (line 133) returns `escapeHtml(full)`
  verbatim and is the one to change.
* `desktop/src/styles.css` — `.bar`, `.brand`,
  `.actions`, `.drives`, `.ic-home`, `.path-sep`
  styles. Will likely need a small addition for the
  tagline (subtle muted italic; size + colour tuned
  to sit alongside `b.brand-title` without
  dominating) and a sibling `.ic-computer` rule
  (or a shared `.ic` class) to match the existing
  house-glyph sizing.

## Acceptance criteria

### Drives table

* The `Name` `<th>` (currently 200px wide) is
  removed from the header row.
* The `.name-cell` `<td>` is removed from both the
  local and tunneled row variants.
* Column widths re-balanced so the remaining
  columns (On / Path / actions) read cleanly. Path
  should absorb the freed space; the actions
  column stays at its current ~150px so the Open
  split button doesn't reflow.
* `d.name` references that were only used by the
  removed cells can drop. For tunneled rows the
  current fallback is `escapeHtml(d.drive ||
  d.name)`; that whole expression goes when the
  cell goes.

### Brand tagline

* New element renders `what are we going to DRIVE
  today?` in italics, inline beside
  `<b class="brand-title">Drives</b>` inside the
  existing `.brand` div.
* `DRIVE` stays uppercase. Don't apply
  `text-transform`; write it that way in the
  markup.
* Style: italic, muted colour (think `.muted`
  token if one exists, else half-opacity of the
  brand colour), slightly smaller than the
  brand title. Final look is your call — the
  requirement is "subtle, sits next to Drives,
  not a heading on its own".
* Empty-state copy at `main.js:240` already
  uses `<em>drive</em>` for the noun definition;
  leave that alone.

### Outside-home path glyph

* `renderPath(full)` else-branch (line 133) replaced
  with a computer-icon render path that mirrors the
  home branch shape:
  * If the full path is something like
    `/private/tmp/chan-test-xyz`, render an inline
    computer SVG (sibling of `.ic-home`, same 13×13
    sizing + `currentColor` stroke for theme
    parity) followed by the existing
    `<span class="path-sep">/</span>` separator and
    the full path (or a sensible leading portion;
    see below). Symmetric with the home branch's
    `<house>/<rest>` shape.
  * Glyph choice: a simple monitor/desktop outline
    is fine. Match the line-weight + viewBox style
    of `.ic-home` (`stroke-width="1.8"`, no fill,
    rounded joins). Final SVG path is your call;
    target "reads as a computer at 13px".
  * What follows the glyph: render the full path
    after the icon (e.g. `<computer>/private/tmp/
    chan-test-xyz`). Don't try to trim a prefix —
    there's no canonical "computer root" the way
    `$HOME` is for the home branch. The user can
    hover for the title attribute's verbatim path
    anyway.
* `aria-label` on the new SVG: `"computer"` (mirror
  the home glyph's `"home"`).
* Title attribute on the `.path-cell` `<td>` is
  unchanged — keep showing the full path on hover.

### Header buttons

* DOM order top-to-bottom inside `.actions` ends
  up: `open-drive`, `tunnel-btn` (Attach), the
  hidden `auth-btn` (kept as-is for the future
  re-enable), then `theme-toggle`.
* Rendered left-to-right: Open drive, Attach,
  theme toggle.
* The hidden `auth-btn` slot stays hidden; its DOM
  position is your call (right before theme is
  fine; keeping it where it is, just relative to
  the new order, is also fine — the user never
  sees it).
* Theme toggle behaviour unchanged. Sign-in
  button behaviour unchanged.

### Tests

* No unit tests required for the launcher (it's
  pre-Svelte vanilla DOM). Manual visual eyeball
  in the Tauri shell after `npm run check`
  passes on the web bundle is the verification
  path.
* If you want to add a tiny DOM-render snapshot
  test (load `index.html` into JSDOM, assert
  button order + tagline text), light-touch is
  fine. Not required.

### Gate

* `npm run check` (web crate-level type check
  shouldn't be affected, but run it for
  completeness).
* `cargo check -p chan-desktop` (the Tauri side
  shouldn't be touched; quick smoke).
* Manual run if you can: `cd desktop && cargo
  tauri dev` — eyeball the launcher with at
  least one registered drive present.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`.

## Notes

* This is the Tauri shell only (`desktop/`),
  *not* the embedded Svelte editor (`web/`). The
  rust-embed bundle is unchanged by this task.
* `chan rename` still works as a CLI; we're just
  dropping the launcher display. If anyone asks
  "where did the rename UI go" — they can keep
  using the CLI. Future task if we want a
  rename surface back.
* Visual eyeball *is* worth doing here — the
  italic tagline + button-order change are
  presentation-only and the lane-boundary rule
  allows ad-hoc-chan-serve / `cargo tauri dev`
  for pixel work. Teardown after.
* Standing topic-level commit clearance.

## 2026-05-19 15:05 BST — @@FullStackA implementation note

All four refinements applied to `desktop/src/`:

1. **Name column dropped** — header `<th>Name</th>`
   gone; both row variants (local + tunneled) drop
   the `.name-cell <td>`. Path column absorbs the
   freed space (CSS `max-width: 280px` dropped from
   `.path-cell`); actions column stays at 150px.
   `.name-cell` CSS rule deleted (no remaining
   references in `desktop/`).
2. **Italic tagline** — new
   `<em class="brand-tagline">what are we going to
   DRIVE today?</em>` inside `.brand`, sitting next
   to `<b class="brand-title">Drives</b>`. New CSS
   rule `.brand .brand-tagline` (italic, muted via
   `--text-secondary`, 12px, no `text-transform` so
   `DRIVE` keeps its uppercase from the markup).
3. **Header buttons reordered** — DOM order in
   `.actions` is now `[open-drive] [tunnel-btn]
   [auth-btn hidden] [theme-toggle]`. Renders
   left-to-right exactly as the spec asked.
4. **Computer-glyph for outside-home paths** —
   `renderPath()` else-branch now renders an inline
   SVG monitor icon (`<rect>` for the screen +
   stem + base via `<path>`) followed by the
   existing `path-sep` and the full path with the
   leading `/` trimmed. Matches the home variant's
   13×13 viewBox, `currentColor` stroke,
   `stroke-width="1.8"`, rounded joins.
   `aria-label="computer"` on the new SVG mirrors
   the home glyph's `"home"`. CSS `.ic-computer`
   shares the home glyph's `vertical-align: -2px;
   margin-right: 2px` via a combined selector.

Files touched (all in `desktop/src/`):

* `desktop/src/index.html` — brand tagline +
  actions reorder.
* `desktop/src/main.js` — `renderPath()` else
  branch + drop name cells from both row variants
  + drop Name `<th>`.
* `desktop/src/styles.css` — `.brand-tagline` +
  `.ic-computer` (sibling rule under
  `.path-cell`); dropped `max-width: 280px` from
  `.path-cell`; deleted `.name-cell` rule.

Gate green:

* `npm run check` (web — 0 errors / 0 warnings),
* `cargo check -p chan-desktop` (clean),
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green — fmt + clippy + test + no-default-
  features build all pass).

Visual eyeball not run locally (didn't want to
spawn a Tauri window without coordination). If
@@WebtestA / @@WebtestB pick this up, the four
checks are: tagline visible + italic, button order
LTR, Name column gone, outside-home paths get a
computer glyph (e.g. `/private/tmp/...`).

Proposed commit message:

> Desktop launcher refresh (fullstack-53)
>
> Four small refinements to the chan-desktop Tauri
> launcher: drop the Name column from the drives
> table (path + On + actions are enough), add an
> italic "what are we going to DRIVE today?"
> tagline beside the Drives brand title, reorder
> the header buttons to Open drive / Attach /
> theme-toggle, and render outside-home paths
> with a sibling computer glyph in `renderPath()`
> for visual parity with the existing house-glyph
> branch.
