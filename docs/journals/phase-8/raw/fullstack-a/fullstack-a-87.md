# fullstack-a-87 — Rich prompt cursor / placeholder Y-axis misalignment (follow-up to -a-84)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Align the empty-state placeholder VERTICALLY with the
CM6 cursor. `-a-84` fixed the X-axis overlap (10px
offset right) but exposed/introduced a Y-axis
misalignment: the cursor sits ABOVE the placeholder
text baseline.

## Reference

@@Alex 2026-05-22 screenshot: cursor `|` visibly
above the "Write a multi-line command and Cmd+Enter"
text. Both should sit on the same baseline.

`-a-84` task body + commit `3869a07` for the prior
X-axis fix shape.

`web/src/components/TerminalRichPrompt.svelte:812`
`.prompt-placeholder`:

```css
.prompt-placeholder {
  position: absolute;
  top: var(--editor-top-pad, 16px);
  left: calc(1rem + 10px);  /* -a-84 fix */
  ...
  line-height: 1.5;
}
```

## Audit hypotheses

### H1 — CM6 cursor top offset differs from --editor-top-pad

`var(--editor-top-pad, 16px)` may not match the
actual cm-line top offset. CM6 internally pads the
cm-content via `cm-line` with its own padding-top
(e.g. CM6 default is ~4px). 16px pad on placeholder
vs ~4px on cursor → 12px Y delta.

### H2 — line-height vs cm-line line-height mismatch

`.prompt-placeholder { line-height: 1.5 }` may
differ from CM6's cm-line line-height. Different
line-heights with text-baseline alignment shifts
the baseline.

### H3 — font-family render-metric mismatch

`.prompt-placeholder { font-family: var(--chan-editor-body-family, inherit) }`
may resolve to a different font than CM6 uses
(inherited from a different ancestor), changing
the baseline.

## Fix shape

After audit, the most likely fix is in CSS:

* Either match `.prompt-placeholder { top: ... }`
  to CM6's actual cm-line top offset.
* OR use `display: flex; align-items: baseline;`
  on a wrapper to baseline-align with CM6.
* OR set `line-height` + `padding-top` on
  placeholder to MIRROR cm-line's box.

Implementer picks after empirical audit (browser
devtools → inspect cm-line position, then adjust).

## Acceptance

1. **Cursor + placeholder share baseline**: the
   cursor `|` aligns with the placeholder text's
   first-character baseline.
2. **No regression on X-axis offset from -a-84**:
   10px right offset preserved.
3. **No regression on hide-on-type or
   blur-on-empty**: existing conditional render
   unchanged.

### Tests

Vitest pin on whatever CSS change shape lands +
audit comment block documenting CM6's baseline
geometry so future tweaks reference it.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* CSS-only audit + fix; tiny.
* @@WebtestA empirical walk after fix lands.

## Authorization

Yes for `web/src/components/TerminalRichPrompt.svelte`
+ test + task tail + outbound.

## Numbering

This is `-a-87`.

## Out of scope

* Re-architecting placeholder via CM6 extension
  (deliberate CSS overlay per `-a-24`).
* Font / size changes.

## 2026-05-22 — ready for review (H2 line-height mismatch)

Two-file change. SPA-only. CSS-only fix.

### Audit verdict

H2 confirmed: CM6 cm-line in Wysiwyg uses
`line-height: 1.8` (standard density;
`Wysiwyg.svelte:749`) while the placeholder
used `line-height: 1.5`. Different block
heights produced different baseline positions
within the same `top: 16px` line. Cursor's
visual block extended further down than the
placeholder text's, putting the visible
baselines out of alignment.

H1 (cm-line top offset vs `--editor-top-pad`):
ruled out — Wysiwyg's `.cm-content` consumes
the same `--editor-top-pad: 16px` the
placeholder uses, so both start at the same y.
H3 (font-family) ruled out — both consume
`--chan-editor-body-family` via the
`composer-editor` cascade.

### Fix

`web/src/components/TerminalRichPrompt.svelte`:

* `.prompt-placeholder { line-height: 1.5 }`
  → `line-height: 1.8`. Matches the
  standard-density cm-line default.
* Rationale comment cites the
  `Wysiwyg.svelte:749-750` density rules +
  the standard-vs-compact 0.15 drift (the
  drift is visually imperceptible at the
  rich prompt's 16px body size).

`web/src/components/richPromptPlaceholderBaseline.test.ts`
(new): 4 raw-source pins covering the
line-height swap, the rationale comment, the
preserved `-a-84` X-offset, and the preserved
`top: var(--editor-top-pad, 16px)`.

### Acceptance

1. Cursor + placeholder share baseline ✓
   (mechanism via tests; @@WebtestA empirical
   walk for confirm).
2. `-a-84` X-offset preserved ✓ (10px right).
3. No regression on hide-on-type /
   blur-on-empty ✓ — conditional render
   untouched.

### Gate

* vitest **933 / 933** (+4 net from `-a-83`'s
  929).
* svelte-check 0 errors / 0 warnings across
  4028 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Match standard-density default** (1.8)
  rather than introduce a CSS variable
  threaded through both Wysiwyg/Source +
  the placeholder. Compact-density drift is
  small enough to ignore; if @@Alex's eye
  catches it on a compact drive, a future
  slice can plumb the var.
* **Did NOT touch `top` or font-family** —
  H1/H3 ruled out; conservative scope.

### Suggested commit subject

```
Rich prompt: match placeholder line-height to CM6 cm-line baseline (fullstack-a-87)
```

Single commit. CSS swap + 4 test pins.

### Files for `git add` (per-path discipline)

* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/richPromptPlaceholderBaseline.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-87.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
