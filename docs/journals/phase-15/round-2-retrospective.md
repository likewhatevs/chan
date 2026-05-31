# Phase-15 round-2 retrospective (v0.21.0)

Author: @@LaneA (@@Architect), 2026-05-31. Roles this round: @@LaneA =
architect; @@LaneB = Dashboard/frontend; @@LaneC = search/indexing; @@LaneD =
terminal/cs/desktop/Team-Work.

## Shipped in v0.21.0 (all merged, gated green)

- **Dashboard part-1:** A4 (Search-slot inspector actions), A3 (slot on/off menu
  + Settings + lock-out test reversal), A6 (license on the version row), A7
  (theme-reactive screensaver preview).
- **Bugs:** BUG-GRAPH (in-graph "Graph from here" -> filesystem mode),
  BUG-EDITOR (conceal re-decorate on geometryChanged; desktop-runtime unverified),
  SUBMIT (Shift+Enter LF fallback; real-agent verified), RELOAD (Ctrl+R ->
  Ctrl+Shift+R per-OS; desktop-runtime unverified), LINKS (clickable terminal
  URLs via openExternalUrl), **IDX** (the indexing wedge: preflight-on-BM25-ready
  + background embed + C-CAP 2000-file cap + the 4097/4096 display clamp).
- **cs CLI:** rename `cs term`->`cs terminal`, infer_subcommands prefix match,
  `cs terminal restart` (by-name server path), markdown `list`, `cs search`,
  `cs dashboard --carousel-off`.
- **Team Work:** TEAM-SELFSTART (lead launches its agent via the worker spawn
  path; real-agent verified), TEAM-GROUP (dialog field + persistence + -N
  conflict + server-side group join), POKE-2.2 (`cs terminal write --submit`).
- **Desktop:** DESKTOP-OPEN (`chan open` OS file-association; desktop-handoff
  branch desktop-only).
- cs-search `<b>`->`**` markdown polish; toast-auto-dismiss audit (no-op, invariant
  already held + guarded).

## Carryover (round-3-backlog.md)

DESKTOP-SHELL (cs-shell crate extraction, @@Host-deferred), survey bubbles 2.3
(@@Host-deferred), IDX Option B (embeddings as a proper background job) + the
bg-embed chip-clobber + the in-flush chip freeze (`EMBED_BATCH_CHUNKS` lever).
@@Host post-release desktop spot-checks: BUG-EDITOR, RELOAD, DESKTOP-OPEN handoff.

## Highlights

- **Cross-lane commit hygiene held.** The round-1 shared-worktree incident did
  NOT recur: every shared-file commit was guarded-atomic (chained add + staged
  audit + post-commit verify), and the genuinely-hard case - the combined
  TEAM-GROUP atomic commit (10 files across two lanes, required-field coupling) -
  landed perfectly, main never red. The cs-search cross-lane rescue likewise.
- **Smoke-first repeatedly caught wrong fixes.** TEAM-SELFSTART: the
  command+env-override fix (blessed) was never reached; the real bug was a
  reattach gap - only the real-agent smoke exposed it. IDX: a live `sample` of
  the wedged process (777/777 in candle BERT matmul) overturned several
  confabulated source-read theories at once.
- **IDX was the round's hardest, highest-impact bug, solved well.** Root-caused
  empirically (synchronous embed pass over a large workspace, not a lock/loop),
  fixed with the least-risk correct architecture (gate first-paint on BM25-ready,
  embeddings in background), + C-CAP + the chip + the display clamp.
- **No bad code landed despite severe tooling corruption.** @@LaneC's reads and
  bash stdout fabricated content mid-round; the discipline (anchor on
  git/sha/compiler-error ground truth, refuse to blind-commit) caught it every
  time. The build error itself reconciled a lane-vs-lane contradiction.
- **The round built and used its own coordination mechanism.** Directed-wake via
  the submit chord (CK-SUBMIT) was validated and then productized as
  `cs terminal write --submit` (POKE-2.2) - the round literally closed its own
  coordination gap.
- **Parallelization where it was safe.** Wave-3 split the TEAM-GROUP dialog to
  @@LaneB while @@LaneD did the orchestrator; the respawned @@LaneC ran
  collision-free empirical QA walks of the cs/IDX/Team-Work surface.

## Lowlights

- **Tooling flakiness was the dominant drag.** Output truncation + confabulation
  caused two @@LaneC confabulation incidents, a tab recycle, and several stalls;
  it consumed real coordination effort that had nothing to do with the work.
- **Poke-delivery stacking burdened @@Host.** Lanes on the pre-CK-SUBMIT `\r`
  recipe sent pokes that stacked un-submitted in my terminal, so @@Host
  hand-Entered them - the cause of the "waiting on you" stalls. Fixed (in
  v0.21.0) by `--submit`, but it cost friction all round.
