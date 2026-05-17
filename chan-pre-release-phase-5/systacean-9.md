# @@Systacean task 9: BUG-WT5-E — full-screen TUI apps don't redraw after a terminal reattach

Owner: @@Systacean
Status: REVIEW
Severity: MEDIUM — visual / UX bug on a feature that otherwise
works. Headline terminal-persistence contract still met
(the PTY survives + the shell is the same process); just the
TUI's on-screen state is broken until the user pokes something.
Source: Alex's 2026-05-17 bug report. "I ran htop in a terminal,
and refreshed the window; the terminal comes back like it needs
a reset. I'd expect my htop to refresh like it would when I
reopen my tmux -CC."

## Symptom

1. Start `htop` (or `vim`, `less`, `top`, any alt-screen TUI) in
   a terminal tab.
2. Reload the browser window.
3. Re-attach replays the byte ring, then the screen sits in
   a half-painted state: cursor in the wrong place, partial
   frames, no live updates from the TUI. Typing `Ctrl+L` or
   resizing the window unsticks it (forces a redraw).

Expected (tmux `-CC` behaviour): on reattach, the TUI's current
frame is repainted cleanly and live updates resume.

## Why this happens

[systacean-5](./systacean-5.md)'s ring buffer is byte-stream.
It captures every byte the PTY ever emitted (up to 1 MiB), and
the reattach replay drains it into the new xterm.js client. For
a line-mode shell, this is fine — replaying the bytes reconstructs
the visible state.

For an alt-screen TUI, this is **not** fine:

* The TUI entered alt-screen mode with `\x1b[?1049h` somewhere
  in the past. If that escape is still inside the ring window,
  the replay replays it — good. If it rolled off the ring, the
  client never sees alt-screen entry — bad.
* The TUI is constantly repainting cells with absolute cursor
  positioning. The byte ring captures all of those operations,
  but replaying them at high speed paints a confused picture
  because they were each emitted *after* the previous frame had
  been observed, not on a blank screen.
* The TUI doesn't know a new client attached. It keeps emitting
  *deltas* from its internal model, which the new client sees
  without the baseline.

tmux solves this by maintaining a logical screen model (cells,
cursor, attributes, alt-screen flag) and on attach sending a
synthesized "set window contents" prelude that brings the new
client into a known state — then live deltas paint correctly
from that baseline. We explicitly punted on that model when
picking Option 4 in [architect-tmux-1](./architect-tmux-1.md);
this task is about narrowing the gap without rebuilding tmux.

## Fix shape

Two layers, both small:

### Layer 1 (MVP): SIGWINCH on attach

On every new attach to an existing session, **after** the ring
replay completes, send `SIGWINCH` to the PTY's child process
group. The kernel forwards the signal to the foreground process
(htop, vim, …); the program treats SIGWINCH as "the window
size might have changed" and full-repaints from its internal
model.

Implementation: in `crates/chan-server/src/terminal_sessions.rs`,
in the attach path right after the replay buffer is sent on the
broadcast, call something like
`pty_master.resize(current_size)` (a no-op resize triggers
SIGWINCH on most `portable-pty` backends) or expose the child's
PID and call `nix::sys::signal::kill(child_pid, SIGWINCH)`.
Prefer the no-op resize route — it's the standard "tell the
program to redraw" idiom and avoids an extra dependency on
`nix`.

This closes ~90% of cases. htop, vim, less, top, btop, lazygit,
fzf, ncdu, ranger, nano all redraw on SIGWINCH.

### Layer 2 (nice-to-have, can ship separately): alt-screen-aware
ring

Track whether the session is currently in alt-screen mode by
sniffing `\x1b[?1049h` (enter) and `\x1b[?1049l` (exit) in the
output stream. If alt-screen is active at attach time, **skip
the ring replay** (the TUI will repaint the screen anyway after
SIGWINCH) and only stream live output from now on. This avoids
the confused-paint-during-replay artefact for alt-screen apps.

