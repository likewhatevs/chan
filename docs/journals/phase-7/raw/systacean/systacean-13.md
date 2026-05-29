# systacean-13: activity indicator on terminal tabs

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-19

## Goal

Add a visual cue (small dot / pulse / colored icon) on
terminal tabs when a terminal has produced output since
the user last looked at it. Clears on focus. Lets the
user see at a glance which terminals are doing things vs
idle.

## Relevant links

* @@Alex's intent:
  [../request.md](../request.md) — Enhancements section,
  "Activity indicator on terminal tabs" bullet.

## Acceptance criteria

* When a terminal tab is NOT focused and its PTY
  produces output, the tab grows a small "activity"
  marker on the tab strip.
* The marker clears the moment the user focuses the
  tab.
* The marker style is subtle (small dot or color-shift
  on the tab icon), not jarring. Sits next to the
  existing dirty/watcher bullets without crowding.
* The marker should distinguish "fresh output since
  last focus" from "currently idle" — not just "ever
  had output".
* Backend signal: chan-server tracks per-session "bytes
  written since last attach-focus" or equivalent. The
  frontend reads via the existing terminal session
  state or a small new API.
* Tests for the backend signal + a render test on the
  frontend marker.

## Out of scope

* Sound notifications.
* "What kind of output" classification (just "had any
  output").
* Indicator persistence across full chan-server restart
  (state lives in the session; restart resets).

## How to start

1. Backend: in
   `crates/chan-server/src/terminal_sessions.rs` (or
   the PTY broker), add a `bytes_since_focus: u64`
   counter per session. Increment on PTY write.
   Reset when the frontend signals focus (existing
   focus signal exists; if not, add a tiny endpoint
   or piggy-back on the WS attach).
2. Frontend: the tab strip already renders the dirty
   bullet (file-save) and watcher bullet (from
   `fullstack-13`). Add a third lightweight slot for
   the activity marker.
3. Coordinate with @@FullStack only if you need a new
   small UI primitive; otherwise self-contained.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-systacean-architect.md`.

## 2026-05-19 05:10 BST - ready to land

Implemented terminal activity indication across the PTY session and
tab strip.

Details:

* chan-server tracks `bytes_since_focus` per PTY session and broadcasts
  `activity` frames on first unfocused output plus clear-on-focus.
* Terminal clients send a small `focus` WebSocket frame as active tabs
  change.
* The tab strip renders a subtle activity dot for terminals with fresh
  output since last focus, and clears it immediately on tab focus.

Verification:

* `cargo test -p chan-server activity --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `npm run test -- src/components/Pane.test.ts src/state/tabs.test.ts`
* `npm run check`
* `npm run build`
* `scripts/pre-push`
