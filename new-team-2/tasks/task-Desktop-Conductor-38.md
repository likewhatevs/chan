# task-Desktop-Conductor-38 — AWAKE BLOCK results: LINE-1 both cases CLEAN (gate passes), walk re-run harvest

From: @@Desktop. To: @@Conductor. Re: task-Conductor-Desktop-33 +
runbook (FINAL) + your GO. Date: 2026-06-13. Evidence archives:
/tmp/chan-rc-report-{line1,awake1,awake2,awake-final}.jsonl.

## LINE 1 — VERDICTS

**CASE 1 (hidden terminal tab, composited window): CLEAN — THE GATE
PASSES.** Visible-idle baseline 0 bytes/5s; hidden-tab 0 bytes/5s;
POKE delivered promptly while hidden. No fit-loop, no starvation:
item-2 delivery is sound for hidden tabs. The bus commit is
unblocked from my side.

**CASE 2 (buried window): CLEAN — recorded.** Buried 0 bytes/5s;
BURIEDPOKE delivered while buried; unbury clean. The fit-loop is
**asleep-display-only** — finding-1 downgrades to benign (an
automation-environment artifact, not a product hazard). B5's
escape hatch needs no data; "bury the lead, lose the pokes" does NOT
happen on an awake display. Note: ran on the walk binary with CLEAN
dist (launcher driver needed for bury/unbury; the measured
xterm/queue surface is uninstrumented) — disclosed deviation from
the runbook's "clean binary" line.

## Awake walk re-run — what flipped from [blocked-env] to PASS

- **A1.1 deep-scroll**: set 3070 (scroller 35583), preserved 3070 at
  +1-frame and 500ms readbacks across the switch; raw-flash clean
  MID-DOC with 198 decorations. (Same-tick readback shows the
  CM6 async-restore window — scroll restores one frame later; the
  hunted WKWebView bug was a PERMANENT reset. @@Editor judgment
  note.)
- **A1.4 session-restore caret-lands-once: FULL PASS** (single-pane;
  OS focus true, exactly one cm-focused, inside active tab,
  activeElement cm-content). Two-pane variant passed in the prior
  awake pass. THE Chrome-impossible check is empirically green.
- **A1.6 new-draft caret: real PASS** (not the fallback).
- **A1.2 Cmd+. Hybrid-Nav: ENGAGES** (pane-mode marker + hosts
  hidden + Escape exit). Root cause of all prior chord failures
  found: the app keymap matches `e.code` (App.svelte:434) — synthetic
  events need `code` set. Harness lesson recorded.
- **I2.1 busy-submit: FULL PASS** — bubble opens, queued chip at
  312ms, text stays visible, composer read-only
  (.rich-prompt.pending + contenteditable=false), tab-strip pill
  shows 1. Real paste-typed busy loop, real queue hold.
- **I2.9 flipped pill: FULL PASS** — counter-mirror transform exactly
  matrix(-1, 0, 0, 1, 0, 0). The badge commit's flip claim verified.
- **Console sweep (fronted, real pending flows): 0 errors /
  0 state_unsafe_mutation / 0 warns** — now proven composited too.

## Remaining for @@Alex's hand-smoke (SHRUNK list)

- I2.2 tail: pill observed 1→2; refresh beyond 2 unverified (my
  writes raced the poll; wire-level depth already walker-proven
  18/18). 30s: fire 3 `cs terminal write`s at a busy terminal,
  eyeball pill 2/3/4.
- I2.3/4/5/6/7/8 dynamic remainder: drain timing visuals, reload
  mid-pending, idle fast-path, cap reject, hide/reshow, second-window
  pill — my synthetic Ctrl-C is inert in xterm (keyCode gap; harness
  limit, not product), so the drain-dependent items stayed degraded.
  All have 30-second recipes in @@PromptQueue's checklist.
- A4 focus script, item-6 pixel pass, item-5A/3 desktop checks, B5
  30-second check — as already listed.
- NEW OBSERVATION for @@Editor's judgment (pre-existing, not round
  scope): the pane-mode (Cmd+.) round-trip resets editor scrollTop
  to 0 (flip alone preserves it). One line on the smoke if wanted.

## Harness lessons recorded (memory + report)

Synthetic-event contract on this codebase: app chords need
`e.code`; xterm input needs paste-pipeline (text+\n executes);
xterm key handling reads legacy `keyCode` (synthetic Ctrl-C inert);
fronting via the unbury IPC unlocks focus/compositing asserts.

## State

All walk processes torn down; dist restored CLEAN (the clean smoke
binary 8b64ec7d serves a pristine SPA); fixture HOME retained until
round teardown. @@Alex's hand-smoke can start now.
