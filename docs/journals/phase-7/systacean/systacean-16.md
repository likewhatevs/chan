# systacean-16: tune activity counter — don't count transient redraws

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-19

## Goal

`bytes_since_focus` (added in `systacean-13`, paired with
`fullstack-25`'s SPA fix) fires the tab-strip activity
dot too eagerly. @@WebtestA observed it firing on a tab
they didn't intentionally write to — cursor blink /
prompt redraw / ANSI state updates seem to count as
"activity" even though there's no real new output for
the user to read.

Tune the counter so transient terminal-control sequences
don't trip it. Real output (text, command results) still
counts.

## Relevant links

* @@WebtestA's side observation in `webtest-a-7` final
  tally (post-fullstack-25 re-test).
* Predecessor: [./systacean-13.md](./systacean-13.md)
  + commit `1694041`.

## Acceptance criteria

* A tab in idle prompt state (just `$ ` with cursor
  blink) does NOT accumulate activity bytes. Tab strip
  marker stays clear.
* A tab that receives a real command output (e.g.
  `echo hello\n`) accumulates activity bytes. Marker
  appears.
* @@WebtestA's specific repro (idle FgTerm picked up
  a transient activity dot during a session change)
  goes away.
* Test coverage:
  * ANSI cursor-position / SGR-only writes don't
    increment.
  * Plain text writes (any visible char output) do
    increment.
  * Mixed (ANSI + visible text) increments by the
    visible-text portion only, or treats the whole
    write as activity if any printable character is
    present — your call which heuristic, document it.

## Likely heuristic

Two reasonable shapes:

1. **Strip CSI / SGR sequences before counting.** If
   the remaining bytes are all whitespace + cursor
   chars, don't increment. If any printable non-
   whitespace remains, increment by the printable
   count.
2. **Detect "interesting" writes structurally.** Treat
   writes that contain newlines or printable text
   chunks as activity, ignore pure-ANSI updates.

Option 1 is more precise; option 2 is simpler.
Document the chosen heuristic in code comments on the
counter path so future-us doesn't relitigate.

## Out of scope

* Sound or other indicator surfaces.
* Activity on non-terminal tabs.
* User-configurable sensitivity. Default behavior
  only.

## How to start

1. Locate `bytes_since_focus` increment path in
   `crates/chan-server/src/...` (terminal sessions /
   PTY broker).
2. Apply the chosen heuristic before incrementing.
3. Add the tests + comment.

## Hand-off

Standard. Pre-push gate green. @@WebtestA re-tests
after landing. Ping via
`alex/event-systacean-architect.md`.
