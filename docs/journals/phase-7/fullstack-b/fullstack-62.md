# fullstack-62: rename Pane Mode → Hybrid NAV (user-facing copy sweep)

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex pulled the "Pane Mode → Hybrid NAV"
rename forward from the phase-8 backlog (item 4)
into the phase-7 wrap. The terminology shift
aligns the keystroke mode name with the Hybrid
concept already in use for pane content. Locked
wording: **`Hybrid NAV`** (NAV uppercase as
@@Alex prefers).

User-facing copy only. Internal symbols
(`paneMode*`, `paneModeOpenTerminal`,
`paneModeKeymap`, etc.) stay — semantic carry-
over, no code churn.

## Wording

* Locked menu entry: **`Enter Hybrid NAV`**
  (with the `Cmd+K` chord as today). No "Press
  Cmd+K" prefix; the chord column communicates
  the action.
* Locked status pill / chip: replace `pane mode`
  with `Hybrid NAV` (lowercase `Hybrid` if the
  pill is sentence-case in current usage —
  match the surrounding case style;
  uppercase `NAV` consistent everywhere).
* Help overlay title (`PaneModeHelp.svelte`):
  current header reads `Pane Mode` → becomes
  `Hybrid NAV`.

## Audit surfaces

Grep `web/src` for user-facing copy. Known sites
(from the existing codebase + walker observations):

* `web/src/components/Pane.svelte` — hamburger
  entry `Enter Pane Mode` (added by `fullstack-46`).
* `web/src/components/PaneModeHelp.svelte` —
  help overlay title + any body text mentioning
  "Pane Mode".
* `web/src/App.svelte` — Pane Mode pill /
  chip rendering (`pane mode · Enter commit ·
  Esc discard`-style text).
* `web/src/state/shortcuts.ts` — shortcut
  `label` strings if any reference the mode
  name.
* `fullstack-61` flash overlay (if it ships
  before this one): copy uses post-rename
  wording.

Internal symbol names DO NOT change:
* `paneMode`, `paneModeDraft`, `paneMode.draft`,
  `paneModeOpenTerminal`, `paneModeOpenBrowser`,
  `paneModeOpenGraph`, `paneModeKeymap*`,
  `app.pane-mode` CSS class, etc.

CSS class names are at the implementer's
discretion — renaming `.pane-mode` → `.hybrid-nav`
is more churn than payoff. Suggest keeping
internal classnames as-is.

## Acceptance criteria

* Hamburger menu pane entry reads
  `Enter Hybrid NAV` (chord `Cmd+K`).
* Help overlay header reads `Hybrid NAV`
  (or `Hybrid NAV — pane navigation` if more
  context helps; your call within the wording
  lock).
* Pane Mode pill / chip uses the new wording.
* No user-facing surface still renders
  literal `Pane Mode`. Grep `web/src` for
  `/Pane Mode/i` and audit each match: drop or
  rename in user copy; leave internal symbols
  + comments + this task file alone.
* Comments and identifier names can keep "pane
  mode" semantics; the rename is presentation-
  layer only.
* The Pane Mode help cheatsheet (Cmd+K then H)
  still works; its title flips to `Hybrid NAV`
  + body text references update.

### Tests

* Negative grep test (Vitest or just a
  shell check baked into the gate): no
  user-facing rendered string contains
  `Pane Mode` (case-insensitive) after the
  sweep.
* Component snapshots that previously
  asserted `Pane Mode` strings flip to
  `Hybrid NAV`.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Coordinate with `-61` (flash overlay) — if
  `-61` ships first, its copy uses old wording
  and this task sweeps it. If `-62` ships
  first, `-61` writes the post-rename wording
  directly. Either ordering works.
* Coordinate with `-60` (hamburger trim) — the
  hamburger entry label change rides with this
  rename; `-60` only drops the trailing rows
  past the colour swatches and leaves the
  `Enter Pane Mode` label alone for `-62` to
  flip.
* The phase-8 backlog (`next-phase-backlog.md`
  item 4) listed this as "container refactor
  + rename + minimal empty pane". The
  container refactor + minimal empty pane STAY
  in phase 8. Only the rename moves forward.
  I'll update the backlog accordingly.
* Queue position: behind `-54`, `-58`, `-59`,
  `-60` on Lane B.
