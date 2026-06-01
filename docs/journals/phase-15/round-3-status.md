# Phase-15 round-3 status (architect-owned, live)

ROUND CLOSED: v0.22.0 cut (release: bump 975737c3). All 6 themes addressed,
3/4 lanes verification-complete, RELOAD-HANG cured in Wave 1. 2 editor smokes
ship empirically-unverified (navigate denied to all lanes) per the pre-release
norm; round-4 backlog seeded. See round-3-retrospective.md. Wave 3 COMPLETE +
merged + barrier-gated.

Architect-handled @@Host nits this wave (live, landed on main, gated-green,
pathspec-committed so they did NOT sweep B's in-flight docs edits):
  0b97944f  cs terminal survey --help gains an EXAMPLES block: each
            supported case (single question, 4 options via stdin, [F]
            follow-up) paired with the wire SurveySpec JSON. chan-shell/
            cli.rs (D's file) - help text only, no behavior change.
  317074c6  Dropped the `embeddings: on/off` row from the Dashboard About
            card. Already covered (richer) by the Search dashboard slot
            (SearchSlotConfig: live hybrid/BM25 mode + off-state hint +
            model picker), so no info lost. web/ EmptyPaneCarousel.svelte
            + its carousel test (B-adjacent dashboard files).
  @@LaneD / @@LaneB: FYI only, both already committed; reconcile if you
  were mid-edit on cli.rs / EmptyPaneCarousel (low risk - your Wave-3
  scopes do not touch these).

On (re)start, read this first to learn the active wave, then do your lane doc's
section for that wave. @@Architect updates this file at every barrier; it is the
single source of "where are we" after a refresh.

## Wave status

```
legend: -- todo  ~~ in progress  GG gated-green  MM merged
        VV verified (verification-only wave, no code to merge)
+------+--------+--------+--------+
| Lane | Wave 1 | Wave 2 | Wave 3 |
+------+--------+--------+--------+
| A    |   MM   |   MM   |   ~~   |
| B    |   MM   |   MM   |   MM   |
| C    |   MM   |   MM   |   VV   |
| D    |   MM   |   MM   |   VV   |
+------+--------+--------+--------+
```

Wave-3 progress (latest at top):
  - A (me): backend scope DONE + merged. Graph hygiene: (1) ghost-node fix
    beb0dc49, (2) EDGES-PK fix ebee9a15 (anchor in the PK; NO migration, v1
    schema changed, fresh ~/.chan adopts it per @@Host). IDX Theme-5 Option B
    (decoupled-signal form, @@Host-chosen depth): (3) chip-clobber fix
    41e7908e (shared bg_embed signal owned off IndexStatus so a watcher
    reindex re-attaches the chip instead of dropping it; wire unchanged, no
    FE change), (4) in-flush freeze fix ba372dcb (EMBED_BATCH_CHUNKS 4096 ->
    2048), (5) Metal hang = follow-up item (gate CHAN_ENABLE_GPU verified
    live; investigation -> round-4 backlog). @@Host nits landed: 0b97944f
    survey --help, 317074c6 About-card embeddings, fbe9bb90 types.ts ghost.
    REMAINING: the 2 B-editor browser smokes (click-caret, [[ stuck bubble),
    then the release gate (+ gateway) + docs(phase-15) commit on @@Host's go.
  - B: Wave-3 DONE + merged (a930a96f Theme-6: 500 raw deletions + 13 README
    essence docs + phase-14 README; pathspec-clean, caught + fixed a greedy-
    regex data-loss in phase-9 before commit). graphData needed NO change
    (my ghost-node fix at the source satisfied it; B verified by source-read,
    not a fabricated diff). Two escalations resolved: types.ts:381 stale
    ghost comment FIXED by me (fbe9bb90); phase-8 raw DEFERRED to round-4
    (see round-4 backlog).
  - C: Wave-3 DONE (verification-only). Survey browser smoke all paths PASS;
    cleared the Wave-2 survey-SPA unverified item. No code, no carryover.
  - D: Wave-3 DONE (verification-only). All 4 verify tasks PASS: survey
    option-pick e2e (:7841), [F] covered by C's joint smoke, desktop cs
    argv0 dispatch + `chan open` removal verified against the REAL desktop
    socket (all 4 agent terminals listed). AppImage cs re-exec + the carved
    .md file-handler stay round-4 (empirically-unverifiable here). No code.
  - B: Theme-6 docs cleanup in-flight (phase-*/README.md edits live in the
    worktree) + graph frontend pending A's ghost-node fix (now ready).

Wave-2 commits on main (local, not pushed), barrier-verified:
  c854d3f8  A    search mentions/paths/.md (BM25 subtoken split; chip -> W3)
  9349dba2  B    heading/block links + image spaces + click-to-caret
  08d7435b  C+D  survey round-trip + per-member agent field + desktop cs +
                 drop chan open (one coherent commit: chan-shell<->chan-server
                 <->SPA mutually dependent, lib.rs/client.ts/mod.rs co-edited)
Wave-2 barrier gate (coherent HEAD 08d7435b): fmt 0 + clippy --all-targets -D
warnings 0 + cargo test 0 (528 chan-workspace + survey 11/8/4 + team_config 11,
zero failures) + build --no-default-features 0 + svelte-check 0/0 + npm build 0:
ALL GREEN. Tree clean but for untracked round docs.

Wave-2 BROWSER-UNVERIFIED (carried to Wave-3 joint smoke; shared-browser
`navigate` denied to B + C that wave, gated-green + source-tested + committed
under the pre-release-merge-unverified norm):
  - [CLEARED Wave-3, @@LaneC] survey SPA overlay render + reply round-trip.
    Empirically verified end-to-end in a real browser at HEAD 08d7435b: option
    pick via click + via keyboard (4-option cap, key handler does not leak into
    the PTY), and [F] -> followup file created through the Workspace sandbox
    with the exact pre-populated content. `to` resolves to the survey target
    per contract. No code changes, no carryover. (My two nit commits on top do
    not touch survey SPA code, so C's verification holds at current HEAD.)
    See round-3-lane-c-journal.md Wave-3 section.
  - STILL PENDING (B editor items, gated-green + source-tested Wave-2):
    - B click-to-place-caret (blank-area click drops the caret)
    - [[ stuck-Indexing bubble FE-resolves-on-idle (needs a churning drive)
    MISATTRIBUTION CORRECTED (2026-06-01): a relayed "LaneB completed" was
    wrong - @@LaneB NEVER ran these (journal correctly shows them unverified;
    `navigate` was denied to B again, even with a fresh server parked on
    :7843). Caught by checking B's journal/commits for evidence before
    clearing (confabulation-discipline near-miss). RESOLUTION (@@Host chose
    (b), 2026-06-01): SHIP both empirically-unverified for v0.22.0 under the
    pre-release-merge-unverified norm. Both gated-green + source-tested;
    `navigate` was denied to BOTH @@LaneB and @@LaneA, so no lane could
    browser-verify this round. @@Host re-reports if either misbehaves; carry
    to a round-4 browser pass when navigate is re-allowed. (Bonus: clobber
    fix 41e7908e partially confirmed via curl - a live server showed
    embedding:{done:300,total:301} during a real background embed; the full
    edit-during-embed transition stays locked by the set_idle_reattaches
    unit test.)

Wave-1 commits on main (local, not pushed), barrier-verified:
  8eb99391  C  team-in-workspace + drop bubble stub
  d1b7c427  A  preflight RELOAD-HANG fix
  b273e0b5  B  [[ relative-markdown links + stuck bubble + resolver %-decode
  68a2adef  D  chan-shell crate + per-agent submit map
Wave-1 barrier gate (coherent HEAD 68a2adef): fmt + clippy --all-targets -D
warnings + cargo test (zero failures, 524 chan-workspace / 328 chan-server inc.
A+C+D's new tests) + build --no-default-features + svelte-check 0/0 + npm build:
ALL GREEN. RELOAD-HANG live smoke PASS (584 reindexing samples, 0 lock
violations; cold build still locks). adb68241 (the collision commit) orphaned;
worktree clean but for untracked round docs.

## Wave-3 scope per lane (lanes re-orient from their own lane-doc Wave-3 sections)

- A (me): IDX Theme-5 = Option B (embeddings as a background job with its own
  status, off the reindex contract) + the FOLDED chip fixes (clobber +
  in-flush freeze) + Metal hang follow-up (CHAN_ENABLE_GPU). Graph: backend
  ghost-node fix (drop unresolved-target nodes) + the EDGES-PK fix (anchor in
  the PK; see carryover below). Then DRIVE the final release gate (incl. the
  gateway nested workspace) + the round-close docs(phase-15) commit on @@Host's
  go. MAY spawn subagents (esp. Option B).
- B: Theme-6 docs/journals cleanup (DELETE-RAW + SUMMARIZE, @@Host-confirmed;
  defer artifacts cited by live URL/ID) - runs AFTER B's Wave-1 relative-link
  rule (done). + graph hygiene FRONTEND (graphData.svelte.ts) paired with A's
  ghost-node fix. Spawn a subagent for the bulk cleanup.
- C: JOINT survey browser/desktop smoke with D (the SPA render + reply
  round-trip C could only curl-verify); any Team-Work polish carryover.
- D: JOINT survey smoke + DESKTOP verifies that need a real .app (cs argv0
  dispatch, AppImage cs wrapper, chan-open-removal handoff) - desktop-only,
  @@Host/desktop verify. NO .md fileAssociation (dropped, @@Host call).

## Touch points this wave (@@Architect-held)

- A<->B Wave-3 GRAPH: A backend ghost-node fix (graph.rs / fs_graph.rs - drop
  unresolved link-target nodes) + the edges-PK fix; B frontend graphData
  consumes the cleaned graph. A provides the shape; B renders. Sequence so the
  ghost-node backend lands before B's graphData change.
- Theme-6 (B) runs AFTER A's relative-link rule (already landed Wave 1); B
  defers deleting any artifact still cited by a live URL/ID (audit-trail rule).

## Touch points (Wave-2, RESOLVED - kept for the retrospective)

- C<->D survey contract: round-3-survey-contract.md pins the SurveySpec /
  SurveyReply JSON shape, the synchronous blocking flow, and the ownership split
  (D = command + TermSurvey frame + shared Rust wire type + survey bus +
  WindowCommand; C = SPA overlay + reply route calling D's bus + followup file).
  AMENDED 2026-06-01 (@@LaneC escalation, RESOLVED): SurveySpec gains
  `followup: { dir, from, to } | null` (populated only when --followup); cli
  gains `--followup-dir` (required w/ --followup). D adds the wire field +
  flag; C echoes it back + names `followup-{from}-{to}-{n}.md`. Both poked.
- A<->B search-API contract: RESOLVED. PROBE done; semantic is NOT on the query
  path (every search is bm25), so the gap is pure punctuation tokenization. A's
  bm25 fix landed (c854d3f8): mentions/paths/.md now match server-side, response
  shape UNCHANGED -> B's search FE is DISPLAY-ONLY (pass raw query through). See
  round-3-search-api-contract.md; B poked.

## Architect decisions (made; for @@Host awareness)

- Linux AppImage `cs` story = OPTION (a): chan-desktop installs a `cs` wrapper
  into `~/.local/bin` on first run (argv[0] detection). Architect + @@LaneD both
  lean (a); decided so D is unblocked. Not the double-tool dependency.
- Survey [F] team context = full `followup: {dir, from, to}` on SurveySpec (not
  the minimal followupDir fallback), matching the followup-{from}-{to}-{n}.md
  naming. Cheap to add while D is mid-flight.
- chan-open / file-handler scope = OPTION (B) (@@LaneD's lean, ratified). DELETE
  `chan open` NOW (verifiable: `cs open` covers the in-terminal OpenPath;
  `maybe_handoff_to_desktop` STAYS - shared with `chan serve`; remove only
  `cmd_open` + `workspace_root_for` + `pick_workspace_root`). DONE by @@LaneD.
- System-wide `.md` file handler = DROPPED (@@Host call, 2026-06-01: "i do not
  want that, at all"). The directive's "desktop becomes the OS .md file handler"
  is NOT happening - no `bundle.fileAssociations`, no round-4 task. Verified the
  tree adds none (tauri.conf.json untouched). The `chan serve` -> desktop
  handoff + `cs open` remain the only file-open paths; Finder double-click of a
  bare .md is intentionally NOT a chan feature. (cs_install.rs is the unrelated
  AppImage `cs` wrapper, kept.)

## @@Host awareness needed (surfaced by the search PROBE)

- SEMANTIC SEARCH IS BUILT BUT NEVER QUERIED. The probe found every search path
  (HTTP route + `chan search` CLI) is BM25-only: `SearchOpts::default().mode ==
  Bm25` and no user-facing caller requests Hybrid/Semantic, yet dense vectors
  are computed + stored on every reindex. So we pay embed compute for retrieval
  that nothing reads. This is a product/scope question (flip hybrid on by
  default? gate behind the existing semantic_enabled opt-in + wire it into the
  route?), NOT a Theme-4 bug, so it is OUT of the mentions/paths fix scope.
  Raised for @@Host to decide direction (possible round-4 item). Theme 5 Option
  B / Metal work is separate and proceeds regardless.

## @@Host decisions (resolved)

- Theme-6 docs/journals cleanup (Wave 3, @@LaneB) = DELETE RAW + SUMMARIZE
  (@@Host confirmed 2026-06-01). Per-phase essence docs with hashtags
  (#reliability / #features / #bugfixes / ...), transcribe then delete images,
  delete the raw round data (preserved in git history). SAFEGUARD: @@LaneB
  defers deleting any artifact still cited by a live URL/ID until those
  references are updated, so the audit trail does not break (cf. the
  "destructive cleanups coordinate with docs" rule). Runs AFTER B's Wave-1
  relative-link rule (done) so the cleanup emits relative links.
- Linux AppImage `cs` = OPTION (a): ~/.local/bin wrapper on first run (see
  Architect decisions above).

## Wave-2 barrier debt (@@Architect-tracked; resolve when sequencing merges)

D's survey transport is code-complete (D's files clippy+test green; followup
amendment incorporated). Open items to resolve at the Wave-2 barrier:

- chan-server survey merge order C<->D: D's `AppState.survey_bus` +
  `SurveyBus::complete_survey` carry `#[allow(dead_code)]` because their only
  consumer is C's not-yet-landed `POST /api/survey/reply` route. DROP both
  allows in whichever commit lands C's route (else clippy flags needless-allow).
  Sequence: D's transport first, then C's route + the allow removals together.
- `lib.rs::router()` reply-route mount is C's edit to a shared file (the route
  assembly point). Sequence C's router() touch so it does not collide with D's
  control-socket/state edits; pathspec-commit both.
- `AppState.survey_bus` rippled into 4 test AppState ctors (routes/{index,
  reports_toggle,screensaver,search}.rs) - mechanical. These show as D's edits
  in the shared tree now; A's bm25 commit (c854d3f8) correctly excluded them.
- `routes/team_config.rs:339` (C WIP) trips clippy::type_complexity and blocks
  the full-crate clippy gate. C must fix before C's merge; the authoritative
  gate is the architect barrier gate on the merged HEAD.

## Wave-3 carryover for @@LaneA (graph)

- EDGES-PK (B's finding, graph.rs:373): the `edges` PK is `(src, dst, kind)`
  with `anchor` a plain nullable column, so a single file linking the SAME
  target with two different anchors (e.g. a heading anchor + a block anchor)
  collides on `INSERT OR IGNORE` and keeps only the first. Niche (single links
  resolve fine), not a Theme-3 bug, but it caps per-file multi-anchor links to
  one. FIX in Wave-3 graph work alongside the ghost-node fix: anchor
  `NOT NULL DEFAULT ''` + PK `(src, dst, kind, anchor)`. Pre-release, graph DB
  rebuilds on reindex, so no migration.

## Cross-lane notes (latest at top)

- SHARED-CHROME COLLISION (Wave 3, resolved, for the retrospective): C and D
  ran their survey browser smokes concurrently (C :7901, D :7841) but the
  Chrome MCP extension drives ONE shared tab group, so D's tab got navigated
  to C's port mid-run and orphaned D's first [F] survey (overlay pushed then
  lost to the nav; the blocked CLI never got a reply). Resolved peer-to-peer:
  D killed the stuck CLI, confirmed server-side survival, poked C; C had
  already covered option-click + keyboard + [F] file-create on :7901, so they
  split coverage (D = option-pick e2e, C = keyboard + [F]) rather than
  re-fight the shared tab. LESSON (standing): the Chrome MCP tab group is
  shared across lanes exactly like the worktree; a joint browser smoke needs
  the same one-driver-at-a-time / claim-by-poke discipline as a shared-file
  commit. Folds into the existing lane-boundaries + persistent-test-server
  norms.

- COMMIT-COLLISION INCIDENT (Wave 1, resolved, for the retrospective): B's first
  commit (adb68241) swept in D's uncommitted chan-shell + control_socket via a
  blanket stage in the inter-command window. Recovered peer-to-peer: B `git reset
  HEAD~1` (mixed, preserves worktrees) then re-committed editor-only via the
  race-proof `git commit -F msg -- <paths>` pathspec form (b273e0b5); D
  re-committed cs-shell as its own clean commit (68a2adef). A (d1b7c427) + C
  (8eb99391) below were untouched. LESSON (now standing for the round): in this
  shared worktree the ONLY race-proof commit is `git commit -F msg -- <explicit
  paths>`; plain `git add` + `git commit`, even chained, can be contaminated by a
  peer's concurrent staging. All lanes use the pathspec form from here.
- teams.rs gray area RESOLVED: it carries TeamConfig.tab_group (C's schema
  domain), disjoint from A's index/search scope.
