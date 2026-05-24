# Phase 9 Wave 1 Live Browser Verification Report

Author: @@WebtestLive
For: @@Architect
Date: 2026-05-23

## Scope

Ran live browser verification against current main, including the requested
Phase 9 Wave 1 checks plus the follow-up terminal stress and pane-mode terminal
identity investigation.

Requested commits present in the target main:

- `6d6f9f0`
- `18fe547`
- `e0a0fe4`
- `46f5703`

Read before testing:

- `~/.ai/profile.md`
- `AGENTS.md`
- `docs/journals/phase-9/request.md`
- `docs/journals/phase-9/roadmap-round1.md`

`docs/journals/phase-9/roadmap-round1.md` was not edited.

## Launch

Throwaway drive:

```text
/private/tmp/chan-phase9-wave1
```

Build and launch sequence:

```bash
npm run build
cargo build -p chan
./target/debug/chan serve /private/tmp/chan-phase9-wave1
```

Browser URL:

```text
http://127.0.0.1:8787/?t=df9bY7ve8BSUzwoqmWaTCiwprD4ozwsE
```

The server was stopped after verification.

## Viewports

- `1280x720`
- `390x740`

## Wave 1 Matrix

| Check                                      | Result | Notes |
| ------------------------------------------ | ------ | ----- |
| Terminal tab switch preserves glyphs       | PASS   | Font and glyph rendering held after switching to draft and back. |
| Terminal renders U+2014 and non-ASCII      | PASS   | U+2014, accented text, CJK, and symbols rendered correctly. |
| Flipped Hybrid left hamburger in viewport  | PASS   | Menu stayed inside viewport on desktop and narrow mobile viewport. |
| Editor bottom-cursor scroll-up stability   | PASS   | Manual upward scroll stayed up and did not snap back down. |
| Bullet markers distinct while typing       | PASS   | Ordered, unordered, and nested markers remained visually distinct. |
| Final-line `---` remains source text       | PASS   | Final-line triple dash remained editable source text. |
| Graph smoke                                | PASS   | Graph opened and rendered nodes and edges. |
| New Draft smoke                            | PASS   | New Draft opened editable draft surface. |
| File Browser smoke                         | FAIL   | Tab became active, but body stayed on welcome placeholder. Console showed Svelte `each_key_duplicate`. |

## Terminal Stress

Follow-up checks opened three terminal tabs and forced scrollback and ANSI
rendering.

Repro steps:

1. Open three terminals.
2. In one terminal run `ps aux`.
3. In a second terminal run a helper script with ANSI 256-color output and
   Unicode.
4. In a third terminal run a combined `ps aux` plus colored scrollback script.
5. Switch away from each terminal and back.
6. Inspect font shape, line wrapping, glyph rendering, and ANSI style state.

Result: PASS.

Observed:

- `ps aux` scrollback did not break terminal font rendering.
- ANSI foreground colors, truecolor, bold, italic, underline, and reverse video
  survived tab switching.
- Unicode and U+2014 rendered correctly after scrollback and tab switching.

## Pane-Mode Terminal Identity Bug

The user reported that creating three Hybrid panes, placing a terminal in each,
and committing left all three terminals acting as if broadcast was enabled even
when the broadcast setting was off.

Reproduced before fix.

Exact repro:

1. Enter Hybrid Nav.
2. Press `/`.
3. Press `/` again.
4. Press `T`.
5. Press `ArrowLeft`.
6. Press `T`.
7. Press `ArrowLeft`.
8. Press `T`.
9. Press `Enter` to commit.
10. Type `echo PANEBUG` in the focused terminal.

Observed before fix:

- All three panes showed `Terminal-1`.
- Output from `echo PANEBUG` appeared in all three panes.
- Broadcast UI was not enabled.

Root cause:

- `paneModeOpenTerminal()` allocated names with `nextTerminalTitle()` against
  the live `layout`.
