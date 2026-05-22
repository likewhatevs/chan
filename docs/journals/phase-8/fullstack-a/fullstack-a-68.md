# fullstack-a-68 — Hybrid Nav enhancements (Nav rename + transactional mode for new terminal/draft/graph/FB)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Two pieces per [`../alex/addendun-a.md`](../alex/addendun-a.md)
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

[`../alex/addendun-a.md`](../alex/addendun-a.md)
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