- **The cs-search saga.** A duplicate client/server ControlRequest enum + a
  lane-vs-lane contradiction (@@LaneC's tooling down but diagnosis right;
  @@LaneD's tooling fine but diagnosis wrong) took several reconciliation turns.
- **Multi-agent core contention.** 4+ agents + concurrent cargo builds on 8 cores
  produced the in-flush chip freeze and slow/locked builds.

## Honest feedback

- **@@LaneB:** clean, disciplined execution - part-1 + the two frontend bugs +
  the IDX frontend pairing + a zero-regression coverage walk, and a textbook
  combined-atomic-commit. The only blemish was the `\r` poke recipe (bootstrapped
  before CK-SUBMIT), a minor process miss.
- **@@LaneC:** the standout. Solved the hardest bug end-to-end, held the line
  under brutal tooling corruption (refused to commit confabulated work - exactly
  right), made the sharp diagnoses (the live sample; the duplicate client enum
  that nobody else spotted), and delivered a thorough QA recovery after the tab
  reset. A model of discipline under bad conditions.
- **@@LaneD:** carried the long pole - terminal bugs + the entire cs CLI + Team
  Work + desktop - with consistent guarded-atomic commits, smoke-first that
  caught wrong fixes, and sharp scope reads (the DESKTOP-SHELL defer was the
  right call, well argued). The most prolific lane; ran clean.
- **@@Host (Alex):** good, decisive product calls (the preflight-ASAP/background
  principle, the cap threshold, the DESKTOP-SHELL defer), and the manual-Enter
  relaying was a genuine backstop for a real tooling limitation. Constructive:
  the round was very large (IDX alone was arguably a round's worth); a tighter
  scope per round would reduce the surface where the flakiness compounds.
- **Architect (me):** the decomposition, cross-lane sequencing, and the
  verify-every-merge gate discipline held the round together and kept main clean.
  Misses: I nearly false-accused @@LaneC of confabulating off a stale `git diff
  --stat` (caught it only by reading the actual diff - too close); I initially
  mis-diagnosed the poke-stacking as my outbound dropping; and I escalated a
  couple of calls to @@Host (some thresholds) I could have just made. Net: the
  ground-truth-verification habit was the most valuable thing I did, but I should
  apply it to my own alarms before raising them.

## Lane input + process improvements for round-3

@@LaneC's round-close input (incorporated):
- WORKED: the IDX fix held a full empirical re-walk (idle-while-embedding, draft
  no-wedge on the exact `Drafts/untitled/draft.md` path); grepping src + the
  served bundle before reporting caught two PHANTOM bugs (a stale local web/dist,
  the claude-in-chrome extension toast).
- @@LaneC self-flagged a real detour: chasing a `--carousel-off` "bug" that was
  just a stale local web/dist - CLAUDE.md already mandates rebuilding web FIRST;
  they skipped it (~one rebuild cycle) and saved it to memory. Good self-awareness.
- @@LaneC noted the architect routing stayed crisp - the adapt-the-plan acks
  (the Cmd+Shift+I server-side substitute, the shell-member -N test) kept them
  unblocked. Appreciated, and a fair balance to the architect self-critique above.

Actionable for round-3:
1. **Shared-file edit heads-up.** A one-line "editing <file>, builds may flicker"
   on the event channel from whoever holds a hot shared file mid-edit would save
   the next lane a transient-E0027 diagnosis cycle (it cost @@LaneC one on
   main.rs; same class as the round-1 shared-worktree discipline).
2. **Pre-allow localhost nav for QA lanes** (@@Alex): the browser-nav re-allow
   gate worked, but pre-allowing localhost navigation for QA lanes shaves a
   round-trip per lane.
3. **Rebuild web/dist before cargo** is in CLAUDE.md but still bit a lane - a
   louder callout in any browser-smoking lane's bootstrap is worth it.

## Round-close mechanics

- This `docs(phase-15)` commit captures the round-2 coordination tree
  (bootstrap/coordination edits + the event files, lane task/journal files,
  interfaces, refs, this retrospective, round-3-backlog).
- `web/package-lock.json` carries a standing 0.19.1->0.20.0 drift (the v0.20.0
  release did not commit the web pin); the v0.21.0 cut bumps web/ to 0.21.0 and
  supersedes it - left for the cut, not this commit.
- Release cut (version bumps across workspace + gateway + web + desktop +
  Cargo.lock -> full gate incl. the gateway workspace -> workflow_dispatch
  dry-run -> tag v0.21.0) waits for @@Host's explicit go.
