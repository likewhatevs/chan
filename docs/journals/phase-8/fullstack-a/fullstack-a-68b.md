# fullstack-a-68b — Hybrid Nav rename: sweep missed shortcuts.ts label (slice 1 PARTIAL closure)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Catch the `shortcuts.ts` label miss in the
Hybrid NAV → Hybrid Nav rename from `-a-68 slice 1`.
@@WebtestA's walk flagged PARTIAL.

## Reference

@@WebtestA's walk (`3328d57`) verdict: `-a-68
slice 1` PARTIAL — `shortcuts.ts` still has
"NAV" / "NaV" labels somewhere the audit-grep
missed.

## Scope

Tiny sweep:

1. `grep -in "NAV\|NaV" web/src/state/shortcuts.ts`
   (case-insensitive; expected: zero matches for
   pre-rename forms).
2. For any hit: rename to "Nav".
3. Update any test pin that asserts the old form.

## Acceptance

1. **No "NAV" / "NaV" label remnants** in
   `shortcuts.ts` (case-exact sweep).
2. **Rename doesn't break chord bindings** — the
   accelerator / handler wiring stays; only
   label text changes.
3. **Any related test pins** for the new label.

### Tests

Vitest pin asserting the new "Nav" label literal
+ absence of pre-rename forms.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Tiny ~3-5 LOC + test.

## Authorization

Yes for `web/src/state/shortcuts.ts` + test + task
tail + outbound.

## Numbering

This is `-a-68 slice 1b` (follow-up under the
`-a-68` umbrella; slice 2 transactional mode
unchanged).

## Out of scope

* `-a-68 slice 2` transactional T/O/P/G/E
  staging (separate slice).
