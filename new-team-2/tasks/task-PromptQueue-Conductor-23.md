# task-PromptQueue-Conductor-23 — item 2 COMPLETE (badge held), evidence inside

From: @@PromptQueue. To: @@Conductor. Cut: 2026-06-12.
Completion for task-Conductor-PromptQueue-2 (Rich Prompt queue
visibility, end-to-end minus the HELD Pane badge).

## Shas

- ca40ea6b — feat(chan-server): server half (QueuedWrite, enqueue_prompt
  all-or-nothing, QueueDepth/PromptDelivered events, prompt-ack/
  prompt-delivered/queue frames, session.queue_depth). Milestone-poked
  earlier; review routed to @@CtxPass (your task-13).
- 86d50a25 — feat(web): web half (tabs store pendingPrompt/queueDepth +
  setters, TerminalTab frame arms + onclose/closed/exit handling,
  RichPrompt pending state machine: read-only compartment, 300ms chip
  grace, 5s ack timeout, delivered-clears-draft, honest reject/fail
  labels, idle teammate-depth label). Review routed to @@TeamFlow.
- Pane.svelte: untouched, per the hold. The badge edit + its
  richPromptTerminalWiring badge pin are ready to go the moment you
  clear it — small, isolated, ~30min including gate re-run.

## Gates (re-run after final edit, real flags)

- Rust: cargo fmt --check 0; RUSTFLAGS="-D warnings" clippy
  --all-targets 0; RUSTFLAGS="-D warnings" cargo test -p chan-server
  424/424. New tests: all-or-nothing-at-cap, depth-counts-messages,
  delivered-on-last-write-only (event order), depth-broadcast-both-
  paths, serde wire pins (prompt-ack both arms, prompt-delivered,
  queue, session+queue_depth), Prompt decode pin extended with id.
- Web: svelte-check 0 errors; make web-check vitest 176 files / 1743
  tests; production build green. New: state/promptQueue.test.ts (store
  transitions incl. stale-id guard), updated + extended source pins in
  richPromptComponent / richPromptTerminalWiring tests.

## Manual recipe — executed at the WIRE level, ALL 18 CHECKS PASS

Chrome could not reach the throwaway server (see Blocked below), so I
drove the recipe over the real terminal WS with a Node walker against
a throwaway `chan serve --standalone` (port 8923, fresh binary built
after `make web-check`, bundle grep-verified for the new frames).
Evidence: new-team-2/evidence/item-2/ (walker + PASS transcript +
the earlier walker-bug run for honesty). Highlights:

1. Busy agent (date/0.3s loop) holds delivery; tagged gemini submit
   acks {queued:true, depth:1}; nothing delivered while flooding.
2. cs terminal write ×3 behind the pair: CLI reports RAW positions
   3/4/5 byte-for-byte while queue frames read MESSAGE depths 2/3/4 —
   the documented divergence, live.
3. Second socket attached mid-queue: its session frame re-syncs
   queue_depth 4. Observer also receives prompt-delivered (foreign id,
   reads depth).
4. Ctrl-C → drains in order; gemini BODY drain emits nothing;
   the CHORD drain emits prompt-delivered{depth:3} then queue{3} —
   delivered-first ordering asserted by frame index.
5. Untagged pokes drain 3→0 with queue frames only (exactly 1
   prompt-delivered in the whole phase); all queued commands executed
   in order on the PTY.
6. Idle fast path: ack+delivered in 946ms (< the SPA's 1s expectation;
   the 300ms chip grace covers it).
7. Cap: 49 pairs + 1 poke = raw 99; a 2-write pair REJECTS
   all-or-nothing (ack {queued:false, depth:50} unchanged); a 1-write
   poke fits slot 100; next CLI write → "matched session(s) at the
   100-write queue cap; nothing queued". Cap regression: strings AND
   semantics byte-for-byte (control_socket.rs untouched by my diff).

## BLOCKED (routing to you) + WKWebView-pending

- Chrome browser smoke of the SPA state machine: the claude-in-chrome
  permission gate denied http://localhost:8923 (and 127.0.0.1) three
  times — the origin isn't allowlisted in the shared Chrome. Wire +
  vitest + svelte-check cover everything except live Svelte runtime
  reactivity (state_unsafe_mutation-class). Options: (a) fold into
  @@Desktop's WKWebView pass — already the round's real gate for
  item 2; (b) @@Alex allowlists a localhost port for me; (c) I pair on
  an already-allowlisted origin if one exists. Recommend (a) +
  checklist below.
- WKWebView checklist for @@Desktop's build: submit over busy agent →
  text stays/dims/read-only, chip after ~300ms; cs write ×3 → idle
  label "N queued"; drain → prompt clears EXACTLY when its message
  prints; reload mid-pending → draft text restored, label recovers,
  badge re-syncs; idle submit → no chip flash; second window sees
  depth changes. (Tab-strip badge only after the held Pane edit.)

## Notes

- Pre-existing bug fixed in passing (per design): at raw 99 a gemini
  submit used to enqueue the body and silently DROP the CR.
- cs CLI prints control responses on stderr (pre-existing; noticed
  while building the walker — not a regression, strings unchanged).
- journal: journals/journal-PromptQueue.md up to date.
