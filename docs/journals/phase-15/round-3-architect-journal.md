# Phase-15 round-3 - @@LaneA / @@Architect journal

Append-only. Latest entry at the bottom.

## Wave 1 (active)

### Orchestration kickoff

Re-oriented from round-3-bootstrap.md -> round-3-status.md -> round-3-lane-a.md
-> round-3-plan.md -> round-3-backlog-comments.md, then read all three peer lane
docs (b/c/d) to hold the full Wave-1 picture. Flipped round-3-status.md to
"Wave 1 (active)", marked all four lanes in-progress (confirmed by live edits in
the shared worktree), and recorded the cross-lane notes. Coordination duties for
the round: own round-3-status.md, gate+sequence merges at each barrier, arbitrate
the A<->B (search API + graph) and C<->D (survey transport) seams, run the
refresh handshake with @@Host.

### RELOAD-HANG fix (critical, Wave-1 priority) -- DONE (code + unit tests)

File: crates/chan-server/src/routes/preflight.rs (+92/-11).

Root cause (re-confirmed against current source, not just the plan):
`index_step` mapped `IndexStatus::Reindexing` AND `IndexStatus::Building` both to
a locked `Running` preflight step. `build_snapshot` turns any non-Done step into
`phase=Running, locked=true`, and the SPA's PreflightOverlay is a full-screen,
no-close, ESC-ignored gate while `locked`. So ANY incremental watcher reindex
re-locked the entire booted UI. @@Host's repro: configuring the Dashboard /
flipping panes writes session+layout files -> watcher -> `Reindexing` -> a Cmd+R
caught in that window hard-locks until the indexer settles (slow on a large
drive), reading as a permanent hang that needs a server kill.

Why the fix is broader than the plan's literal minimum: I traced both
`Building` set-sites in indexer.rs. Site 1 (line 333, spawn_coordinator) fires on
EVERY full-rebuild request from the rebuild channel -- including a mid-session
coalesced/VCS-burst rebuild (>64 files), not just the cold boot build. Site 2
(line 861, StatusUpdater) holds `Building` only during the foreground BM25 pass
and flips to `Idle{embedding:Some}` at the first embed-batch flush (Option A:
BM25 commits incrementally in facade.rs build_all ~698/742, so search is live and
preflight unlocks while embeddings finish in the background). Net: a mid-session
full rebuild parks status at `Building` over an ALREADY-searchable index, and the
old code locked the overlay there too -- same session-crashing bug class as the
reported Reindexing case, just a rarer trigger. The plan's stated invariant was
exactly right: "only the cold initial build (indexed_docs == 0) should lock."

Fix: the boot overlay is a FIRST-boot gate, so only an unsearchable index may
lock it.
  - `Reindexing` -> Done (always; an incremental reindex is always over a built
    index).
  - `Building` + `indexed_docs == 0` -> Running/locked (genuine cold first build,
    nothing committed yet). This is now the SOLE locking state.
  - `Building` + `indexed_docs > 0` -> Done (warm mid-session rebuild over a
    searchable index; must not re-lock a booted session).
  - `Idle` -> Done; `Error` -> Failed (unchanged).
