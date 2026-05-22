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
