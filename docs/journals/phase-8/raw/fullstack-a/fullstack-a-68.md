# fullstack-a-68 — Hybrid Nav enhancements (Nav rename + transactional mode for new terminal/draft/graph/FB)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Two pieces per [`../alex/addendun-a.md`](../alex/addendum-a.md)
"## Hybrid Nav enhancements":

1. **Confirm and apply Nav rename**: today's "NaV" /
   "NAV" naming gets cleaned to "Nav" consistently
   across SPA + chan-desktop menu labels.
2. **Transactional mode for new terminal / new draft /
   new graph / new file browser**: enter Hybrid Nav,
   pick a Hybrid pane, press chord keys (T / O / P / G / E)
   to STAGE additions, only materialize on Enter; Esc
   discards.

## Reference

[`../alex/addendun-a.md`](../alex/addendum-a.md)
verbatim:

> First of all, let's confirm NAame/moV -> Nav
> - [ ] Back to transactional mode for new terminal, new draft, new graph, new file browser.. this means we can:
>   - [ ] Enter Nav mode, pick a Hybrid, press T to add terminals, O for file browsers, P for smart prompt terminal, G for graph, E for editor on draft
>   - [ ] Only on Enter we materialise; on Esc we dont do it

## Chord mapping

* `T` — add terminal
* `O` — add file browser
* `P` — add smart prompt terminal (rich-prompt-enabled
  terminal)
* `G` — add graph
* `E` — add editor with new draft (depends on `-a-66`
  Cmd+N draft creation; if `-a-66` not yet shipped,
  fall back to a placeholder OR scope-poke)
* `Enter` — commit staged additions
* `Esc` — discard staged additions

## Acceptance