`build_snapshot` reads the live BM25 doc count via `workspace.index_stats()` (the
codebase's canonical cold-build signal, cf. workspace.rs:2246) and passes it to
`index_step`. A stats read error falls back to 0 (treat as cold, stay locked) --
conservative: never open onto an unproven index.

Verified mid-cold-build safety: during a cold build doc_count stays 0 until the
first embed-flush commit; at that commit the status independently flips to Idle,
so the overlay unlock is governed by the Idle transition either way -- the
indexed_docs gate never unlocks a genuinely-empty cold build prematurely.

Tests (crates/chan-server/src/routes/preflight.rs, all green):
  - reindexing_never_locks: Reindexing -> Done at indexed_docs 0 and 1200.
  - cold_build_locks_but_warm_rebuild_does_not: Building@0 -> Running (keeps
    current/total); Building@42 -> Done.
  - reindexing_keeps_preflight_unlocked: end-to-end build_snapshot on a fresh
    BM25 workspace with Reindexing -> phase Ready, locked:false (the exact
    contract the lane doc's curl check specifies).
  - building_index_locks_until_settled (pre-existing) still green: a fresh
    workspace is indexed_docs == 0, so a cold Building still locks.

Gate status: fmt clean; `clippy --all-targets -- -D warnings` clean across the
whole workspace (compiled at a coherent moment); `cargo test -p chan-server
--lib routes::preflight` 8/8 green on the integrated tree. Full `cargo test` +
`--no-default-features` + web build deferred to the wave barrier (the shared tree
flickers mid-wave as @@LaneD's chan-shell lands; the full gate is only meaningful
on a coherent tree).

PENDING at barrier (the one empirically-unverified item): the live large-drive
browser smoke -- serve a ~1200-file drive from a renamed binary copy, induce
`Reindexing` under file churn, curl `/api/preflight` to confirm `locked:false`,
and reload in the browser to confirm no hard-lock. Deferred to barrier to run
against a coherent tree + a freshly-built binary (a stale binary on a churning
mid-wave tree would give a false result).

### Wave-1 BARRIER (all four lanes done) -- CLOSED + refreshed into Wave 2

Order on main (local): 8eb99391 C, d1b7c427 A, b273e0b5 B, 68a2adef D. Each
verified clean via `git show --stat` (no cross-lane sweep).

COMMIT-COLLISION INCIDENT (resolved peer-to-peer): B's first commit adb68241
swept in D's uncommitted chan-shell + control_socket (blanket stage in B's
inter-command window). My forensic snapshot caught a transient mid-recovery
state (D's work uncommitted) and I started to re-commit it for D, but D had
already self-committed it as 68a2adef (my `git add` was a no-op, no harm). B
recovered via `git reset HEAD~1` (mixed) + the race-proof `git commit -F msg --
<paths>` pathspec form. LESSON ratified for the round: pathspec commit only;
plain add+commit (even chained) is contaminable. Recorded in status for the
retrospective.

Barrier gate on coherent HEAD 68a2adef: fmt=0, clippy --all-targets -D
warnings=0, cargo test=0 (zero failures; 524 chan-workspace + 328 chan-server
incl. my 8 preflight + D's chan-shell + C's team tests, 34 targets), build
--no-default-features=0, svelte-check 0/0, npm build green.

RELOAD-HANG LIVE SMOKE -- PASS (this closes the one empirically-unverified
item). Renamed binary /tmp/chan-lanea-smoke, --standalone --no-token, 300-file
drive on :7811, scoped teardown. Caught the cold build at locked:true
(cold_lock_seen=1, first-boot gate preserved), then a 300-file edit burst gave
584 `reindexing` samples with ZERO lock violations (preflight stayed
locked:false throughout). The warm-Building branch was not exercised live (a
non-git drive does not coalesce into a rebuild); it stays covered by the
cold_build_locks_but_warm_rebuild_does_not unit test. Drive + server torn down,
no peer servers touched, git HEAD intact.

REFRESH HANDSHAKE: wrote round-3-survey-contract.md (C<->D shape + ownership),
flipped status to Wave 2 active with per-lane scope + carryover, decided AppImage
= option (a). Surfacing the Theme-6 docs-cleanup destructiveness decision to
@@Host at this refresh so it is locked before Wave 3. Poking @@Host to refresh
all into Wave 2.

## Wave 2 (active)

### Re-oriented + picked up coordination

Refreshed into Wave 2. Confirmed all four lanes at the Wave-1-done / start-of-
Wave-2 state (peer journals + git log: 8eb99391 C, d1b7c427 A, b273e0b5 B,
68a2adef D on main). event-*.md are stale (round-2/v0.21.0 era); round-3 runs on
the lean poke bus (cs terminal write). round-3-status.md already at Wave 2 active
with per-lane scope. Survey contract (C<->D) confirmed solid before the seam
escalation below.

### Theme 4 search PROBE -- DONE, fix landed (c854d3f8)

Ran the live probe the plan demanded (renamed binary /tmp/chan-lanea-probe,
embed-model bundled, 10-doc drive with mentions/paths/.md + topical distractors,
port 7821, scoped teardown). Two findings:

1. SEMANTIC IS NEVER ON THE QUERY PATH. Every `/api/search/content` query
   reported mode=bm25. `SearchOpts::default().mode == Bm25` (facade Mode default
   is Bm25); the route's "defaults to Hybrid" comment is STALE; `chan search`
   CLI is BM25-only too. Dense vectors are built + stored (indexed_vectors=10)
   but NO user-facing caller requests Hybrid/Semantic. So @@Host's "maybe
   semantic already covers mentions/paths" is empirically FALSE -- semantic is
   dead weight at query time. SURFACED to @@Host as a separate product question
   (flip hybrid on? it is built-but-unqueried); NOT folded into this fix.
2. The gap is pure punctuation tokenization. Bare words already match; the
   literal @ / . in the prefix regex made the punctuated forms return nothing.

Fix (bm25.rs, my file): `try_build_prefix_query` splits each whitespace token
into the alphanumeric subtokens the default tokenizer produced and ANDs them.
`@@LaneA` -> lanea; `src/routes/search.rs` -> src AND routes AND search AND rs;
`bootstrap.md` -> bootstrap AND md. Bare word -> one subtoken, so ordinary
queries are byte-identical; snippet highlight uses the same subtokens. No
reindex, no response-shape change. Tests: mention/path/filename + the helper
(528 chan-workspace lib tests green). LIVE re-probe after rebuild: @@LaneA 0->2
hits, @@Architect ->3, search.rs ->1, bootstrap.md ->1, full path still 1; every
plain-query control (pasta, garlic, team roster, bare LaneA) unchanged; snippet
bolds LaneA. Scoped gate (fmt/clippy --all-targets/test on chan-workspace)
green. Committed pathspec-only as c854d3f8 (bm25.rs alone; verified staged stat
+ git show --stat HEAD -- the tree also carries D's survey-bus AppState fan-out
into routes/{search,index}.rs test modules, NOT swept).

A<->B contract written (round-3-search-api-contract.md) + poked B: response shape
UNCHANGED, search FE is display-only, pass the raw query through, do NOT
client-side parse mentions/paths. Optional FE affordance only. Non-goal:
node-level mention precision (graph concern, not BM25).

### C<->D survey seam arbitration (escalated by @@LaneC)

@@LaneC flagged the [F] followup needs team context but D's SurveySpec carried
none, and neither the SPA nor the reply route can re-derive the team-dir (multi-
team workspace). The context originates with the surveying agent ($CHAN_TAB_NAME
+ bootstrap-known team-dir), so it must ride the wire. DECISION (D mid-flight, so
cheap now): SurveySpec gains `followup: { dir, from, to } | null`, populated only
when --followup; cli gains `--followup-dir` (required w/ --followup). Full shape
(not the minimal followupDir fallback) to match the established
`followup-{from}-{to}-{n}.md` naming. Recorded as the 2026-06-01 amendment in
round-3-survey-contract.md; poked D (add the wire field) + C (approved, proceed).

### Next (my Wave-2 remainder)

IDX chip fixes (Theme 5): chip clobber (shared bg-embed signal off the reindex
status) + in-flush chip freeze (heartbeat / smaller EMBED_BATCH_CHUNKS). Then the
Wave-2 barrier verification + refresh handshake.

### Wave-2 coordination round (decisions + lane handoffs)

- IDX chip fixes FOLDED into Wave-3 Option B (architect call): chip-clobber is
  structurally Option B's "shared bg-embed signal off the reindex status", and
  the in-flush freeze is the same embed-status concern; both are cosmetic polish
  behind the landed Wave-1 preflight fix (the real RELOAD-HANG cure). Doing
  piecemeal mitigations now then re-architecting in Option B = throwaway. So A's
  Wave-2 deliverable is Theme-4 search + coordination; chip + Option B + Metal
  land together in Wave 3. (Offered @@Host the cheap EMBED_BATCH_CHUNKS tuning as
  an opt-in if they want it banked early.)
- chan-open / file-handler scope = OPTION (B) (D's escalation): delete `chan
  open` now (cs open covers in-terminal; maybe_handoff_to_desktop stays for
  `chan serve`), CARVE the "desktop becomes OS .md handler" to round-4 because
  it needs a new tauri fileAssociations + RunEvent::Opened capability that is
  (a) unverifiable in this env (real .app + Finder double-click) and (b) a
  SYSTEM-WIDE default-.md-handler claim = a @@Host product call. Known accepted
  pre-release gap: CLI-install .md double-click breaks until round-4. Poked D.
- @@LaneD survey transport DONE (code-complete, D's files green; followup
  amendment incorporated correctly). Barrier seam debt recorded in status:
  survey_bus dead_code allows (drop with C's route), C's router() mount,
  team_config.rs:339 type_complexity (C WIP). Acked D.
- @@LaneB Wave-2 DONE + merged (9349dba2): heading/block round-trip + image
  spaces + click-caret; search FE correctly display-only (consumed my contract).
  Two items browser-pending (click-caret + stuck-bubble) - navigate was denied
  to B (shared browser), so I run those smokes at the barrier on a coherent
  rebuilt server. EDGES-PK finding (graph.rs:373, anchor not in the edges PK ->
  multi-anchor-same-target collides) logged for my Wave-3 graph fix. Acked B.

### Wave-2 state: converging, NOT at barrier

A MM (c854d3f8), B MM (9349dba2), D GG-holding (survey transport, sequences with
C's chan-server survey work), C ~~ (survey SPA + reply route + team-agent field +
the type_complexity fix). Long pole = C. Barrier duties queued: sequence the C+D
chan-server survey merges (resolve dead_code allows + router() + type_complexity
on the merged HEAD), run the full gate + the two browser smokes, then the refresh
handshake with @@Host. My own Wave-2 coding is complete.

### Wave-2 BARRIER -- CLOSED + merged

All four lanes done. C finished last (survey SPA + reply route + team-agent +
the team_config type_complexity fix). The survey is a HARD C+D coupled unit
(chan-shell <-> chan-server <-> SPA mutually dependent; lib.rs / client.ts /
routes mod.rs co-edited), so it cannot split into compiling sub-commits ->
landed as ONE coherent barrier commit 08d7435b (37 files, +2425/-179).

Sequencing: A (c854d3f8) + B (9349dba2) already on main from earlier in the
wave. Ran the authoritative full gate on the coherent integrated tree BEFORE
committing (not trusting C/D's self-gates): cargo fmt (auto-fixed one rustfmt
diff in D's wire.rs) -> fmt --check 0; clippy --all-targets -D warnings 0 (incl.
C's type_complexity fix + D's dead_code allows, which clippy did NOT flag as
needless even with C's route now using them); cargo test 0 (528 chan-workspace +
the survey bus/route/followup suites + 11 team_config, zero failures, confirmed
by NAME not just exit code); build --no-default-features 0; svelte-check 0/0; npm
build 0. Committed via the race-proof auto-generated pathspec (git diff
--name-only + the 5 new code files; verified 0 docs staged, 37 files; post-commit
git show --stat HEAD audited). Tree clean but for untracked round docs.

@@Host's .md-handler call (2026-06-01 "i do not want that, at all"): the tree
already adds NO bundle.fileAssociations (Option B), so it is aligned; the carved
round-4 file-handler task is DROPPED. Recorded in status.

Wave-2 BROWSER-UNVERIFIED (carried to Wave 3): click-to-caret, [[ stuck bubble,
survey SPA render+reply. Shared-browser `navigate` was denied to B + C this wave;
all three are gated-green + source-tested + committed under the
pre-release-merge-unverified norm. Surfacing to @@Host: either re-allow browser
access for a focused pass or verify at the Wave-3 full smoke / desktop verify.

Flipped status -> Wave 3 active + Wave-3 per-lane scope + the A<->B graph touch
point. Refresh handshake to @@Host next.
