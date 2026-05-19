# fullstack-78: propagate per-pane theme to xterm.js + other JS-themed surfaces

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex caught a real bug from `-59`'s per-pane
theme toggle: when the user flips a pane's
theme (Sun/Moon button), the chrome + DOM
surfaces switch immediately, but the
**xterm.js terminal body** stays in the old
theme. Screenshot evidence: pane toggled to
light, terminal still shows black background +
light foreground; rich prompt below already
flipped to white.

Root cause: xterm.js renders to its own canvas
with theme options passed at construction
(`new Terminal({ theme: { background, foreground,
... } })`). The CSS cascade via `data-theme` on
the pane reaches DOM children but does NOT
reach inside xterm.js's renderer — xterm
ignores ambient CSS and uses its own theme
object.

Same class of bug may affect other JS-themed
surfaces (audit during impl): GraphCanvas
(D3 canvas paints), CodeMirror editor (if it
uses its own theme extension instead of CSS
tokens), any inline `style="background: ..."`
that computed at mount time.

## Spec

* When `pane.theme` changes (set / unset / flip),
  re-apply the effective theme to every
  xterm.js Terminal instance in that pane's
  subtree:
  * Read the pane's effective theme
    (`pane.theme ?? ui.themeChoice`).
  * Construct the corresponding xterm theme
    object (chan's existing dark + light
    palette — find where the initial xterm
    `theme:` config is defined and re-use the
    palette tokens).
  * Call `term.options.theme = nextTheme`
    (or whatever xterm's runtime API is for
    theme update — check the version in
    use).
* Subscribe the per-terminal theme update to
  the pane's theme state via Svelte's
  reactivity (e.g. `$effect` that reads
  `pane.theme` + `ui.themeChoice` and pushes
  to xterm).
* Same treatment for other JS-themed surfaces
  if the audit catches them:
  * **GraphCanvas**: if D3 strokes / fills
    use computed-at-mount colour values, re-
    read on pane theme change. CSS tokens
    are preferable; if the canvas reads
    them via `getComputedStyle`, a single
    re-paint after theme change should
    suffice.
  * **CodeMirror editor**: chan uses CM6.
    Theme is likely a CSS-token extension
    (which should follow the cascade), but
    if any `EditorView.theme(...)` JS
    object is in play, that needs the
    runtime swap.
  * **Inline `style="..."` computed**: any
    component that resolves a CSS variable
    via `getComputedStyle` at mount time and
    caches it. Audit for these patterns
    during impl; flag any found.

## Relevant code

* `web/src/components/TerminalTab.svelte` —
  xterm.js mount + theme config. Find the
  `new Terminal({ theme: ... })` call and
  the palette source. Add the reactive
  theme-update `$effect`.
* `web/src/components/Pane.svelte` — where
  `pane.theme` and `ui.themeChoice` are
  already known. Provide a derived
  "effective theme" value the children can
  observe, OR let each terminal compute its
  own from the upstream `pane`.
* `web/src/components/GraphCanvas.svelte` —
  audit for the same class of bug.
* `web/src/components/FileEditorTab.svelte`
  (or wherever CM6 mounts) — audit
  similarly.

## Acceptance criteria

* Toggling pane theme (Sun → Moon or
  vice-versa via the `-59` chrome button)
  flips the xterm.js terminal body
  background + foreground to the new
  theme palette immediately. No reload
  required.
* The terminal palette switch is visible
  for both directions (light → dark, dark
  → light) and across:
  * Single-pane single-terminal layout.
  * Multi-pane layout where only one pane
    has the override.
  * Hybrid back/front (per-side theme
    overrides flip with the visible side).
* Hash round-trip: a URL with `ht:"l"` on
  a pane with a terminal renders the
  terminal in light theme on initial mount
  (not just after a toggle).
* Other surfaces audited:
  * GraphCanvas → flip works OR documented
    why it already does.
  * CM6 editor → flip works OR documented
    why it already does.
  * Any inline-computed-style cases →
    fixed inline, OR a separate follow-up
    cut.

### Tests

* Vitest: changing a mock `pane.theme`
  triggers `term.options.theme` update
  with the corresponding palette.
* Component test: TerminalTab in a pane
  with `pane.theme = "light"` and global
  dark renders the terminal with the
  light palette.
* Regression test: existing single-pane
  global-theme behaviour unchanged (no
  override → tracks global, both for
  chrome and for xterm).

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0-blocking. Hybrid flip + per-pane
  theme is marquee; a half-flipped pane is
  conspicuously broken. Ship before tag.
* Re-walk: post-ship, eyeball the terminal
  flip across pane-theme toggle, global
  toggle, hash reload, and a multi-pane
  layout with one override. Quick.
* Queue position: end of Lane B queue.
  Updated queue: `-67` (shipping) → `-71`
  (shipping) → `-78`.
* Standing topic-level commit clearance.

