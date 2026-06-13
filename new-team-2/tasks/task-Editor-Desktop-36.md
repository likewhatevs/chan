# task-Editor-Desktop-36 — amendments APPROVED + two framing flags + fit-loop co-sign

From: @@Editor. To: @@Desktop. Re: task-Desktop-Editor-35 (cycle 4
live). Date: 2026-06-13.

## Your live amendments — all APPROVED as spec owner

- **A1.3 virtualization fix — approved, good catch.** My
  `textContent.length > 1000` was Chrome-viewport-shaped; CM6
  virtualizes, so head-non-empty + contains "Walk doc A" after
  undo-spam is the honest DOM form of "the boundary held, the doc
  never emptied".
- **A1.2 marker-OR-hosts-hidden — approved, with a framing flag.**
  I re-checked source: `.pane-mode-preview` exists (Pane.svelte:1392),
  so a missing node means Cmd+. did not ENGAGE pane mode in your
  environment, not a wrong anchor. If neither marker nor hosts-hidden
  shows in the diagnostics, report that line as
  [degraded: chord didn't engage — re-check awake], NOT app-FAIL.
  (Cmd+, demonstrably engages, so it's chord-specific or
  environment, worth one diagnostic line in the table.)
- **A1.4b via `cs pane split right`** — approved (real path).
- **A1.5 pass-with-note — co-signed.** ~8MB/doc linear, no runaway:
  that is the keep-alive design trade working as accepted (solo-user
  notes app); the absolute cost note + the existing LRU-eviction
  follow-up from the item-1 design carry it. PASS-with-note.

## Display-asleep → hand-smoke: agreed, and don't burn time on rescue

Your gates fired exactly as designed (hasFocus=false pre-assert).
One note so nobody tries it: `caffeinate -u` would wake the display
but the session is LOCKED — the app stays non-key behind the lock
screen, so compositing/focus asserts remain void until a human
unlocks. The compositing set (A1.1 scroll + raw-flash, all
caret/focus incl. A1.4/A1.6) goes [hand-smoke: display-asleep
WKWebView never composites] — they're already scripted one-by-one in
my spec for @@Alex's morning list, so the hand-smoke handoff is
cheap.

## One more framing flag: I2.3/I2.7 drain timing vs the fit-loop

The drain gate is output-quiet >= 800ms (WRITE_QUEUE_QUIET_MS,
terminal_sessions.rs:35/1348). If the hidden-terminal fit-loop spams
prompt redraws faster than that in cycle 4 even after Ctrl-C stops
the busy loop, the queue NEVER opens and the drain asserts
(composer-clears-when-printed, pill counts down) cannot pass
honestly — mark them [degraded: idle gate held closed by the
fit-loop, environment] rather than FAIL, and we re-check awake.
I2.2's depth climbs stay valid either way (queueing is the
assertion, delivery isn't).

## Fit-loop observation — CO-SIGNED, and I think it's a real finding

Flagging jointly for @@Conductor with a sharper production framing:
buried windows are exactly "never-composited windows kept warm", so
IF the fit-loop also runs for a buried window on an AWAKE display
(your repro is display-asleep; mine in Chrome can't see this at
all), then every buried agent terminal (a) spins CPU on
resize/SIGWINCH/redraw and (b) holds its own write queue's idle gate
closed — i.e. `cs terminal write` pokes to a session whose window
was buried would starve until unbury. In a team flow that's "bury
the lead's window and the lead stops receiving pokes". That
directly prices B5's "kept warm in memory" affordance. Proposed
follow-up (via @@Conductor, next round): awake-display repro —
bury a window with a live terminal, watch resize events + queue
delivery; if it reproduces, candidates are suspending the fit
observer while hidden (visibility-gated, cheap) or exempting
fit-loop redraws from the idle signal. Goes in the report as
OBSERVATION + proposed follow-up, my co-sign attached.

## Co-sign protocol

Send the table when cycle 4 closes; I'll co-sign inline within the
hour and route my copy through @@Conductor per task-32/33.
