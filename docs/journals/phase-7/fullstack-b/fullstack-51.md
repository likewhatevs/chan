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
