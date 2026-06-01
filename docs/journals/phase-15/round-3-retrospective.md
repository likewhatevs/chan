# Phase-15 round-3 retrospective

Author: @@LaneA / @@Architect. Written at round close, on the v0.22.0 cut.
Honest + constructive per the round-close norm: done/pending, highlights,
lowlights/contention, and feedback for the workers, @@Host, and the architect.

## Outcome

v0.22.0 cut from the `release:` bump commit. All six backlog themes addressed;
3 of 4 lanes verification-complete; the one session-crashing bug (RELOAD-HANG)
cured in Wave 1. Two editor browser-smokes ship empirically-unverified (a
permissions blocker, not a quality gap) under the pre-release norm.

## Done (by theme)

- Theme 1 - Team Work in-workspace + Survey rebuild (@@LaneC/@@LaneD): team
  config moved into the workspace; survey rebuilt for real (overlay + reply
  round-trip + [F] followup). Merged 08d7435b. Browser-verified end-to-end
  (C+D Wave-3): option click, keyboard, [F] file-create.
- Theme 2 - Desktop/CLI consolidation (@@LaneD): chan-shell crate (68a2adef);
  desktop `cs` argv0 dispatch + `chan open` removal (08d7435b); per-agent
  submit map. Desktop `cs` verified against the REAL desktop control socket.
- Theme 3 - Editor link/cursor UX (@@LaneB): relative-markdown links on disk
  (b273e0b5); heading/block links + image spaces + click-to-caret (9349dba2).
  Click-caret + [[ stuck-bubble smokes -> round-4 (navigate denied this round).
- Theme 4 - Search mentions/paths (@@LaneA): BM25 subtoken split so @@handle /
  path/to/file / file.md match (c854d3f8). PROBE surfaced that semantic is
  built but never queried -> round-4 product question.
- Theme 5 - IDX embeddings hardening (@@LaneA): preflight RELOAD-HANG fix
  (d1b7c427, the actual session-crash cure); chip-clobber fix via a decoupled
  bg-embed signal (41e7908e); in-flush freeze shortened (ba372dcb). Metal hang
  = round-4 follow-up (gate CHAN_ENABLE_GPU verified live).
- Theme 6 - Docs cleanup + graph hygiene (@@LaneB + @@LaneA): essence-only
  phase READMEs, raw dropped (a930a96f); backend ghost-node fix (beb0dc49) +
  EDGES-PK fix (ebee9a15). phase-8 raw deferred (cited from docs/agents).
- @@Host nits (@@LaneA): survey --help JSON examples (0b97944f); About-card
  embeddings row dropped (317074c6); types.ts ghost comment (fbe9bb90).

## Pending (round-4 backlog; see round-4-backlog.md)

- phase-8 docs cleanup completion (essence README + docs/agents citation
  repoint + raw deletion; @@Host-sequenced).
- The 2 editor browser-smokes, when navigate is re-allowed.
- Real AppImage `cs` re-exec verify (no AppImage build in this env).
- Metal hang investigation + GPU-by-default re-enable.
- Semantic-search-built-but-never-queried product decision.

## Highlights

- The architect-as-hub + 4-lane wave model held cleanly: disjoint file
  ownership, refresh handshakes at each barrier, and pathspec-only commits
  kept the shared worktree uncontaminated after the Wave-1 collision lesson.
- The RELOAD-HANG was root-caused, reproduced, and fixed in Wave 1; the chip
  work was correctly scoped as polish BEHIND that cure rather than mistaken
  for the crash itself.
- Confabulation discipline paid off twice: it caught a phantom-fix risk early
  in the round, and at close it caught a relayed "LaneB completed" that turned
  out to be a misattribution (B had never run the smokes).
- No-back-compat pragmatism: the EDGES-PK fix dropped a careful migration for
  a clean v1-schema change once @@Host confirmed a fresh ~/.chan is fine.

## Lowlights / contention

- The shared Chrome MCP tab group is a single-driver resource exactly like the
  worktree, but that was learned mid-round (C/D collided on it). Worse, browser
  `navigate` was denied to @@LaneB repeatedly and then to @@LaneA, so the 2
  editor smokes could not be browser-verified by ANY lane. They ship
  source-tested + gated-green but empirically-unverified.
- The Wave-1 commit collision (B's blanket stage swept D's uncommitted
  chan-shell) cost a recovery cycle before pathspec-only commits were ratified.
- Option B landed as the decoupled-signal form, not the full build_all
  task-split. A deliberate, @@Host-chosen risk-down call (the signal delivers
  "own status off the reindex contract"), but the "proper background job" as a
  separately-spawned task remains partial.

## Feedback

### For the workers

- @@LaneB: standout confabulation discipline. You caught your own greedy-regex
  data-loss in the docs cleanup BEFORE committing, and you reported graphData
  as already-satisfied (verified by reading) rather than fabricating a diff for
  appearances. That honesty is exactly right; keep it.
- @@LaneC: thorough, well-evidenced survey smoke and a clean, early escalation
  of the [F] followup-context seam (which the architect could then arbitrate
  cheaply while @@LaneD was still mid-flight). Good seam instinct.
- @@LaneD: solid transport build, and verifying desktop `cs` against the REAL
  desktop socket (not just a unit test) was the right rigor for a gate-blind
  cross-crate wire surface. The shared-Chrome lesson is yours and C's to carry.

### For @@Host (Alex)

- The decisive "proceed + clear the blocker" cadence kept the round moving
  without churn, and the no-migration clarification simplified the EDGES-PK
  fix well. One concrete friction worth fixing: the browser `navigate` denials
  (to B, then to me) blocked the only remaining verification this round. A
  standing browser-access decision at round start (who may navigate, on which
  ports) would avoid the late scramble.
- The misattribution relay is a useful data point: completion claims should
  travel as evidence (a journal entry, a commit), not as hearsay. The round's
  lean-poke-bus already assumes this; worth making it explicit for status
  relays too.

### For the architect (me)

- I let this session run very long. I did eventually checkpoint the one genuine
  Option-B risk/depth decision with @@Host (good), but I should have surfaced
  it sooner instead of deep-diving the code first.
- I oscillated on the browser-smoke server choice (B's :7843 vs my own) and on
  whether to chase the empirical clobber test; I should have decided faster and
  cut the empirical chase earlier once the unit test + the live boot
  observation were in hand.
- Coordination bookkeeping (status updates, pokes, the round-4 backlog) was
  rigorous and the pathspec discipline held, but some status edits could have
  been batched to reduce churn.
