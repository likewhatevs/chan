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
