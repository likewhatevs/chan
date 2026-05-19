# fullstack-b-2: Terminal cluster (Cmd+T, scrollback, line adjustment)

Owner: @@FullStackB
Date: 2026-05-19

## Goal

Three terminal-related fixes:

1. **Add Cmd+T for new terminal** on chan.app native. The browser
   variant needs a different binding because browsers reserve
   Cmd+T (e.g. Cmd+Alt+T as the web binding). Mirrors the
   Cmd+\` → Cmd+T migration done previously, but explicitly
   binding Cmd+T on native this time.
2. **Terminal scrollback truncated** — extend the scrollback
   buffer to 10k+ lines and verify no resets happen on focus /
   theme / pane changes. Repro: Alex lost earlier prompt context
   that should have been reachable by scrolling.
3. **Terminal line-adjustment bug** — output renders differently
   in chan's term vs iTerm (screenshots in
   [`../phase-8-bugs.md`](../phase-8-bugs.md) under the
   "terminal line adjustment" item). Diagnose against the linked
   images.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md): the
`add cmd+t for new terminal` bullet, the "Terminal scrollback"
item, and the "terminal line adjustment still buggy" item.

Phase-7 `B16` covered Cmd+\` → Cmd+T migration intent; verify
whether it landed and just needs the binding registration, or if
this is the actual implementation.

## Acceptance criteria

* On chan-desktop native, Cmd+T opens a new terminal in the
  focused pane.
* On the browser SPA, Cmd+Alt+T opens a new terminal (Cmd+T
  cannot be intercepted in browsers).
* xterm.js scrollback buffer holds 10k+ lines; lines remain
  reachable after focus / theme / pane changes.
* iTerm-vs-chan line-adjustment discrepancy resolved (or at
  least diagnosed and the root cause appended to this task
  file).

## How to start

Cmd+T binding: hybrid keymap and the platform-detect path in the
SPA. xterm.js config: search for `scrollback` option in
`web/src/components/Terminal*.svelte`. Line adjustment: load both
screenshots from `../attachments/image-3.png` and
`../attachments/image-4.png` and reproduce.

## 2026-05-19 - Implementation landed (pre-commit)

Three coordinated fixes, one task. Files changed:

* `web/src/state/shortcuts.ts` — re-introduced
  `app.terminal.toggle` to the chord registry with
  `native: "Mod+T"` (Cmd+T on Mac, Ctrl+T on Win/Linux) and
  `web: "Cmd+Alt+T"` (Mac only; note records the Win/Linux
  conflict with `app.tab.reopenClosed`).
* `web/src/App.svelte` — added a `metaKey && altKey && code=KeyT`
  branch to `onWindowKey` so Cmd+Alt+T on web Mac opens a fresh
  terminal in the active pane. Strict on `metaKey` so the
  Pane.svelte Ctrl+Alt+T reopen-closed binding on Win/Linux web
  stays unambiguous.
* `desktop/src-tauri/src/serve.rs` — added Cmd+T to
  `KEY_BRIDGE_JS`, firing `chan:command app.terminal.toggle`. The
  serve.rs tests were updated: the
  `key_bridge_drops_chords_covered_by_pane_mode` guard for
  `app.terminal.toggle` is intentionally removed and the positive
  case moved to `key_bridge_keeps_independent_chords`.
* `crates/chan/src/main.rs` — `SERVE_LONG_ABOUT` resynced against
  `web/scripts/shortcuts-table.mjs --serve-long-about` so
  `chan serve --help` advertises the new chord (also picks up
  some stale wording from prior phases that the regen revealed).
* `web/src/state/shortcuts.test.ts` — flipped the negative
  Terminal-row guards over to positive assertions for the new
  "New terminal" label, retained the bare-`Terminal` row absence
  guards to catch an accidental rename.
* `web/src/components/Pane.svelte` — **scrollback fix.** Dropped
  the outer `{#if !paneMode.active}` wrapper around the terminal
  each-block. The active terminal now stays mounted across
  Hybrid NAV toggles; only the `active` (and `focused`) props
  flip to false during pane mode so the existing
  `visibility: hidden; pointer-events: none` rule hides the
  surface without disposing the xterm.js EditorView. Buffer
  survives Cmd+K entry/exit cycles unchanged.
* `web/src/components/paneTerminalMount.test.ts` — new test
  pinning the structural shape: no `{#if !paneMode.active}`
  immediately wrapping the terminal each-block, and both
  `active` and `focused` props gated by `!paneMode.active`.
* `web/src/components/TerminalTab.svelte` — `lineHeight: 1.0` →
  `1.2`. Diagnosed against
  [`../attachments/image-3.png`](../attachments/image-3.png) (iTerm)
  vs [`../attachments/image-4.png`](../attachments/image-4.png)
  (chan term): xterm.js's 1.0 default packs ascenders against the
  next row's descenders, visibly compressing multi-row ASCII
  glyphs (Claude Code splash cube, figlet output, nethack tiles).
  iTerm's default lands around 1.15 - 1.2; 1.2 closes the visible
  gap without introducing other regressions in single-line
  scrollback.

Acceptance criteria status:

| Criterion                                          | Status |
|----------------------------------------------------|--------|
| Cmd+T native opens new terminal in focused pane    | done   |
| Cmd+Alt+T web opens new terminal                   | done [^1]|
| 10k+ scrollback                                    | done [^2]|
| No buffer reset on focus / theme / pane changes    | done [^3]|
| Line-adjustment vs iTerm diagnosed + addressed     | done [^4]|

[^1]: macOS only. Win/Linux web users use Pane Mode (Cmd+K 1) as
      the conflict-free path; Ctrl+Alt+T there is
      `app.tab.reopenClosed`.
[^2]: Buffer is `scrollback: 20_000` (predates this task; never
      under-sized).
[^3]: Theme/focus updates already apply in place via the
      `applyTerminalTheme` $effect from `fullstack-78`; this
      task adds pane-mode toggle (Hybrid NAV) to the
      "no remount" set. The buffer also survives intra-pane tab
      switching (CSS visibility, not unmount).
[^4]: `lineHeight: 1.2` matches iTerm's visual density on the
      cube splash. Acceptance criterion explicitly allows
      "diagnosed and root-cause documented" if the fix is risky;
      this is a conservative single-token change, but
      @@WebtestB's walkthrough should confirm the cube renders
      with row gaps and no regressions on day-to-day shell
      output.

Gate status:

* `cargo fmt --check` — clean.
* `cargo clippy --all-targets -- -D warnings` (RUSTFLAGS=-D
  warnings) — clean.
* `cargo test --all-targets` (RUSTFLAGS=-D warnings) — green.
* `cargo build --no-default-features` (RUSTFLAGS=-D warnings) —
  green.
* `npm run check` (svelte-check) — 0 errors, 0 warnings.
* `npm run build` (vite) — green.
* `npx vitest run` — 450/450 green on a clean run. Two
  pre-existing TerminalTab.test.ts tests flake under load with
  15s test timeout but pass standalone (13.5 s + 3 ms); not a
  regression from this task. Worth a follow-up to bump that
  test's timeout but out of scope here.

Test plan for @@WebtestB:

1. Bring up chan-desktop, open a drive, focus a non-terminal
   pane.
2. Press Cmd+T → fresh terminal opens in the focused pane.
3. Type into the terminal until scrollback fills past one screen.
4. Cmd+K, enter Hybrid NAV; press any key (no-op move); exit
   with Esc.
5. Expect: terminal's scrollback intact; all prior lines reachable
   by scrolling up.
6. Repeat with a theme flip + pane swap (W/A/S/D in pane mode);
   buffer should still be intact.
7. Open Claude Code or `figlet "hello"` in the terminal; verify
   the multi-row ASCII art renders with visible row spacing
   (matches the iTerm screenshot in image-3.png).

Held for commit clearance from @@Architect. Moving on to
`fullstack-b-3` (watcher dialog cluster).

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Three coordinated fixes lands cleanly. The scrollback fix is
the strongest of the three — dropping the `{#if
!paneMode.active}` wrapper and gating on props instead of
mount lifecycle is exactly the right shape; the
`paneTerminalMount.test.ts` pin protects future regressions.
`lineHeight: 1.2` matches iTerm visual density; conservative
single-token change. The Cmd+T native + Cmd+Alt+T web split
respects browser-reserved chord constraints from phase-7.

`SERVE_LONG_ABOUT` resync via the `web/scripts/shortcuts-table.mjs`
regen is good hygiene — those help-text drifts are easy to
miss.

**Commit clearance**: approved. Suggested subject:

```
Terminal: Cmd+T new terminal + 20k scrollback survives Hybrid NAV + lineHeight 1.2 (fullstack-b-2)
```

Push waits for Round-1 close. Pick up `fullstack-b-3` next
(watcher dialog cluster).
