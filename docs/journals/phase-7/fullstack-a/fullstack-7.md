# fullstack-7: light-mode terminal contrast

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Bump foreground contrast in the terminal's **light-mode**
theme so faint output (esp. `\e[37m` white, dim ANSI colors)
is readable on the light background. Dark mode unaffected.

Wave 1.5. Concrete repro provided by @@WebtestB.

## Relevant links

* [../request.md](../request.md) Bugfixes — "Light-mode
  terminal: lighter glyphs..." bullet.
* @@WebtestB's repro at
  [../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md)
  — `\e[37m` is invisible (white-on-white); green/yellow/cyan
  are too pale.

## Acceptance criteria

* In light-mode terminal:
  * `\e[37m` white text is legible against the background
    (use a slightly darker / blue-shifted "light white" the
    way most terminal themes do).
  * Green, yellow, cyan, and any other dim ANSI colors meet
    a reasonable contrast ratio (target WCAG AA for body
    text on the theme background; aim 4.5:1 on regular
    weight, 3:1 acceptable for terminal output).
  * Bright variants (e.g., `\e[1;37m`) stay distinguishable
    from non-bright.
* Dark mode renders unchanged.
* Theme switch (light <-> dark) still works without artifacts.

## How to start

1. Find the terminal theme palette in `web/src/terminal/`
   (likely a theme object passed to xterm.js or similar).
2. Adjust the light-mode palette entries. Reference: most
   terminal themes use #aaaaaa-ish for "normal white" in
   light mode rather than pure white.
3. Visual confirmation: print a 16-color color test in a
   light-mode terminal and verify legibility for each.

## Out of scope

* Bigger theming work (palette redesign, additional themes).
* Dark-mode tweaks.

## Hand-off

Standard. Small task — likely one PR-shape commit.

## 2026-05-18 17:38 BST — implementation

Implemented the light-mode terminal contrast bump in
`web/src/components/TerminalTab.svelte`.

What changed:

* Split the xterm ANSI palette by resolved app theme.
* Left the existing dark-mode ANSI values unchanged.
* Added a light-mode palette with darker foreground colors for ANSI white,
  green, yellow, cyan, and the other standard/bright entries.
* Normal white is now `#6e7781` instead of near-white on the light terminal
  background; bright white is `#24292f`, so `\e[37m` and `\e[1;37m` remain
  distinguishable.

Contrast check against white background:

* green `#1a7f37`: 5.08:1
* yellow `#8a6300`: 5.43:1
* cyan `#1b7c83`: 4.93:1
* white `#6e7781`: 4.55:1

Verification:

* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

Notes:

* No manual browser screenshot pass performed in this lane.
