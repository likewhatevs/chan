# @@Systacean task 10: BUG-WT5-E round-2 — alt-screen replay suppression + force-redraw on attach

Owner: @@Systacean
Status: REVIEW
Severity: MEDIUM (HIGH for the UX of any TUI workflow) — closes
out the headline terminal-persistence contract for full-screen
applications.
Source: [webtest-1](./webtest-1.md) round-9 smoke, with Alex's
side-by-side screenshot diff confirming the partial-redraw.

## What round-9 actually showed

systacean-9 implemented both layers from the original brief: a
post-replay no-op resize and an alt-screen sniff with replay
suppression. On a live `htop` reload, the result is:

* **Dynamic data updates**: PIDs, CPU%, selection highlight all
  move correctly. So htop IS alive and IS receiving the post-
  attach SIGWINCH (its periodic `refresh()` redraws the *cells*
  it normally repaints).
* **Static chrome stays missing**: CPU index labels (`0[`, `1[`,
  …), CPU percentage labels (`38.0%]`), header words (`Tasks:`,
  `Load average:`, `Uptime:`), the process-table header row
  (`PID USER PRI NI VIRT RES S CPU%▽ MEM% TIME+ Command`), the
  `Main` tab pill, and the `F1–F10` footer are all gone. The
  USER / PRI / NI columns are populated only for ~half the rows.

The pattern is the load-bearing clue: htop draws the chrome
**once at startup** (and on a real `LINES`/`COLUMNS` change), then
emits only delta cell writes for dynamic data. On reattach:

1. The alt-screen sniff suppressed the byte-ring replay (good).
2. The no-op `pty_master.resize(current_size)` delivered SIGWINCH
   but the size didn't actually change. htop's signal handler
   sees "size unchanged" and re-runs `refresh()`, which paints
   the dynamic cells but does **not** re-emit the chrome.
3. The new xterm.js instance therefore receives only the delta
   stream from then on. Chrome is permanently invisible until the
   user types `Ctrl+L` (which htop maps to "force full redraw").

## Fix shape

Two changes inside `crates/chan-server/src/terminal_sessions.rs`:

### 1. Cross-chunk-safe alt-screen sniff

The current sniff likely matches `\x1b[?1049h` / `\x1b[?1049l`
against each ring write in isolation. xterm.js doesn't guarantee
the sequence lands in a single PTY chunk — it can split across
read boundaries. Add a small rolling tail buffer (the last
~8 bytes of output) on the Session, and prepend it to the next
chunk before scanning. Reset after a match. This makes the sniff
robust to chunk boundaries without parsing the whole VT100
grammar.

Confirm the sniff is actually working by adding a debug log line
on every state transition (`tracing::debug!("alt_screen entered/
exited")`); Webtest A will look for the line in `server-r10.log`.

### 2. Force-redraw via winsize wobble (not no-op resize)

Replace the post-replay `pty_master.resize(current_size)` with a
real one-tick wobble:

```rust
// htop / vim / less re-emit the *chrome* (not just dynamic
// cells) only on a real LINES/COLUMNS change. A no-op resize
// delivers SIGWINCH but the TUI's signal handler sees size
// unchanged and skips the full repaint. Send a one-tick wobble
// so the TUI thinks the window grew, then snap back; the
// repaint that fires for the first transition draws everything,
// the second transition is a cheap no-op for the program.
let original = current_winsize();
let wobble = PtySize {
    cols: original.cols,
    rows: original.rows.saturating_sub(1).max(1),
    pixel_width: original.pixel_width,
    pixel_height: original.pixel_height,
};
pty_master.resize(wobble)?;
// 50ms is enough for the TUI to handle the first SIGWINCH and
// emit the wide repaint; less than that and some programs
// coalesce both signals into one.
std::thread::sleep(std::time::Duration::from_millis(50));
pty_master.resize(original)?;
```

Apply this in the controller-thread Redraw command handler, not
the WS task — same place the no-op resize currently lives.

### 3. Optional but recommended: send an alt-screen prelude

When the alt-screen sniff confirms the session is in alt-screen
mode at attach time, also push a small prelude through the WS
broadcast **before** the wobble: `\x1b[?1049h\x1b[2J\x1b[H`
(enter alt-screen, clear screen, home cursor). This brings the
fresh xterm.js client into the same alt-screen state the program
thinks it's in, so the first repaint after the wobble paints on
a clean canvas. Without this, the prior session's last few bytes
in the xterm.js buffer can leak through.

