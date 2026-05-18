# webtest-b-4: Round 2 wave-A walkthrough lane (Lane B)

Owner: @@WebtestB
Cut by: @@Architect
Date: 2026-05-18

## Goal

Walk through Round 2 wave-A from the backend / terminal /
end-to-end angle. Lane B covers: the watcher → PTY poke
path, the watcher lifecycle vs terminal lifecycle, and
multi-tab dispatch correctness.

Rolling task; append verdicts as each piece lands.

## Relevant links

* Backend: [../systacean/systacean-9.md](../systacean/systacean-9.md).
* Frontend: [../fullstack/fullstack-13.md](../fullstack/fullstack-13.md).
* Schema: [../architect/journal.md](../architect/journal.md)
  ("2026-05-18 21:00 BST" entry).

## Acceptance criteria

For each item, report PASS / FAIL / PARTIAL.

### When `systacean-9` lands

1. **Watcher lifecycle vs terminal close**:
   * Attach watcher to a dir; close the terminal tab.
   * Drop a synthetic event into the watched dir.
   * Verify no dispatch occurs (watcher dropped with the
     terminal as spec'd).
2. **Watcher replacement**:
   * Attach watcher to dir A; then call POST again with
     dir B.
   * Drop events into both; verify only dir B events
     dispatch.
3. **Multi-tab dispatch**:
   * Spin up two terminals with different `@@names`
     (via `chan open` env or rename UX). Attach watcher
     to one of them, watching a shared dir.
   * Drop events with `to: @@Tab1` and `to: @@Tab2`;
     verify the right tab gets `poke\n` each time.
4. **No self-loop**:
   * Try to make chan-server respond to a watched event
     by writing into the watched dir (shouldn't happen
     by construction; verify there's no infinite loop
     under stress).
5. **PTY input format**: confirm the dispatched poke is
   literally `poke\n` — not extra whitespace, not a
   different sequence.

### When `fullstack-13` lands

6. End-to-end happy path:
   * Open rich prompt in terminal A.
   * Set watcher on dir X.
   * From terminal B (a different tab), atomic-write a
     survey event targeting @@TerminalA into dir X.
   * Verify terminal A's tab gets `poke\n`, opens the
     bubble overlay with the survey rendered correctly.
7. Reply path:
   * From terminal A, pick an option + scope, submit.
   * Verify reply JSON lands in dir X with correct
     schema (`type: survey-reply`, `id` matches the
     original).

### Carry-over verdicts

8. Re-confirm `systacean-7` (DMG build) on current main
   by running `make -C desktop build` and confirming
   the DMG artifact lands.
9. Re-confirm `systacean-8` (B19 scrollback retention):
   reload the browser on an active PTY session, verify
   prior xterm scrollback re-appears.

## How to start

* Bring up a fresh `chan serve` on 8810 against a
  throwaway drive.
* For synthetic events, use the same recipe as
  `webtest-a-6` (mkdir + atomic mv).
* Permission scope carried over.

## Hand-off

Ping after each cluster via
`alex/event-webtest-b-architect.md`.
