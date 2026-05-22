# webtest-b-4 — -b-25 runtime walkthrough (chan-desktop orphan-detect heuristic + reclaim-dialog PID display)

Owner: @@WebtestB
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Runtime walk of `fullstack-b-25` (`f29611c`) — the
tightened orphan-detect heuristic + the new custom
reclaim-dialog with candidate-PID display.

## Reference

* Task body: [`../fullstack-b/fullstack-b-25.md`](../fullstack-b/fullstack-b-25.md)
  + the @@FullStackB commit-readiness tail.
* Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md)
  "chan-desktop orphan-detection heuristic too loose".
* Prior walk: `webtest-b-3` (the `-b-22` walkthrough
  that surfaced both pieces).

## Acceptance

### Heuristic tightening (live runtime)

1. **Real-orphan case**: spawn `chan serve <drive-key>`
   in a terminal; launch chan-desktop against the same
   drive; observe the heuristic identifies the running
   `chan serve` as a candidate.
2. **False-positive avoided**: launch a wrapper like
   `bash -c "tail -f /tmp/junk.log <drive-key>"` (where
   `<drive-key>` happens to match); confirm chan-desktop's
   heuristic does NOT flag it as a candidate. (You can
   inspect via the new `find_drive_lock_candidates` IPC if
   needed.)
3. **Wrapper rejected**: `bash -c "chan serve <drive-key>"`
   — confirm the wrapped chan serve is rejected per the
   positional-argv check (argv[0] basename ≠ `chan`).

### Reclaim dialog PID display (live runtime)

4. **Dialog renders PID + command**: trigger the
   reclaim flow; dialog shows each candidate's PID +
   command-line.
5. **Cancel works**: click Cancel; no processes touched;
   chan-desktop stays unable to acquire the lock.
6. **Reclaim works**: click Reclaim; candidates
   SIGTERM'd; chan-desktop acquires the lock; toast
   surfaces.
7. **Multi-candidate case**: if you can stage TWO
   orphan `chan serve` processes against the same
   drive, confirm both show in the dialog.

### Cancel UX

8. Backdrop click + Escape cancel the dialog (per the
   @@FullStackB tail: "Backdrop click + Escape cancel").
9. Reclaim button has initial focus + Enter triggers it.

### Walkthrough audit trail

Append a fresh dated heading to
[`webtest-b-1.md`](webtest-b-1.md):
`## 2026-05-22 — fullstack-b-25 runtime walkthrough
(heuristic + reclaim dialog)`. Capture verdicts +
screenshots of the new dialog + tear-down evidence.

## How to start

1. `git status` clean; `git log --oneline -5` confirms
   `f29611c` in HEAD.
2. Rebuild chan-desktop (Cargo + bundle as needed).
3. Set up the orphan test conditions per the
   acceptance checks (you have standing perm for
   throwaway drives + chan-desktop runtime).
4. Walk checks 1-9.
5. Append verdict; fire poke to
   `event-webtest-b-architect.md`.
6. Tear down per the standing rule.

## Coordination

* @@WebtestB lane.
* Standing chan-desktop runtime permission covers this
  walk on throwaway drives.
* By-PID SIGTERM only for the staged orphan processes;
  no `pkill -f`.
* Medium walk; ~30-45 min (multiple staged conditions).

## Numbering

Highest committed `webtest-b-N` is `-3` (the `-b-22`
walk). This is `-4`.

## Out of scope

* Re-walking the parts of `-b-22` already verified.
* Server-side lock primitive (Round-3 polish; separate
  bug-list entry).
* Re-architecting the lock-takeover protocol.