1. **Nav label consistent**: search SPA + chan-desktop
   for "NAV" / "NAv" / "Nav " label variants; settle
   on "Nav" everywhere (per @@Alex's flag).
2. **Transactional staging**: Enter Hybrid Nav; press
   T multiple times → staged terminals visible (e.g.
   dimmed ghost rows in the tab strip); Enter
   materializes; Esc discards. Test the 5 chord keys.
3. **Sequence**: T then O then G + Enter materializes
   3 tabs in order.
4. **Esc resets**: T T T then Esc → no terminals
   added; state restored.

### Tests

Vitest pins for chord handlers + staged-vs-materialized
state separation + Enter/Esc resolution.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA SPA primary.
* If chan-desktop menu labels need touching for the
  Nav rename, that's a trivial cross-lane to
  @@FullStackB — bundle if minimal OR scope-poke if
  substantial.
* Atomic-audit-commit discipline.

## Authorization

Yes for SPA Nav state / chord handlers + label
renames. If chan-desktop menu labels need editing,
inline that under @@FullStackB authorization or fire
a scope poke.

## Numbering

This is `-a-68`.

## 2026-05-22 — slice 1 (Hybrid NAV → Hybrid Nav rename) ready for review

Four-file change. SPA-only. Title-case
rename only — transactional staging deferred
to slice 2.

### What landed

`web/src/components/Pane.svelte`:
* Hamburger menu entry: `Enter Hybrid NAV`
  → `Enter Hybrid Nav`.
* Preview aria-label: `Hybrid NAV preview`
  → `Hybrid Nav preview`.

`web/src/components/PaneModeHelp.svelte`:
* aria-label: `Hybrid NAV help` → `Hybrid
  Nav help`.
* Title text: `Hybrid NAV (Cmd+.)` →
  `Hybrid Nav (Cmd+.)`.

`web/src/components/hybridNavRename.test.ts`:
* Header comment updated to cite both
  `fullstack-62` (original Pane Mode →
  Hybrid NAV rename) and `fullstack-a-68
  slice 1` (NAV → Nav demotion).
* Existing 4 pins updated to expect
  title-case "Nav".
* +2 new pins guarding against regression
  to the all-caps form (one for Pane.svelte,
  one for PaneModeHelp.svelte).

`web/src/components/Pane.test.ts`:
* `Enter Hybrid NAV` → `Enter Hybrid Nav` in
  the hamburger menu-labels expectation
  (slice-1 pin landed as part of `-a-67
  slice 2`'s 9-entry list).

### What's deferred to slice 2

Per addendum-a:

* Transactional mode for new terminal /
  draft / FB / graph / editor staging in
  Hybrid Nav.
* T / O / P / G / E chord handlers that
  stage instead of materialise.
* Enter to commit; Esc to discard.
* Ghost-row visuals in the tab strip for
  staged additions.

Slice 2 is substantial (state machine +
visual ghost rows + materialisation + chord
handlers). Cutting slice 1 ships the
visible-copy half + clears the way for the
heavier piece.

### Acceptance (slice 1 only)

1. **Nav label consistent**: all visible
   surfaces (hamburger entry, preview
   aria-label, help dialog title +
   aria-label) read "Hybrid Nav" ✓.
2. **No regression to all-caps "NAV"** ✓
   — new pins assert absence in the visible
   strip.
3. **Internal symbols / CSS classes
   preserved**: `paneMode.active`,
   `.pane-mode-help`, `.pane-mode-preview`,
   `.pane-mode-flash` all stay (internal
   surfaces, not user-facing).
4. **Transactional staging**: DEFERRED to
   slice 2.

### Gate

* vitest **1028 / 1028** (+2 net from
  `-a-67 slice 2`'s 1026: +2 new pins on
  the no-NAV regression guard; 4 existing
  pin updates to expect Nav).
* svelte-check 0 errors / 0 warnings across
  4038 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Title-case "Nav"** per @@Alex's
  addendum-a flag ("NAame/moV -> Nav").
  Matches the typographic conventions of
  the rest of the UI ("Hybrid" is
  title-case; "Nav" follows suit).
* **Internal symbols stay** — function
  names (`paneMode`, `paneModeKeymap`),
  CSS classes (`.pane-mode-*`), and
  comments don't ship to the user. The
  rename's blast radius is bounded to
  visible copy + ARIA labels.
* **Slice 1 alone**: visible copy can ship
  independent of the transactional
  staging. @@WebtestA can walk slice 1
  before slice 2 lands.

### Suggested commit subject

```
Hybrid NAV → Hybrid Nav rename (fullstack-a-68 slice 1)
```

Single commit. 2 component edits + 2 test
file updates (one rename, one menu-label
expectation).

### Files for `git add` (per-path discipline)

* `web/src/components/Pane.svelte`
* `web/src/components/PaneModeHelp.svelte`
* `web/src/components/hybridNavRename.test.ts`
* `web/src/components/Pane.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-68.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.

## 2026-05-23 — slice 2 (Hybrid Nav transactional T/O/P/G/N staging)

SPA-only. Substantial state-machine extension to
restore the addendum-a "back to transactional
mode" framing for the Hybrid Nav spawn chords.

### Chord mapping (final)

* `T` — stage Terminal
* `O` — stage File Browser
* `P` — stage Smart Prompt Terminal (terminal
  tab with rich-prompt overlay armed open)
* `G` — stage Graph (V stays aliased for
  muscle memory)
* `N` — stage New Draft editor (changed from
  `E` per @@Alex's in-flight ask; matches the
  top-level Cmd+N "new draft" chord
  mnemonic — both routes share the same
  `api.createDraft()` round-trip)
* `Enter` — materialize the staged draft +
  commit (draft layout → live)
* `Esc` — cancel; draft + queue discarded

### State machine changes

`paneMode` singleton extended with:

* `stagedDraftEditors: { paneId }[]` — queue
  of pending "create draft + open" intents.
  Each entry pins the target paneId at press
  time so a later focus change doesn't
  redirect the materialization. Resets to
  `[]` in `enterPaneMode` / `commitPaneMode`
  / `cancelPaneMode`.

New helpers in `tabs.svelte.ts`:

* `paneModeOpenRichPromptTerminal(ctx?)` —
  spawns a fresh terminal in the draft with
  `richPrompt: { open: true, mode: "wysiwyg",
  ... }` so the rich-prompt overlay surfaces
  on first mount.
* `paneModeStageDraftEditor()` — pushes
  `{ paneId: draft.activePaneId }` onto the
  queue. No backend side effects.
* `paneModeStagedTabIds()` — derives the
  set of "staged" tab ids by diffing the
  draft layout against the live layout. The
  set lifts on commit (draft becomes live →
  no diff) or cancel (whole draft discarded
  → no draft to diff).

App.svelte changes:

* T → `paneModeOpenTerminal` (direct draft
  write, no commit).
* O → `paneModeOpenBrowser` + browser-
  selection prime (no commit).
* P → `paneModeOpenRichPromptTerminal` (no
  commit). Replaces the pre-`-a-68 slice 2`
  toggle behavior; reachable outside Hybrid
  Nav via Cmd+P / terminal hamburger.
* G + V → `paneModeOpenGraph` (V is the
  legacy alias).
* N → `paneModeStageDraftEditor` (queue
  enqueue).
* Enter → `materializeStagedDraftEditors()`
  resolves the queue (async `api.createDraft()`
  + `openInPane` per entry, snapshot up
  front since `commitPaneMode` clears the
  queue) → `commitPaneMode()`.
* Esc → `cancelPaneMode()` (queue auto-
  cleared; round-trips never fire so no
  orphan drafts).

Pane.svelte changes:

* New `paneModeStagedSet = $derived(paneModeStagedTabIds())`.
* Each tab DOM gets `class:staged={paneModeStagedSet.has(t.id)}`.
* CSS: `.tab.staged` (opacity 0.65 + dashed
  border + transparent bg). `.tab.staged.active`
  keeps a slightly higher opacity.

PaneModeHelp.svelte changes:

* Group title "Spawn" → "Stage (Enter to
  commit, Esc to discard)".
* Rows relabel: "Spawn …" → "Stage …".
* New `N` cap "Stage New Draft" added; `V`
  cap drops (still aliased in keymap).

### Files touched

* `web/src/state/tabs.svelte.ts`
  * `paneMode` singleton extended with
    `stagedDraftEditors` field; resets in
    enter / commit / cancel.
  * 3 new exports: `paneModeOpenRichPromptTerminal`,
    `paneModeStageDraftEditor`,
    `paneModeStagedTabIds`.
* `web/src/App.svelte`
  * Imports: added
    `paneModeOpenRichPromptTerminal`,
    `paneModeStageDraftEditor`, `openInPane`.
    Dropped `paneModeStageSpawn` (the single-
    intent abstraction; transactional flow
    writes directly to draft).
  * T/O/G/V/P/N case rewrites per the chord
    table above. Enter runs
    `materializeStagedDraftEditors()` before
    `commitPaneMode()`. Esc unchanged
    (cancelPaneMode clears the queue).
  * New helper `materializeStagedDraftEditors`
    walks the queue snapshot + fires
    `api.createDraft()` + `openInPane` per
    entry in parallel.
* `web/src/components/Pane.svelte`
  * Imports + derived `paneModeStagedSet` +
    `class:staged` binding + `.tab.staged`
    CSS.
* `web/src/components/PaneModeHelp.svelte`
  * Group rename + N row addition + V row
    drop.

### Tests

* `web/src/state/paneModeStaging.test.ts`
  (new): 10 architectural pins for the state
  machine (paneMode field + reset on enter
  / commit / cancel), the 3 new helpers,
  Pane.svelte's derived set + class:staged
  binding + CSS.
* `web/src/components/paneModeKeymap.test.ts`:
  full rewrite of the spawn-commit + rich-
  prompt blocks — pins flipped from "commit
  immediately" to "stage, no commit", new
  tests for N + materialize ordering.
* `web/src/components/paneModeHelpClickable.test.ts`:
  Stage group title + N row + V drop.

### Decisions

* **`N` not `E` for new draft** — per
  @@Alex's in-flight ask. Mnemonic matches
  the top-level Cmd+N chord; both routes
  share `api.createDraft()`.
* **`V` stays aliased to `G`** — same
  keymap handler; documented in the chord-
  table comment. Lets muscle memory survive
  the rename.
* **Drafts created on Enter, not on press
  of N** — keeps Esc clean (no orphan
  drafts) at the cost of a slight delay on
  materialize. Acceptable; the round-trip
  is fast.
* **Materialize fires per-entry in
  parallel** — independent createDraft calls
  can race safely (each mints a unique
  filename); the queue snapshot up-front
  protects against the commitPaneMode
  side-effect clearing it mid-iteration.
* **Ghost tabs render in the draft layout,
  not as a parallel ghost-list** — the
  draft IS the staging area; tabs that
  exist in draft but not in live = staged.
  Cheaper + simpler than a sibling
  ghost-tabs array.

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1198 / 1198** (+14 from
  `-a-67f`'s 1184; 10 new pins + 4 test
  updates net).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy
  --all-targets -- -D warnings` → clean
  (no Rust delta).

### Suggested commit subject

```
Hybrid Nav: transactional T/O/P/G/N staging (fullstack-a-68 slice 2)
```

### Files (per-path)

* `web/src/state/tabs.svelte.ts`
* `web/src/App.svelte`
* `web/src/components/Pane.svelte`
* `web/src/components/PaneModeHelp.svelte`
* `web/src/state/paneModeStaging.test.ts` (new)
* `web/src/components/paneModeKeymap.test.ts`
* `web/src/components/paneModeHelpClickable.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-68.md`

Autonomous-commit mode. No clearance held.
`-a-68` umbrella closes. Picking up `-a-75`
(carousel) next.
