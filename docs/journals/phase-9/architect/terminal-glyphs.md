# Terminal Glyphs

Date: 2026-05-24
Owner: @@Architect
Status: live visual renderer verified, current transport path verified

## Roadmap Items

- Terminal fonts not rendering after switching tabs.
- Terminal cannot render certain characters, for example em dash.

## Current Evidence

Existing static coverage pins the intended frontend behavior:

- `TerminalTab.renderer.test.ts` verifies WebGL addon wiring, context-loss
  fallback, atlas refresh after styled output, binary output writes, font
  readiness refresh, and no string coercion of binary terminal output.
- `TerminalTab.font.test.ts` verifies the OS-default and Source Code Pro font
  chains plus cursor defaults.
- `webtest-live-wave-1-report.md` records Browser/iab PASS evidence for
  terminal tab switching, U+2014 rendering, non-ASCII glyph rendering, ANSI
  style retention, and glyph stability after scrollback and tab switching.
- WebtestLive's later a210034 run opened a terminal from a draft, but did not
  repeat the glyph probe because Browser/iab terminal typing and output reads
  were unreliable for that MCP probe.

## Transport Probe

Ran a throwaway `chan serve` with isolated HOME and drive, then connected
directly to `/api/terminal/ws` and sent:

```text
printf 'OUT alpha \342\200\224 beta \316\273 \342\234\223\n'
```

The PTY/WebSocket output preserved the expected UTF-8 glyphs:

```text
OUT alpha [U+2014] beta [U+03BB] [U+2713]
```

This rules out corruption in the server terminal route, PTY input path, PTY
output path, WebSocket binary frames, and frontend binary-write contract. The
renderer path already has Browser/iab evidence from the earlier wave report.

## Limitation

The Architect session's Browser plugin reported no available `iab` browser
instances, so this lane could not capture a fresh screenshot from current HEAD.
The existing Browser/iab report covers the visual renderer, but a current-HEAD
refresh can be requested if this area is touched again.

## Optional Current-HEAD Refresh

Ask WebtestLive to open a terminal, run:

```sh
printf 'OUT alpha \342\200\224 beta \316\273 \342\234\223\n'
```

Then verify visually that U+2014, U+03BB, and U+2713 render as those glyphs
after:

- initial terminal output
- switching away to a different tab and back
- focusing the terminal again

Capture a screenshot only if any glyph renders incorrectly or changes after
tab switching.
