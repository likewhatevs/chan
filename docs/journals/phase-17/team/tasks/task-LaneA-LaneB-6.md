# task-LaneA-LaneB-6: R2-3 - per-terminal survey

From: @@LaneA  To: @@LaneB  Wave: round-2

Round-1 is committed + PUSHED (origin/main 03bb91f8). Round-2 now.

## @@Alex's ask (round-2/draft.md + image-3)

"survey must be per terminal, not window-wide ... Each terminal could have their
own survey and they should not impact each other."

VIEW docs/journals/phase-17/round-2/image-3.png first.

## Scope

Today a survey raised over a terminal renders window-wide (BubbleOverlay at the
App root, z:39000 - you saw this in B1's z-order work). Make surveys
PER-TERMINAL: each terminal tab owns its own survey state + overlay, independent
of the others, exactly the pattern you used for B1's rich prompt (visibility +
state keyed by tab id, not a window-global flag). Two terminals can each show
their own survey without colliding; answering/dismissing one does not touch the
other.

Files: web/src/components/BubbleOverlay.svelte (yours) + wherever the survey
state lives (the open_survey window-command handler / the survey store). If the
survey state is in a file outside your owned list, STOP and route to @@LaneA
(like the B4/store.svelte.ts case) - I'll authorize if it's clearly the survey
plumbing.

NB: this also improves the `cs terminal survey` channel the lead uses to ask
@@Alex - a per-terminal survey targets exactly the terminal it was raised on.

## Gate

- make web-check + svelte-check + npm run build.
- Browser-smoke (Svelte-5 reactivity is runtime-only): raise a survey on two
  different terminals (e.g. `cs terminal survey --tab-name=<each>`); each shows
  its OWN survey; answering one leaves the other intact. rust-embed: npm build
  before cargo build; smoke the served bundle.

## Report

Cut task-LaneB-LaneA-N (repro + fix + own-gate-green + pathspec shas) + poke.
This is the last round-2 item on your side.
