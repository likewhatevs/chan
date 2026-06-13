# Awake-block runbook — finding-1 repro (LINE 1) + walk re-run + clean smoke

Author: @@Desktop. For: @@Conductor's sequencing once @@Alex is at
the machine (display awake + unlocked). Re: task-Desktop-Conductor-36
(+ Editor co-sign task-Editor-Desktop-36), task-Conductor-Desktop-33.

## Artifacts (all staged, nothing to build)

| artifact | path | sha256[..16] |
|---|---|---|
| CLEAN smoke binary (for @@Alex's hand-smoke) | /tmp/chan-desktop-gate-target/debug/chan-desktop | 8b64ec7d33dd78c9 |
| WALK binary (instrumented: launcher driver + debug IPCs + throttling) | /tmp/chan-rc-walk-bin/chan-desktop | b2ab624b47fe2f22 |
| SPA driver master | /tmp/chan-rc-walk-bin/rc-spa-driver.js | (runtime-injected, below) |
| report server | /tmp/chan-rc-report-server.py | — |
| isolated HOME + fixture ws | /private/tmp/chan-rc-home, /private/tmp/chan-rc-ws | — |

Provenance notes: both binaries from worktree b82a0a27 (now pristine,
`git -C /tmp/chan-desktop-gate status` clean). Clean binary verified
marker-free (0 hits for the embedded launcher-driver tag and the
debug-IPC strings). Walk-binary sha differs from the report's
first-build 5d7d5b0f — it was relinked during cycles 2-4 (incremental
touch-rebuilds); b2ab624b IS the binary cycles 2-4 ran on.
web/dist is currently CLEAN (clean binary serves it as-is; the walk
binary reads it from disk at runtime, so re-instrument before a walk
run, restore after — steps below).

## LINE 1 — finding-1 repro, TWO CASES per @@Conductor's disposition
## (accepted ruling on the joint observation; supersedes the earlier
## single-sequence framing)

Question: does the hidden-terminal fit-loop (continuous SIGWINCH →
prompt redraw → write-queue idle gate held closed) reproduce when
the display is AWAKE? Joint observation @@Desktop+@@Editor.

Setup, shared by both cases (CLEAN binary + isolated HOME; no
instrumentation — measurement is cs scrollback growth):

1. `HOME=/private/tmp/chan-rc-home /tmp/chan-desktop-gate-target/debug/chan-desktop &`
2. In the launcher: turn chan-rc-ws ON, Open a window; in the SPA
   open a terminal tab (or `cs terminal new` with
   CHAN_CONTROL_SOCKET=$TMPDIR/chan-control-<newpid>-*.sock and
   CHAN_WINDOW_ID from `cs window list`).
3. Baseline (terminal tab VISIBLE, shell idle):
   `R1=$(cs terminal scrollback --tab-name=Terminal-1 | wc -c); sleep 5; R2=$(...wc -c)`
   — expect growth ~0.

### CASE 1 — hidden terminal TAB, composited window
### [REQUIRED — GATES THE BUS COMMIT, ~2 min]

4. Switch to a FILE tab in the same (visible, composited) window —
   the terminal hides via item-1 keep-alive. Repeat the 5s growth
   measurement.
5. While the tab is hidden:
   `cs terminal write --tab-name=Terminal-1 $'echo POKE\n'` —
   does POKE deliver within ~2s (watch scrollback)?

VERDICT CASE 1: growth ≈ 0 AND POKE delivers → hidden-TAB delivery
is sound; the round's bar holds; proceed to CASE 2. Growth ≫ 0 OR
POKE starves → **item-2 delivery is broken for hidden tabs = this
round's bar; STOP, task to @@Conductor before any bus commit.**

### CASE 2 — BURIED window
### [RECORDED CHECK ONLY — does NOT gate the commit; ~5 min; fix
### (if it reproduces) is NEXT-ROUND; pre-existing behavior class,
### B5-priced]

6. Re-show the terminal tab, confirm idle again, then bury the whole
   window (red-dot close). Repeat the 5s growth measurement and the
   POKE-delivery probe while buried; unbury and confirm any starved
   poke arrives on unbury.

RECORD CASE 2 either way: reproduces → the buried-list-cap escape
hatch in the B5 decision note gets data (feeds report + follow-ups,
data not speculation); clean → the fit-loop is asleep-display-only,
note as benign. The live survey is NOT amended in either outcome
(starvation exists under both cap semantics — @@Conductor's ruling).

## Walk re-run (the [blocked-env]/[degraded] lines, ~5 min)

1. Re-instrument dist (walk binary reads it live):
   `cp /tmp/chan-rc-walk-bin/rc-spa-driver.js /tmp/chan-desktop-gate/web/dist/rc-spa-driver-v2.js`
   and add `<script src="rc-spa-driver-v2.js"></script>` before
   `</body>` in /tmp/chan-desktop-gate/web/dist/index.html.
2. Reset session state:
   `rm -rf /private/tmp/chan-rc-home/Library/{Caches,WebKit} "/private/tmp/chan-rc-home/Library/Application Support/Chan Desktop/config.json" /private/tmp/chan-rc-home/.chan/workspaces`
3. `rm -f /tmp/chan-rc-report.jsonl; echo '{}' > /tmp/chan-rc-state; python3 /tmp/chan-rc-report-server.py &`
4. `HOME=/private/tmp/chan-rc-home /tmp/chan-rc-walk-bin/chan-desktop &`
5. Drive via /tmp/chan-rc-state exactly as the archived session
   (launcher seq: turn-on, open-window; ack claim-main/setup-tabs
   after `cs terminal new` + `cs open walk-doc-{a,b}.md`; remaining
   acks on demand). @@Desktop drives if at the keyboard;
   the need/ack trail is self-describing in /tmp/chan-rc-report.jsonl.
6. Lines that now bind: A1.1 deep-scroll, A1.4/A1.4b/A1.6 caret
   (hasFocus true), Cmd+. engagement re-check, full I2 dynamic block
   (bubble chord on window-target + paste-typed busy loop are already
   in the staged driver).
7. Restore after: delete the two dist edits from step 1.

## Clean smoke (after walk re-run, or independently)

CLEAN binary + @@Alex's hand-smoke list (TeamFlow checklist § 5
hand-smoke lines + @@Editor's A4 30-second script + item-6 pixel
pass + B5 30-second check). Real-HOME run is fine for the smoke —
the clean binary carries no instrumentation; suggest his normal
workflow rather than the fixture workspace.

## Teardown after the block

kill walk processes; `rm -rf /tmp/chan-rc-* /private/tmp/chan-rc-*
/tmp/rc-bin /tmp/chan-rc-walk-bin`; worktree + target dir stay until
round teardown (they are the build base). b6gtk container: remove at
round close (`sdme rm b6gtk`, fs `chan-desktop-ubuntu` stays).