If shipping it inflates the diff too much, ship steps 1+2 alone
and validate with htop first; the wobble is the primary fix.

## Acceptance criteria

* `htop` in a terminal tab + browser reload:
  * Chrome (CPU labels, header words, footer F-keys, column
    headers) is fully visible within ~200ms of reattach.
  * Dynamic data continues updating live.
  * No `Ctrl+L` needed.
  * Identical visual state to a fresh-launch htop, per a Webtest
    A side-by-side screenshot diff (this is now the canonical
    acceptance image — Alex caught the round-9 regression that
    way).
* `vim` mid-edit + reload: file content visible, statusline
  visible, mode indicator visible, syntax-highlighting colours
  intact.
* `less` paging a big file + reload: same page visible,
  `:` prompt at bottom visible, `q` exits cleanly.
* Plain bash with scrollback + reload: prior prompt + recent
  output visible (the non-alt-screen replay path stays
  unchanged).
* Server log shows `alt_screen entered` when the user runs htop,
  `alt_screen exited` when htop exits.
* Full pre-push gate: `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --no-default-features`,
  `cargo test`, `npm --prefix web run check`, `npm --prefix web
  test -- --run`, `npm --prefix web run build`.

## Test expectations

* Unit test in `terminal_sessions::tests` for the cross-chunk
  sniff: drive `\x1b` and `[?1049h` as two separate writes
  through the sniff helper and assert state transitions to
  `in_alt_screen = true`.
* Unit test asserting `request_redraw` invokes resize with the
  wobble pattern (mock the PtyMaster's `resize`; assert it's
  called twice with the expected sizes in order).
* Keep the systacean-9 alt-screen-replay-suppression test green.

## Hardening

* The 50ms sleep happens on the controller thread; confirm it
  doesn't starve other sessions' broadcasts. The thread is
  per-session anyway in the current registry design, so it
  shouldn't, but check.
* If `pty_master.resize` fails (rare; usually only on a closed
  master), don't crash the registry. Best-effort; log and move on.

## Coordination

* @@Webtest A re-runs the htop / vim / less reload trio with
  side-by-side screenshots against fresh-launch baselines.
  Service is on PID 8248; they should rebuild + relaunch when
  this lane reaches REVIEW.
* No frontend change needed.

## Out of scope

* A real screen-model implementation (tmux's approach). Still
  the right long-term direction; not now.
* Cursor-shape / attribute preservation across reattach — those
  travel inside the alt-screen state via the prelude (if shipped)
  or via the program's own redraw.

## Progress

* 2026-05-17 @@Systacean: picked up after Webtest A's round-9
  failure report. Inspecting the current PTY session registry
  and replacing the per-chunk alt-screen sniff / no-op redraw
  with the requested rolling sniff and winsize wobble.
* 2026-05-17 @@Systacean: implemented the rolling alt-screen tail,
  attach prelude, and controller-thread winsize wobble. Local
  code gates are green; Webtest A owns the htop/vim/less visual
  re-smoke and screenshot diff.

## Completion notes

* `terminal_sessions::tests::alt_screen_sniffer_matches_sequences_across_chunks`
  covers split `\x1b[?1049h` / `\x1b[?1049l` detection through the
  rolling tail buffer.
* `terminal_sessions::tests::redraw_wobble_pattern_resizes_then_restores`
  covers the rows-1 -> original resize sequence used by the redraw
  command. `redraw_wobble_keeps_single_row_sessions_moving` pins the
  one-row edge case.
* Reattach in alt-screen mode now skips byte-ring replay, sends
  `\x1b[?1049h\x1b[2J\x1b[H` to the fresh WebSocket client, then
  requests the redraw wobble on the per-session controller thread.
* Local verification: `cargo fmt --check`, `cargo test -p chan-server
  terminal_sessions`, `cargo clippy -p chan-server --all-targets --
  -D warnings`, `cargo build --no-default-features`, `cargo clippy
  --all-targets -- -D warnings`, `cargo test`, `npm --prefix web run
  check`, `npm --prefix web test -- --run`, and `npm --prefix web run
  build`.
* Webtest A still needs to capture the htop / vim / less screenshot
  diffs against a rebuilt service.
