# fullstack-51: terminal line metrics — match iTerm's row-height + char-cell

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Goal

Running `claude` inside chan's terminal renders the
multi-line ASCII-art logo with broken vertical
stacking — the block characters that should form a
contiguous logo are visibly offset between rows. The
same `claude` invocation in iTerm renders cleanly.

@@Alex's screenshots 2026-05-19 14:30 BST show:

* iTerm: `Claude Code` logo block characters form a
  clean shape, aligned with the text rows next to
  them.
* Chan terminal: vertical block characters don't
  stack — appears the row height (or character cell
  metric) is slightly off, so each row of the logo
  starts at a different sub-pixel offset.

Match iTerm's rendering: each terminal row should be
exactly one character cell tall with no extra
spacing.

## Relevant links

* @@Alex's chat note + screenshots 2026-05-19 14:30 BST.
* xterm.js config + theme tokens live in chan's
  frontend terminal component (likely
  `web/src/components/TerminalTab.svelte` or a
  sibling that mounts the xterm instance).

## Acceptance criteria

* Run `claude` (or any TUI that uses block characters
  for ASCII art) inside chan's terminal — block
  characters form a contiguous, aligned shape with
  no inter-row gaps.
* Plain text rendering is unaffected (line spacing
  should still read comfortably for prose).
* Light + dark modes both render correctly.
* Matches iTerm reference at default zoom.

## Likely seam

xterm.js exposes:

* `lineHeight` option (default `1.0`, but chan may
  have overridden).
* `letterSpacing` (less likely the issue here, but
  worth confirming = 0).
* `fontFamily` + `fontSize` (a font that doesn't
  ship monospace metrics or has hinting issues at
  the chosen size can produce row drift).

iTerm uses the system font (SF Mono or Menlo by
default) with `lineHeight = 1.0` and no letter
spacing.

Check chan's xterm options:

* `lineHeight` should be `1.0` (or `null` —
  whichever yields iTerm-style packing).
* `fontFamily` should be a monospace stack that
  resolves to a real monospace font.
* `fontSize` should be even (helps integer pixel
  alignment).

## Out of scope

* Custom Powerline / Nerd Font handling.
* Per-terminal-tab font overrides.
* Color rendering changes.

## How to start

1. Open `claude` in chan's terminal + DevTools.
   Inspect the `.xterm-rows` row heights. Compare
   to iTerm's actual row pixel size at the same
   font.
2. Find chan's xterm config in the SPA. Toggle
   `lineHeight: 1.0` if it's currently anything
   else.
3. Re-test with `claude` and a `cat` of a block-
   character ASCII art file to confirm.
4. Regression test if practical (rendering tests
   are hard, but a metric assertion on the row
   height vs font size is feasible).

## Hand-off

Standard. Pre-push gate green. Visual eyeball
required — the lane-boundary rule allows an ad-hoc
chan serve + browser tab for this (teardown after).
Ping via `alex/event-fullstack-b-architect.md`.

## 2026-05-19 14:55 BST — implementation + visual verification

**Root cause:** `web/src/components/TerminalTab.svelte:266`
set `lineHeight: 1.15` in xterm.js' `new Terminal({ ... })`
options. xterm.js applies that as a multiplier to the
intrinsic font cell height; with SFMono 13px the
intrinsic cell is ~15px, and `ceil(15 × 1.15) = 18px`,
so each row was 18px tall while character glyphs
(including full-block `█` U+2588) were drawn at the
font's natural ~15px. That left ~3px of vertical
padding per row, producing the visible inter-row gap
in `claude`'s ASCII-art logo.

**Fix:** change `lineHeight: 1.15` → `lineHeight: 1.0`.
One-line diff. iTerm's default line height is also 1.0
for system monospace fonts, so this matches the
reference Alex compared against.

**Visual verification** (ad-hoc chan serve on a throw-
away drive at `/tmp/chan-test-fullstack-51`, browser
tab driven via Chrome MCP):

* Spawned a terminal tab with a connected bash PTY.
* `cat`-ed a five-line block-character ASCII art file.
* Measured the bounding boxes of the five rendered
  block-character rows:

```
row 0: top 76,  bottom 91,  height 15
row 1: top 91,  bottom 106, height 15
row 2: top 106, bottom 121, height 15
row 3: top 121, bottom 136, height 15
row 4: top 136, bottom 151, height 15
```

  Gap between consecutive block rows: `[0, 0, 0, 0]`
  (each row's top is exactly the previous row's
  bottom). Total span 75px = 5 × 15. Contiguous,
  matches iTerm.

* Confirmed the inline row style is
  `height: 15px; line-height: 15px;` (down from 18px
  on the old build), and the rendered font is
  SFMono-Regular 13px per `getComputedStyle` on the
  `.xterm-rows` element.

* Plain text rendering unchanged — bash prompt
  renders at the same 15px row height with no
  perceptible difference in prose readability (this is
  the iTerm default; the change brings us in line).

* Light + dark mode both unaffected — the
  `lineHeight` option lives at the xterm constructor
  call and doesn't branch on theme; my change is
  theme-agnostic by construction.

**Gate:**

* `npm run check` → 0 errors / 0 warnings.
* `npm run test` → 35 files / 319 tests passed.
* `npm run build` → clean.
* `scripts/pre-push` → green (clippy, cargo test all
  crates, cargo build with `--no-default-features`).

**Teardown:** chan serve killed (PID 41963),
`/tmp/chan-test-fullstack-51` drive unregistered +
directory removed, Chrome MCP tab closed.

**Out of scope reaffirmed:** didn't touch font family,
font size, letter spacing, theme tokens, or the
chan-server side.

**Commit readiness:**

* File: `web/src/components/TerminalTab.svelte` (1
  line change).
* Proposed message:
  ```
  Set xterm lineHeight to 1.0 for iTerm-matching row metrics (fullstack-51)
  ```
* Standing topic-level commit clearance applies.
  No HOLD pokes since the 14:30 BST cut.
