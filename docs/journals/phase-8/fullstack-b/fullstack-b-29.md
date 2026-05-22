# fullstack-b-29 — Enable xterm.js customGlyphs to fix box-drawing + block-element gap rendering

Owner: @@FullStackB
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Fix the ASCII table / mascot pixel-art rendering
bug where box-drawing + block-element glyphs render
with vertical gaps in the terminal.

@@Alex 2026-05-22 (re-flag): "the ascii / terminal
lines issue is still showing... have we got to that
task and to the reference I shared about xterm.js
for fixing this?"

## Reference

* `phase-8-bugs.md` lines 28-31 — full audit.
* xterm.js issue [#2409](https://github.com/xtermjs/xterm.js/issues/2409):
  "Manually draw pixel-perfect glyphs for Box
  Drawing and Block Elements characters."
* Fix shipped in xterm.js 4.14.0.
* We're on `@xterm/xterm@^6.0.0` (well past).

## Fix shape

**One-line addition** to `web/src/components/TerminalTab.svelte:344`'s
`new Terminal({...})` options:

```ts
new Terminal({
  ...existing options...
  customGlyphs: true,
});
```

`customGlyphs: true` tells xterm.js to manually draw
pixel-perfect glyphs for box-drawing + block-element
Unicode ranges instead of letting the system font
render them with implicit padding / anti-aliasing.

### Audit before commit

* Confirm xterm.js 6.x default for `customGlyphs`
  (may already default to `true`; if so the issue
  is elsewhere — e.g. renderer choice or
  font-weight interaction). Audit the upstream
  6.x release notes.
* Verify the active renderer (Canvas vs WebGL).
  `customGlyphs` may have constraints per renderer
  in newer xterm.js. Audit if needed.
* If `customGlyphs` is already `true` by default in
  6.x AND the issue persists, dig deeper —
  could be the chan-side terminal font, CSS
  `font-feature-settings`, or a related option
  like `allowProposedApi` / `drawBoldTextInBrightColors`.

## Acceptance

1. **Box-drawing characters connect cleanly**:
   ASCII tables (e.g. `┌─┬─┐ │ ├─┼─┤ │ └─┴─┘`)
   render as continuous lines, no vertical gaps
   between cell corners.
2. **Block-element glyphs render pixel-perfect**:
   Claude Code mascot pixel-art (▀ ▄ █ ▌ ▐ etc.)
   renders without gaps.
3. **No regression**: existing terminal rendering
   (text, colors, cursor, scrollback) unchanged.

### Tests

Vitest pin on `new Terminal({...})` carrying
`customGlyphs: true` (or whatever the fix shape
turns out to be after the audit).

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackB lane (you already touched
  `TerminalTab.svelte` in `-b-26` adding the
  Reload + Open Inspector entries).
* Atomic-audit-commit.
* Tiny fix; ~1-5 LOC depending on audit outcome.

## Authorization

Yes for `web/src/components/TerminalTab.svelte` +
test + task tail + outbound.

## Numbering

This is `-b-29`.

## Out of scope

* Re-architecting the terminal renderer (WebGL vs
  Canvas).
* Font changes.
* Other xterm.js 6.x option tuning unless surfaced
  by the audit as the actual cause.