* Standing topic-level commit clearance.

## 2026-05-19 20:35 BST — implementation

**Visible-text rename only.** Internal symbols
(`paneMode`, `paneModeKeymap`, `paneModeOpen*`,
the `.pane-mode-*` CSS classes, `paneMode.active`
state, etc.) stay untouched per the task spec.
Only the user-visible labels and ARIA strings
flip.

**Edits:**

* `web/src/components/Pane.svelte`:
  * Hamburger row label `Enter Pane Mode` →
    `Enter Hybrid NAV`.
  * Pane Mode preview's `aria-label` from
    `"pane mode preview"` → `"Hybrid NAV preview"`.

* `web/src/components/PaneModeHelp.svelte`:
  * Dialog `aria-label="Pane Mode help"` →
    `"Hybrid NAV help"`.
  * Title text `<div class="title">Pane Mode</div>`
    → `Hybrid NAV`. (Kept the title plain
    "Hybrid NAV" — no "— pane navigation"
    subtitle; the cheatsheet body explains the
    bindings already.)

* `web/src/state/shortcuts.ts`:
  * `app.pane.mode` entry's `label` flipped to
    `"Enter Hybrid NAV"`. This label feeds the
    web/native shortcut tables AND the hamburger
    chord column via `chordLabel("app.pane.mode")`
    — the menu line above + the cheatsheet's
    chord references all pick up the rename for
    free.

**Tests updated:**

* `web/src/state/shortcuts.test.ts` — the
  `advertises Pane Mode (Cmd+K) as the
  canonical spawn surface` test's regex flipped
  to `/^Enter Hybrid NAV\s+Cmd\+K$/m`. Test
  name updated to "Hybrid NAV" to match.

* `web/src/components/Pane.test.ts` — the
  hamburger focus-color test's `menuLabels()`
  assertion flipped from `["Enter Pane Mode",
  ...]` to `["Enter Hybrid NAV", ...]`. The
  fullstack-60 trim sentinel and the empty-pane
  context-menu tests don't reference the old
  label.

**New sentinel** in
`web/src/components/hybridNavRename.test.ts`:

* `Pane.svelte hamburger entry reads Enter
  Hybrid NAV` — positive assertion on the new
  copy.
* `Pane.svelte Hybrid NAV preview aria-label
  uses the new copy`.
* `Pane.svelte renders no user-facing 'Pane
  Mode' string` — negative grep with a strip-
  comments-and-style helper so internal
  variable accesses (`paneMode.active`),
  comments explaining historical naming, and
  the `pane-mode-*` CSS class names don't trip
  the match. Catches future regressions to the
  old label in visible text or attribute
  values.
* `PaneModeHelp.svelte title + aria-label use
  Hybrid NAV` — positive assertion.
* `PaneModeHelp.svelte renders no user-facing
  'Pane Mode' string` — negative grep.

**Audit result:** grep `web/src` for
`/Pane Mode/i` after the change. Remaining
matches are all comments, internal test names,
CSS class names, or variable identifier
references — none render to the user. The
sentinel pins this with the comment-stripping
helper.

**Gate.** `npm run check` 0/0; `npm run test`
37 files / 384 tests (was 36 / 379; +5 from
the new sentinel + the flipped existing
asserts); `npm run build` clean;
`scripts/pre-push` green.

**Out of scope (per task):**
* No internal symbol renames.
* No CSS class renames (`pane-mode-help`,
  `pane-mode-preview`, etc.).
* `fullstack-61` flash overlay copy: the flash
  shows just "H for help" — no `Pane Mode`
  string. No coupling.
* Phase-8 backlog items (container refactor +
  minimal empty pane) stay deferred.

**Commit readiness:**

Files staged:
* `web/src/components/Pane.svelte`
* `web/src/components/PaneModeHelp.svelte`
* `web/src/state/shortcuts.ts`
* `web/src/state/shortcuts.test.ts`
* `web/src/components/Pane.test.ts`
* `web/src/components/hybridNavRename.test.ts`
  (new)
* `docs/journals/phase-7/fullstack-b/fullstack-62.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Rename "Pane Mode" → "Hybrid NAV" in user copy (fullstack-62)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 17:30 BST cut.