Sniffing is cheap: two byte-pattern matches per chunk written to
the ring; toggle a `Session.in_alt_screen: AtomicBool` accordingly.
Doesn't try to parse the whole VT100 grammar; both sequences are
unambiguous enough to spot with a substring search.

If layer 2 ships, layer 1 still runs after the no-replay path so
the program redraws.

## Acceptance criteria

* `htop` in a terminal tab + browser reload: after reattach,
  htop's UI is fully repainted within ~1 second and live updates
  resume. No `Ctrl+L` needed.
* `vim` mid-edit + reload: file content visible, cursor on the
  expected line, statusline correct, command-mode reachable
  without typing anything.
* `less` paging a big file + reload: screen redraws to the same
  page, `q` exits cleanly.
* Line-mode bash with `htop` history scrollback (no current TUI):
  reload still replays the visible prompt + recent commands
  correctly (current behaviour preserved).
* Full pre-push gate green.

## Test expectations

* Unit test in `terminal_sessions::tests`: attach to a session
  whose PTY master's `resize` (or signal-send) helper is mocked;
  assert the attach path invokes it after replay. Use the same
  test harness systacean-5 set up.
* Conditional integration test that asserts the child receives
  SIGWINCH after attach (use the existing `conditional_pty_*`
  pattern from `routes/terminal.rs::tests`).
* If layer 2 lands: a unit test that drives the ring with a
  `\x1b[?1049h` write, attaches, and asserts the replay is
  empty.

## Hardening

* `portable-pty`'s `resize` API on the BSD/Linux/Windows backends
  all map to TIOCSWINSZ; same-size resize emits SIGWINCH on the
  receiving side. Confirm on macOS first (the dev host); Linux
  is unlikely to surprise.
* If a TUI explicitly ignores SIGWINCH (rare; some lock-step
  daemons), document the limitation.
* Sniffing alt-screen escapes adds a sub-microsecond per output
  chunk. Confirm it doesn't show up in any benchmark from
  [systacean-4](./systacean-4.md).

## Coordination

* @@Webtest A re-runs the htop + vim + less reload trio on the
  fixed bundle once it lands. Service is up on PID 87431; they
  should rebuild as part of the next phase-5 wave anyway.
* The fix is server-side; no frontend change needed.

## Out of scope

* A full screen-model implementation (tmux-style). Explicitly
  deferred to a future phase per the
  [architect-tmux-1](./architect-tmux-1.md) decision.
* Cursor-position restore inside line-mode shells (not broken;
  the ring already covers it).
* Compatibility with screen multiplexers running INSIDE the chan
  terminal — orthogonal.

## Progress

* 2026-05-17 @@Systacean: picked up after task check found the
  unblocked TUI reattach redraw bug. Inspecting terminal registry
  attach/replay path before patching.
* 2026-05-17 @@Systacean: implemented the attach redraw and
  alt-screen replay suppression. Full gate is green.

## Completion notes

Implemented both layers:

* After WebSocket replay, `routes::terminal::terminal_ws` calls
  `AttachHandle::request_redraw()`. The registry sends a `Redraw`
  command through the PTY controller thread, which re-applies the
  current winsize with `portable-pty::MasterPty::resize`. That is
  the no-op resize/SIGWINCH path TUIs use for repaint.
* `terminal_sessions::Session` now tracks alt-screen state by
  sniffing `\x1b[?1049h` / `\x1b[?1049l` in output chunks. When
  a session is currently in alt-screen mode, attach replay is
  skipped; the forced redraw provides the fresh screen instead of
  replaying stale frame deltas.

New tests:

* `terminal_sessions::tests::alt_screen_active_skips_replay_until_exit`
* `terminal_sessions::tests::alt_screen_sniffer_matches_expected_sequences`
* `terminal_sessions::tests::request_redraw_broadcasts_current_size`

Also re-ran `routes::terminal::tests::conditional_pty_programs_validate_real_terminal`.

Full gate green:

* `cargo fmt --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo build --no-default-features`
* `cargo test`
* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build`

Live htop / vim / less reload transcripts are not run in this lane;
@@Webtest A owns that browser re-smoke per coordination above.
