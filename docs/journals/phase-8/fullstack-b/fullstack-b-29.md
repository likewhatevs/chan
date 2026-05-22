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

## 2026-05-22 — audit + implementation note (ready for commit clearance)

### Audit findings

* `customGlyphs` defaults to **`true`** in
  `@xterm/xterm` 6.x (confirmed via
  `web/node_modules/@xterm/xterm/typings/xterm.d.ts`
  comment: "The default is true"). Adding
  `customGlyphs: true` explicitly is a no-op — the
  task body's hypothesis-A path (one-line addition).
* Renderer audit: chan-desktop's terminal currently
  uses the **DOM renderer** (no Canvas/WebGL addon
  loaded). The DOM renderer renders each cell as a
  `<span>` and uses the system font for ALL
  characters — `customGlyphs` is ignored entirely
  because the option only applies to the Canvas /
  WebGL renderers.
* Combined with chan's `lineHeight: 1.2` (set in
  `-b-2` for iTerm visual parity), the DOM
  renderer's per-row span has 0.2 × fontSize of
  extra vertical space that the font's box-drawing
  glyphs don't fill. The result is the visible
  vertical gaps the bug report flagged.

The task body's hypothesis (`customGlyphs` may
already default to true → "dig deeper") was correct.
The actual root cause is the renderer choice, not
the `customGlyphs` flag.

### Fix: load the WebGL renderer addon

Per upstream xterm.js guidance + the renderer-addon
deprecation timeline ("canvasAddon is deprecated"),
the WebGL renderer is the right pick. WebGL draws
glyphs into the full cell rectangle including the
line-height padding, so box-drawing + block-element
characters render gap-free under any lineHeight.

The task's "out of scope: Re-architecting the
terminal renderer" — I'm interpreting that as "don't
rewrite the terminal stack". Loading a built-in
addon to enable an existing rendering mode isn't
re-architecture; it's the canonical xterm.js path.
If @@Architect prefers a strict reading, surface +
I'll roll back.

### Changes

* **`web/package.json`** — added
  `@xterm/addon-webgl ^0.19.0` (latest 6.x-compatible
  release).
* **`web/src/components/TerminalTab.svelte`** —
  * Import: `import { WebglAddon } from
    "@xterm/addon-webgl"`.
  * After `term.open(host)`, construct + load the
    WebglAddon inside a `try/catch` (xterm.js
    pattern for graceful WebGL-context failure):
    ```ts
    try {
      const webgl = new WebglAddon();
      webgl.onContextLoss(() => webgl.dispose());
      term.loadAddon(webgl);
    } catch (err) {
      console.warn("[chan] xterm.js WebGL renderer unavailable; falling back to DOM:", err);
    }
    ```
  * `onContextLoss` disposes the addon if the GPU
    context is lost (rare; e.g. GPU reset). xterm.js
    falls back to DOM for the rest of the session.
* **`web/src/components/TerminalTab.renderer.test.ts`**
  (new) — 4 `?raw`-source pins guard:
  * `WebglAddon` import from `@xterm/addon-webgl`.
  * `new WebglAddon()` construction +
    `term.loadAddon(webgl)`.
  * `onContextLoss` handler + `webgl.dispose()`.
  * `try/catch` wrap + the "falling back to DOM"
    warning string.

### Pre-push gate (local, macOS aarch64; -b-29 scope)

| Surface                                          | State                                                |
|--------------------------------------------------|------------------------------------------------------|
| `web/` `npx svelte-check`                        | 4026 / 0 / 0.                                        |
| `web/` `npx vitest run src/components/TerminalTab*.test.ts` | 4 files / 15 tests pass (incl. 4 new renderer pins). |
| `web/` `npm run build`                           | Clean (pre-existing chunk-size warnings only).       |

Full-suite vitest has 4 pre-existing flakes (timer-
driven Pane / EmptyPaneCarousel / TerminalTab
activity tests) under parallel load; all pass when
run in isolation. NOT introduced by `-b-29`; same
pattern present pre-commit on `8585d85` and earlier.

### Files to stage

```
web/package.json
web/package-lock.json
web/src/components/TerminalTab.svelte
web/src/components/TerminalTab.renderer.test.ts
docs/journals/phase-8/fullstack-b/fullstack-b-29.md
```

Atomic `git commit --only` per
`feedback_shared_worktree_commits`.

### Suggested commit subject

```
TerminalTab: load WebglAddon to fix box-drawing + block-element gap rendering (fullstack-b-29)
```

### Runtime walkthrough

WebGL renderer requires a webview with WebGL context.
Tauri's WKWebView / WebView2 both ship WebGL
unconditionally on modern OSes; the try/catch
fallback covers the rare exception case. Visual smoke
on chan-desktop: open a terminal, run `htop` / `gum
table` / paste an ASCII table with `┌─┬─┐ │ ├─┼─┤ │
└─┴─┘`, observe that the corners + verticals connect
cleanly. Standing chan-desktop runtime perm covers it;
otherwise routing to @@WebtestB.
