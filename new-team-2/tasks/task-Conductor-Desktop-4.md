# task-Conductor-Desktop-4 — item 6 (launcher Open) + desktop-lane backlog

From: @@Conductor. To: @@Desktop. Cut: 2026-06-12.

## Scope, in order

1. Item 6 — launcher Open: always enabled, auto-turn-on, failure
   dialog with reason. desktop/src/main.js + styles.css ONLY; no
   Rust changes needed. Design (read fully):
   new-team-2/designs/item-6-launcher-open-auto-on.md.
   Line numbers from main @ 3ebee587 — verify before editing.
2. B3 — default.json negative pin for read_dropped_paths (tiny;
   phase-23 carryover).
3. CHECK IN with me (task back) BEFORE starting B5/B6/B4 — they are
   phase-22 carryover with no design doc in this round's designs/.
   Recover context from docs/phases/ (phase-22/23) + the new-team-1
   bus, write a short per-item note, and get my ack first.
   - B5: buried-window memory visibility.
   - B6: GTK set_menu in-place mutation check via sdme.
   - B4: Linux drop path-print investigation (likely documented
     no-op).

## Standing lane duty (priority over B5/B6/B4)

You own desktop builds for the team's WKWebView verification —
items 1, 2, 4, 6 all gate on WKWebView — and the final smoke DMG.
Build requests route through me; expect them mid-round. Fresh-binary
provenance check before any re-walk of a previously-failed test.

## Gate

- Item 6 is manual + review (no launcher test harness exists): walk
  the design's verification list — happy path (off → Open → on +
  window), failure path (hold the flock via a second `chan serve` on
  that workspace → Open and, separately, the pill → dialog with the
  in-use reason; dismiss via OK / Escape / backdrop; pill consistent
  after), toggle-off unchanged, remote rows unaffected, real desktop
  build (`make desktop-dev` or bundle).
- A pure-function extraction + small node test is welcome, not
  required. Don't copy the keydown-listener leak from the existing
  dialogs (use { once: true } or explicit removal — it's in the
  design).
- Commits pathspec-atomic: `git commit -F <msg-file> -- <paths>`;
  staged-stat before, show-stat after.

## Review pairing

Your launcher JS → adversarial review by @@Editor (I route).

## Completion

ONE completion poke after item 6 + B3, with
new-team-2/tasks/task-Desktop-Conductor-<n>.md: shas, the full
verification walk evidence, screenshots if cheap, B5/B6/B4 context
notes (or a plan to gather them). Journal:
journals/journal-Desktop.md, append-only.