- During a pane-mode transaction, the new terminal tabs exist only in the
  draft layout.
- Each staged terminal therefore received the same visible title,
  `Terminal-1`.
- `TerminalTab.svelte` connects with `windowId` plus `tabName`.
- `chan-server` terminal session lookup reattaches by `(window_id, tab_name)`
  when there is no existing session id.
- Three staged tabs with the same title in the same window reattached to the
  same PTY.

This was not true terminal broadcast mode. It was accidental PTY identity
collision.

## Impact Check

Impact appears limited to terminal-family components.

Terminal-family affected paths:

- Plain terminal placement with `T`.
- Rich-prompt terminal placement with `P`.

Reason:

- Terminal tabs are the components that pass `tab_name` and `window_id` to the
  server and can reattach to backend PTY sessions by that identity.
- File Browser and Graph can have duplicate visible titles, but those titles
  are client UI state only in this path and do not attach to server terminal
  sessions.

Fix included both `T` and `P`.

## Code Changes

Changed:

- `web/src/state/tabs.svelte.ts`
- `web/src/state/tabs.test.ts`

Behavioral change:

- Added layout-aware terminal title collection.
- `nextTerminalTitle()` can now allocate from a supplied `LayoutState`.
- Normal terminal creation still allocates from live layout.
- Pane-mode terminal creation now allocates from the draft layout.
- Pane-mode rich-prompt terminal creation now allocates from the draft layout.

Regression coverage:

- Added coverage for three pane-mode terminals created in one draft transaction.
- Added coverage for two pane-mode rich-prompt terminals created in one draft
  transaction.
- Tests assert distinct terminal titles and unique ids.

## Verification After Fix

Commands:

```bash
npm test -- --run src/state/tabs.test.ts
npm run build
cargo build -p chan
```

Results:

- `src/state/tabs.test.ts`: PASS, `127` tests passed.
- `npm run build`: PASS. Existing large chunk and dynamic import warnings
  remained.
- `cargo build -p chan`: PASS.

Live retest:

1. Rebuilt embedded frontend.
2. Relaunched `chan serve` against the throwaway drive.
3. Entered Hybrid Nav.
4. Pressed `/`, `/`, `T`, `ArrowLeft`, `T`, `ArrowLeft`, `T`, `Enter`.
5. Confirmed panes showed distinct names: `Terminal-3`, `Terminal-2`,
   `Terminal-1`.
6. Typed `echo FIXED` into the focused terminal.

Result: PASS. `echo FIXED` appeared only in the focused pane.

## Screenshots

All screenshots were written under:

```text
/private/tmp/chan-phase9-wave1-shots
```

Key evidence:

- `01-terminal-glyphs.png`
- `02-terminal-switch-back.png`
- `03-editor-markers-final-hr-source.png`
- `04-editor-scroll-up-stable.png`
- `05-flipped-menu-desktop.png`
- `06-flipped-menu-narrow.png`
- `07-file-browser-active-but-placeholder.png`
- `08-graph-smoke.png`
- `09-terminal1-ps-aux-switchback.png`
- `10-terminal2-ansi-switchback.png`
- `11-terminal3-ps-ansi-scroll-switchback.png`
- `12-terminal2-visible-ansi-style.png`
- `13-pane-mode-terminals-shared-output.png`
- `14-pane-mode-terminals-fixed-before-input.png`
- `15-pane-mode-terminals-fixed-after-input.png`

## Known Gaps

- File Browser still fails the smoke check. It opens an active tab, but the
  body remains on the welcome placeholder and the console reports Svelte
  `each_key_duplicate`.
- No File Browser fix was attempted in this batch.
- No commit was created.
- The screenshot directory is under `/private/tmp`; copy or archive it if the
  evidence needs to survive temp cleanup.

## Recommended Next Step

Commit the terminal identity fix and regression tests as one frontend state
change. Treat the File Browser `each_key_duplicate` failure as a separate
follow-up investigation.