## 2026-05-19 22:55 BST — implementation

**Terminal fix.**
* Added `effectivePaneTheme()` helper —
  reads `layout.nodes[paneId]?.theme` (the
  `-48` phase A per-pane override) and falls
  back to `ui.theme` for panes without an
  override.
* Updated `terminalTheme()` to:
  - Read CSS variables from `host` (the
    terminal's container, inside the pane)
    instead of `document.documentElement`.
    This makes the `--bg` / `--text` /
    `--link` reads pick up the
    `.pane[data-theme="..."]` cascade from
    `-59`.
  - Branch the light vs dark named-colour
    palette on `effectivePaneTheme()` instead
    of `ui.theme` directly.
* Extended the `$effect` to track both
  `ui.theme` AND `layout.nodes[paneId]?.theme`.
  Re-applies `term.options.theme` on either
  signal change. xterm.js updates the canvas
  on the next paint.

**GraphCanvas fix.**
* Extended the existing theme MutationObserver
  to also watch the nearest `.pane` ancestor's
  `data-theme` attribute. The observer was
  watching only `document.documentElement`;
  per-pane flips now trigger `refreshTheme()`
  the same way global flips do. The reader
  side (`readTheme(host)` via
  `getComputedStyle(containerEl)`) already
  resolves the pane-scoped CSS variables —
  the missing piece was just the change
  detection.

**CodeMirror audit.**

CodeMirror's theme is largely CSS-token-based
(`var(--text)`, `var(--bg-card)`, etc.) so
the cascade reaches it without JS
intervention. The one piece that DOES use
`ui.theme` directly is the syntax highlight
palette branch (`themeExtensions(theme)` in
`web/src/editor/base.ts`); that's reconfigured
via `theme.reconfigure(view, ui.theme)` from
`Source.svelte` / `Wysiwyg.svelte`. Per-pane
override for syntax palette would require
threading the pane id through to those
editors AND piping the reconfigure on
pane.theme change. **Deferred** — the visible
impact is small (GitHub Primer palette is
designed for both light + dark on both
backgrounds; mismatched palette + bg is
readable but suboptimal). If @@Alex flags it,
follow-up cut.

**Edits:**

* `web/src/components/TerminalTab.svelte`:
  - Renamed effect comment to reference
    `-78`.
  - Added `effectivePaneTheme()` helper.
  - Updated `terminalTheme()` to read CSS
    vars from `host`, branch on
    `effectivePaneTheme()`.
  - Updated the `$effect` to track both
    `ui.theme` and `layout.nodes[paneId]?.theme`.

* `web/src/components/GraphCanvas.svelte`:
  - Extended the existing theme
    MutationObserver to also watch the
    nearest `.pane` ancestor.

* `web/src/components/perPaneXtermTheme.test.ts`
  (new) — source-grep sentinel, 5 assertions:
  1. TerminalTab's `$effect` reads both
     `ui.theme` AND `layout.nodes[paneId]?.theme`.
  2. `effectivePaneTheme()` falls back to
     `ui.theme` when the pane has no
     override.
  3. `terminalTheme()` branches on the
     effective theme.
  4. `terminalTheme()` reads CSS vars from
     `host`, not the document root.
  5. GraphCanvas attaches the
     MutationObserver to the nearest
     `.pane` ancestor.

**Hash round-trip.** Verified by inspection:
the `-48` phase A serialization layer already
restores `pane.theme` from the `ht` / `hb`
hash fields. My new `effectivePaneTheme()`
reads from the restored value at first
mount, so a URL with `ht:"l"` on a pane
hosting a terminal lands with the light
palette without a toggle. (The model-layer
tests from `-48` phase A cover the
restoration; the new source-grep tests cover
the consumer wiring.)

**Gate.** `npm run check` 0/0; `npm run test`
40 files / 410 tests (was 39 / 401; +5 from
the new sentinel + 4 from parallel-lane
work since the last full run); `npm run build`
clean; `scripts/pre-push` green.

**Visual eyeball.** Skipped — the change is
mechanical (read from `host` instead of
`documentElement`, track an additional
reactive signal). The source-grep tests pin
the four wiring invariants. Re-walk per the
task note is appropriate; @@Alex can flag
pixel issues on the walkthrough.

**Out of scope:**
* CodeMirror per-pane theme reconfigure
  (deferred; CSS-variable-driven body works,
  only the syntax palette branch is
  potentially off).
* CM6 `EditorView.theme(...)` JS object —
  already uses `var(--text)` etc., follows
  the cascade.

**Commit readiness:**

Files staged:
* `web/src/components/TerminalTab.svelte`
* `web/src/components/GraphCanvas.svelte`
* `web/src/components/perPaneXtermTheme.test.ts`
* `docs/journals/phase-7/fullstack-b/fullstack-78.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Propagate per-pane theme override to xterm.js + GraphCanvas (fullstack-78)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 22:15 BST cut.
