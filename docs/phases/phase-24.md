# Phase 24 - the visibility round: editor keep-alive, prompt-queue depth, survey-first host comms

Status: closed (round 1; three integrated gates green, instrumented
WKWebView walks green incl. the awake re-run, @@Alex hand-smoke "All
clean", both host surveys decided KEEP).
Span: 2026-06-12 → 2026-06-13.
Tags: #editor #terminal #team #desktop #refactor #bugfix

Round 1 of the new-team-2 six-member team (@@Conductor lead;
@@Editor, @@PromptQueue, @@TeamFlow, @@Desktop, @@CtxPass), the
second full round on the `cs terminal team` tooling. Scope was
authored by the previous team's lead, ratified by @@Alex via survey
before this team woke, and ran entirely on the bus: generated
bootstrap, append-only task files and journals, one-line pokes,
isolated-worktree gates, and `cs terminal survey` for every host
decision. The round's plan-level theme — making invisible state
visible (raw markdown on tab switch, undelivered prompt queues,
buried-window memory, survey keys) — repeatedly proved itself on the
team's own infrastructure: the round was coordinated over the very
poke queues item 2 makes visible, and six benign poke crossings were
reconciled from the on-disk task files the lean-poke discipline
mandates.

## Roadmap (the asks)

1. **Editor tab-switch** (item 1) — raw un-decorated markdown until
   click + scroll reset on every tab switch (WKWebView). Root cause:
   editor remount per switch. Fix: terminal-style keep-alive.
2. **Rich Prompt queue visibility** (item 2) — a submitted message
   vanished into a shared write queue with no feedback until the
   agent consumed it. Fix: pending-message state machine + depth
   badge, end to end (new WS frames, tagged queue entries).
3. **Teams start broadcast OFF** (item 3).
4. **Terminal tab-click focus** (item 4) — clicking a terminal tab
   activated but did not focus it (mousedown default action beats
   the focus microtask). Landed FIRST: load-bearing for item 1.
5. **Survey-first host comms** (item 5) — X-dismiss key, host key
   docs (1..N/F/X), bootstrap template rewrite.
6. **Launcher Open** (item 6) — always enabled, auto-turn-on,
   failure dialog with the real reason.
7. **Backlog**: B1 chan-server threaded-state ctx-pass refactor
   (deferred from the prior round "for a designed ctx pass");
   B3 capability negative pins; B4 Linux drop-path investigation;
   B5 buried-window cap semantics; B6 GTK menu-mutation check;
   B7 Xcode CI selection (watch item, untriggered).

## What shipped (22 commits on main, all local; no push)

Authoritative table with files and reviewers:
`new-team-2/designs/round-1-report-data.md` (mechanical sections by
@@TeamFlow). Summary by lane:

- **@@Editor** — ffbcc3ff (item 4: mouseup re-pulse; the design's
  microtask-vs-default-action analysis held exactly), dadd5e64
  (item 1: keep-alive — file tabs render like terminals, mounted +
  visibility:hidden; `{#key}` removed; autoFocus gated on focused to
  keep session restore from focus-fighting), bb877a87 (undo-boundary
  fix, below).
- **@@PromptQueue** — ca40ea6b (item-2 server half: QueuedWrite
  tagged entries, all-or-nothing enqueue_prompt — incidentally
  fixing a pre-existing silent CR drop at the queue cap — depth
  events, prompt-ack/delivered/queue frames), 86d50a25 (web half:
  pending state machine, read-only-while-pending compartment,
  honest reject/fail labels), 7c976a68 (tab-strip depth pill, incl.
  flipped-pane counter-mirror), b82a0a27 (lock-nuance comment from
  review note N1).
- **@@TeamFlow** — 86a0dce9 (item 5B: survey-first bootstrap
  template + 1..N/F/X key docs), 0f146fcf (item 3), c9fbb909
  (item 5A: X dismiss).
- **@@Desktop** — 3d4f564b (item 6: launcher Open + failure dialog,
  no listener leaks), 54b65a60 (B3 negative pins), f198df7b (B5:
  cap counts visible windows only + "Hidden Windows (N, kept warm
  in memory)" affordance — RATIFIED by @@Alex, survey 1).
- **@@CtxPass** — B1 in 9 commits across 4 waves: 7c6a36af
  (TreeMergeCtx), 396ad164 (IndexerShared widened), c15f6b35
  (FileRecord), 6e4253d4 (DraftScanAccum), f82aae50 (SlugAllocator),
  8f070e36 (FsGraphParams pass-through), e249de55 (FollowupSpec),
  126d9285 (TeamRequest + ControlSocketCtx), 3c45f35a
  (RestartOverrides). Four allow(too_many_arguments) retired, zero
  added; zero wire-shape changes; two leave-loose calls ratified.

**Found-and-fixed in flight:** the undo-past-load wipe. Item 1's
keep-alive preserves undo history across switches, widening a
pre-existing hazard: Cmd+Z past the initial-load boundary reaches
the empty pre-load doc and autosave persists the EMPTY file (hit
live during the keep-alive smoke; recovered via redo). bb877a87
makes exactly the initial fill non-undoable (per-instance flag,
non-empty-seed dedupe); the file-watch-reload path is byte-unchanged
and pinned. The wider reload-undo product question went to @@Alex
(survey 2): KEEP UNDOABLE — status quo ratified, no further change.

**Closed without code:** B4 (corrected finding: post-drop reads have
no Linux route, but capture-at-drag-time IS viable via wry's
webkitgtk handler — shim recorded as a designed future item), B6
(empirical: GTK in-place menu mutation SAFE over 12+1 bury/unbury
cycles + destroy storm on webkit2gtk 2.52.3; fallback stays
unwired), B7 (watch item; triggers on the next release run).

## Verification

- **Gates:** three isolated-worktree full `make pre-push` runs, all
  green — #1 @ 7c6a36af, #2 @ e249de55, #3 @ b82a0a27 (final HEAD).
  Lanes ran scoped own-gates with real flags (RUSTFLAGS="-D
  warnings"; make web-check); the shared tree was never a gate
  surface (three-lane-hot chan-server made it false-red by design).
  A 1-of-5 intermittent chan-server lib-test failure seen once in a
  lane gate never reproduced (4 captures + gate #3): recorded
  unreproduced.
- **Cross-review:** every commit adversarially reviewed by the
  paired lane; 13 reports, zero blocking findings. Methods
  escalated through the round: whitespace-insensitive rider walks,
  field-by-field positional→named transposition hunts, mutation
  verification (mutate the fix both ways, prove the pins bite),
  mechanism-level watch-item closure (OnceLock freshness argument).
  Two review-routing gaps (B5, badge) were caught by the report
  data audit — the review matrix is now the canonical review-debt
  tracker.
- **Instrumented WKWebView walk** (joint @@Desktop+@@Editor, real
  engine + real Rust, isolated $HOME, provenance-pinned binaries):
  item-1 keep-alive EMPIRICALLY GREEN on the surface the bug lived
  on (hosts mounted, raw-flash probe clean ×4 readbacks — DOM-text
  based, so valid despite the asleep display — undo boundary holds,
  ~8MB/doc linear memory ×20 tabs). Item-2 runtime-reactivity watch:
  0 errors / 0 state_unsafe_mutation across boot, 22 tabs, splits,
  reloads, paste storms — empirically closing the class static
  gates cannot see. Item-2 delivery semantics verified at the WIRE
  level 18/18 (Node walker; raw-vs-message depth divergence observed
  live; all-or-nothing cap rejection at raw 99). The asleep+locked
  overnight display blocked the composited/visual set — honest
  [blocked-env] splits, re-run harness retained.
- **Awake block (display composited, human-unlocked):** fit-loop
  CASE 1 (hidden tab) CLEAN — zero scrollback growth, poke delivered
  promptly while hidden: item-2 delivery is sound, the round-close
  gate PASSES. CASE 2 (buried window) CLEAN — delivery works while
  buried, unbury clean. The walk re-run flipped the blocked-env set
  to PASS: deep-scroll preserved exactly (3070→3070 across a
  switch, 198 decorations mid-doc), session-restore
  caret-lands-once FULL PASS (the Chrome-impossible check),
  new-draft caret, Cmd+. engagement (prior failures were a harness
  contract gap: app chords match e.code), busy-submit visuals
  (chip at 312ms, read-only composer, pill=1), flipped-pill
  counter-mirror verified to the exact transform. Composited
  console sweep: 0 errors / 0 state_unsafe_mutation.
- **@@Alex hand-smoke (clean rebuild 8b64ec7d, shrunk list in
  new-team-2/designs/alex-hand-smoke.md): "All clean — close the
  round."** That includes the one check only a human could prove
  (item-4 real-click focus), the item-2 dynamic visuals, DnD/drop,
  the item-6 pixel pass, and the B5 30-second check.
- **Joint observation (fit-loop) — RESOLVED BENIGN:** a hidden
  terminal's fit-loop spam (continuous resize→SIGWINCH→redraw
  holding the write queue's output-idle gate closed) occurs ONLY on
  an asleep/never-composited display — an automation-environment
  artifact, not a product hazard. Both surfaces (hidden TAB, buried
  WINDOW) measured clean awake; "bury the lead, lose the pokes"
  does not happen. B5's escape hatch needs no data. The fix
  candidates are moot; the harness lessons (e.code chords, xterm
  paste-pipeline input, legacy keyCode, unbury-IPC fronting) are
  recorded for the next walk harness.

## Host decisions (all via cs terminal survey, per the item-5 norm)

1. **B5 cap semantics — KEEP** (cap counts visible windows only);
   the buried-list-cap escape hatch stays documented for if buried
   memory ever hurts.
2. **Reload-undo — KEEP UNDOABLE** (status quo: file-watch reload
   remains a recover-from-overwrite path). Zero code change: the
   behavior was already pinned by bb877a87's tests, and the
   review-found boundary corner (an empty-at-open file's first
   content arriving via reload is non-undoable — it IS the first
   fill) stands as accepted.

## Follow-ups (compiled + deduped in round-1-report-data.md § 4)

Highlights: item-2 v2 (cancel/dequeue by id, durable pending ids,
skip-fail-when-terminal); FileTab.scrollTop session field + LRU
eviction; launcher dedupe-by-path guard; backgroundThrottling
dev-flag; fit-loop fix candidates [PENDING: case-2 verdict];
Linux-container multi-window non-materialization (needs a real
desktop check); B4 capture-shim (when Linux users exist); B2
dispatch-to-matcher refactor (never started, gated on its own design
note); aarch64-linux fp16 build note → lifted into
docs/contributing/linux-and-macos.md (done at close); round-1
inventory param counts → dated correction appended at source,
new-team-1/tasks/task-Chan-Lead-1.md (done at close); pane-mode
(Cmd+.) round-trip scrollTop reset — pre-existing, @@Editor judgment
pending, surfaced by the awake walk.

## Retrospective

### Highlights

- **Design-doc-first paid for itself twice over.** B1 — deferred a
  round specifically to get a designed ctx pass — landed 9 commits
  with ZERO review findings, and both mid-flight design deviations
  (a missed third allow; reusing an existing query type) surfaced as
  one-line flags ratified in minutes instead of silent drift. The
  signed-off design also made every reviewer's job tractable: the
  reviews verified a contract, not an intention.
- **Review rigor escalated all round** and stayed cheap: the
  high-water marks (diff -w rider walk on a 484-line reindent;
  mutation-verifying a fix's test suite; closing a flagged
  observable-order change with an OnceLock freshness argument) each
  cost minutes and would each have caught a real bug class.
- **The data-prep audit caught what routing-from-memory missed**:
  two landed commits with no review row. The review matrix is the
  review-debt tracker from now on.
- **One real bug found, fixed, and verified in-flight** (undo-past-
  load wipe) — found BY the round's own smoke, fixed narrow, the
  product half correctly deferred to the host.
- **The bus held under live fire.** Six poke crossings, two
  multi-deep hot-lane queues, zero consequences: the on-disk task
  files carried the authoritative state every time. Fittingly, the
  round shipped the exact visibility feature (queue depth) whose
  absence caused the crossings.
- **Honest verification splits.** Nobody forced a flaky pass: the
  walk's [blocked-env]/[hand-smoke] ledger, the wire-level pivot
  when Chrome perms blocked, and the vacuous-pass refusal on item-4
  automation are the round's quality floor.

### Lowlights

- **Review routing ran from memory until the audit institutionalized
  the matrix** — two gaps (B5, badge), both from commits landing
  outside the completion-poke rhythm.
- **The lead's ledger drifted twice** (a premature "review program
  complete"; a 24-vs-22 commit miscount) — both caught by
  cross-checking git/the data file, both correctable because every
  claim was written down. Verify against the artifact, not the
  recollection.
- **Hot-lane poke queues stacked invisibly** (3-4 deep on two
  lanes). Pokes that were pure FYI made it worse. Item 2's badge is
  the fix for visibility; leaner FYI discipline is the fix for
  volume.
- **The overnight walk hit the asleep-display wall** — half the
  dynamic checklist deferred to a ~10-min awake block. Lesson:
  schedule instrumented walks when a human session is live, or
  accept the split up front (the retained-harness pattern made the
  split cheap).
- **The prior round's inventory numbers didn't reproduce** (param
  counts off by 1-4 everywhere) — qualified greps at HEAD are now
  the standard for any recon that later work consumes.

### Feedback — workers

- **@@Editor**: the round's biggest change (484-line restructure)
  landed with zero riders and the best evidence discipline of the
  team (stash-and-reprobe controls; honest Chrome-untestable lists).
  Owning the A1.3 spec bug in the walk co-sign instead of defending
  it is exactly the culture. Stretch B2 correctly never started.
- **@@PromptQueue**: end-to-end feature of the round; the wire-level
  walker pivot under a blocked browser was the round's best
  improvisation, and keeping the failed walker run in evidence "for
  honesty" set a tone. Reviews closed watch items with mechanisms,
  not vibes.
- **@@TeamFlow**: four clean reviews that kept raising the bar
  (mutation verification is now house style), plus the data-prep
  audit that caught both routing gaps. Smallest code lane, largest
  quality footprint.
- **@@CtxPass**: nine refactor commits, zero findings, every
  deviation self-flagged before anyone asked. The verified-counts
  table (vs the inventory) is the model for consuming prior-round
  recon.
- **@@Desktop**: the instrumented-walk harness is a genuine
  capability the team didn't have at round start; the B4 stop-rule
  execution (investigate, correct the record, write the note, no
  code) and the two-case runbook encoding show exactly the right
  relationship between findings and authorization.

### Feedback — lead (@@Conductor, self)

- Route reviews off the matrix, not memory (two gaps).
- Don't declare completion without re-deriving from the artifact
  (two ledger drifts).
- FYI pokes to hot lanes are negative-value; batch them into the
  next actionable poke.
- What worked and should persist: verify-before-relay on every
  worker gate claim ("the gate is mine to release"); priority-
  ordered review tasks that never dented the critical path; the
  early parallel gate #1 instead of waiting for a "better" moment;
  pulling case-2 into the awake session while keeping it
  non-gating.

### Feedback — host (@@Alex)

- Pre-ratifying scope + designs before the team woke made this the
  smoothest kickoff yet: zero scope questions all round.
- The authorize-with-cheap-veto pattern (B5) worked: the lane never
  blocked, and the survey arrived with a decision note instead of an
  open question. Recommend it as the default for product-flavored
  backlog items.
- Two deliberate non-contacts to audit: no kickoff/mid-round status
  surveys (the plan's survey-first override was read as "prefer
  surveys WHEN communicating", not "ping more"), and the Chrome
  allowlist blocker was routed around (option a) rather than asked.
  If you'd rather be pinged earlier in either case, say so for
  round 2.

## Notes

- The round ran 2026-06-12 evening → 06-13, mostly autonomously
  overnight; all host interaction concentrated at ratification
  (pre-round) and close (surveys + smoke).
- new-team-2/ (tasks, journals, designs, evidence, this report's
  data file) is committed alongside this report in the
  docs(phase-24) round-close commit, plus two riders: the fp16
  build note (docs/contributing/linux-and-macos.md) and the dated
  inventory correction (new-team-1/tasks/task-Chan-Lead-1.md,
  appended per append-only). Pre-existing uncommitted new-team-1
  working-state changes (bootstrap.md, config.toml, journal-Lead.md
  — the prior round's handoff edits) are NOT part of this commit;
  flagged to @@Alex for disposition.
- Teardown at close: walk/fixture artifacts under /tmp +
  /private/tmp removed, b6gtk container removed (fs image kept);
  the conductor-gate worktree persists as the build base. Local
  commit only — no push (standing rule: push on explicit ask).
