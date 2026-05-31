# Round-2 @@LaneC journal - search / indexing

Domain: search / indexing. Coordinate through @@LaneA (architect).
Wave-1 = IDX (indexing-never-completes) -> CK-INDEX-IDLE. Wave-2 = cs
search (gated on CK-RENAME, landed in source by @@LaneD) + toast audit.

## SESSION CONDITIONS (load-bearing)

Env is FLAKY: OUTPUT TRUNCATION + CONFABULATION (model invents
plausible file content under truncation), NOT byte fabrication on disk.
Proven repeatedly: a temp file genuinely 89 lines / hash d8fa1c9e was
"Read" back as a fake 39-line version with `// ...` placeholders.
Discipline (mandated, in force): single atomic commands; no `||`; no
parallel storms; sha-verify disk vs `git show HEAD` before reasoning;
anchor on git status; trust subprocess ground truth (cargo counts,
curl, `sample <pid>`) over file reads. EDITS are safe (they fail on
mismatch, never corrupt) but old_string must be built from hash-verified
content.

## sha-VERIFIED ANCHORS (disk == HEAD blob)
- chan-server/src/indexer.rs        cbcb5c915342f918
- chan-workspace/src/workspace.rs   8f530440...780e
- chan-workspace/src/index/facade.rs 8bc6f49c...b2ee

## IDX ROOT CAUSE - PROVEN (live stack sample, ground truth)

`sample` of the wedged :8820 process (pid 49740): reindex thread 777/777
on-CPU in candle BERT forward/matmul, inside
reindex_with_aggression -> flush_embed_batch ->
Embedder::embed_documents_cancelable -> BertModel::forward -> matmul.
Footprint 2.5G; server log frozen (not spinning); `requesting rebuild`=0.
=> NOT a loop/deadlock/unavailable-spin. The synchronous embed pass over
a 4096-file SOURCE repo never finishes, so reindex_with_aggression never
returns -> coordinator (indexer.rs ~333) never hits Ok(Ok)->set_idle ->
IndexStatus pinned Building{embedding} -> preflight.rs:104 keeps overlay
locked (ready only on Idle). ONE cause, BOTH symptoms (stuck status bar +
Cmd+R hang). Empty drive = 0 chunks -> embed never entered -> instant
Idle (matches @@LaneD's split).

WITHDRAWN theories (all confabulation-derived, none landed on disk):
phantom 66-line indexer; re-entrant index_lock deadlock; watcher
self-write loop; embedder-unavailable infinite warning loop.

## CRITICAL EMPIRICAL FINDING (shapes Option A)

At the embed wedge, BM25 search returns EMPTY:
`curl :8820/api/search/content?q=workspace` -> {"ready":true,
"mode":"bm25","hits":[]} (repeated, stable; also for unique terms).
=> "EmbedBatch transition == BM25-ready" does NOT hold as-is. The embed
flush blocks before the first timed BM25 commit (build_all comment:
BM25 commits every BM25_COMMIT_INTERVAL via commit_and_reload; the embed
flush wedges inside the first interval). So unlocking preflight at
EmbedBatch with no commit would ship a ready-but-empty-search UI.
=> Option A REQUIRES a forced bm25.commit_and_reload() at the
IndexFile->EmbedBatch boundary (the minimal facade.rs touch the
architect pre-authorized).

## DRAFT-EDIT CONCERN (architect refinement, Host-ratified)

Original "stuck reindexing Drafts/..." is the watcher path
(apply_watch_change -> index_draft_file), a separate spawn_blocking from
the coordinator reindex. HYPOTHESIS (needs clean read of bm25.rs writer
lock + drain loop, currently confabulating): the drain loop "single
thread owns the [bm25] writer" and holds it across the embed flush, so a
draft edit's index_one queues behind the multi-minute embed = the
original Drafts stuck-reindex. If true, the isolation slice is: commit+
reload (release) the bm25 writer BEFORE each embed flush, so the writer
is free during embed. This SAME change also fixes the empty-search
finding above (commit makes BM25 visible at the boundary). Two birds.
Pull only this minimal slice; NOT the full B contract rework (round-3).
FLAG to architect before landing the facade slice.

## OPTION A - APPROVED PLAN (chan-server scope + minimal facade slice)

Goal: gate preflight/first-paint on BM25-ready, not the embed phase;
embeddings finish in background; search upgrades bm25->hybrid when
vectors land. Fixes both symptoms, preserves semantic, least risk.

### Status wire shape (I define; @@LaneB builds frontend against it)
Add a background-embed field to the terminal Idle state rather than a new
variant (preflight already maps Idle->ready, minimal churn):

  IndexStatus::Idle {
      indexed_docs: u64,
      indexed_vectors: u64,
      model: String,
      embedding: Option<EmbedProgress>,   // NEW
  }
  struct EmbedProgress { done: u32, total: u32 }  // serde camelCase to match SPA

- embedding = Some{done,total} while background embeds run; None when
  fully settled (vectors done).
- ready/preflight: Idle -> ready regardless of `embedding` (unlock at
  BM25-ready). PreflightOverlay: no change.
- AppStatusBar: Idle.embedding=Some -> passive "embedding N/M" chip
  (NON-blocking, not the stuck "reindexing" pill); =None -> fully idle.
- Building/Reindexing unchanged (real foreground passes still gate).

### Backend (mine, chan-server)
1. indexer.rs IndexStatus::Idle gains `embedding: Option<EmbedProgress>`;
   all existing Idle constructions set embedding: None (set_idle,
   reconcile_idle, spawn initial).
2. indexer.rs StatusUpdater: on the FIRST EmbedBatch event of a pass,
   set status = Idle{live stats, embedding: Some{done,total}} (BM25 is
   committed by the facade slice at this point). On subsequent EmbedBatch,
   update embedding progress only. On pass return, reconcile_idle sets
   embedding: None.
3. preflight.rs: confirm Idle->ready ignores embedding (likely no code
   change; add/verify a test).
4. IDX-DISPLAY clamp: Building.current = current.min(total) at both
   StatusUpdater Building arms (display-only; 4097/4096 -> 4096/4096).

### Facade slice (minimal, in-scope, FLAG first)
- In build_all drain loop: bm25.commit_and_reload() at the
  IndexFile->EmbedBatch boundary (before the first embed flush) AND
  ensure the bm25 writer is not held across the embed flush, so (a) BM25
  is searchable when we report ready and (b) draft edits don't queue
  behind embed. NO reindex-contract change.

### Frontend (offer to @@LaneB)
- web/src/api/types.ts: IndexStatus Idle gains embedding?: {done,total}.
- AppStatusBar.svelte: passive embedding chip when Idle.embedding set.
- PreflightOverlay.svelte: no change (gates on ready) - verify only.

### CK-INDEX-IDLE verify (heavy content drive, my own :8821)
- preflight overlay unlocks (Cmd+R works) while embeds still run.
- status bar shows passive "embedding" chip, not stuck "reindexing".
- BM25 search returns hits during background embed (the empty-search
  finding is fixed).
- a NEW draft edit indexes into BM25 + is searchable WITHOUT queuing
  behind the embed (architect's serial-queue check).
- eventually settles to fully Idle (embedding None) when vectors land.

## CROSS-LANE
- @@LaneD: CK-RENAME landed in source (cs term->cs terminal). Binary
  still `term` until rebuilt; pokes use `cs term write` meanwhile.
  wave-2 cs search builds under `cs terminal`.
- @@LaneB: idle, offered for IDX frontend. Accepting (see event file):
  they take types.ts + AppStatusBar chip + PreflightOverlay verify
  against the wire shape above; I take backend + facade slice + verify.

## 2026-05-31 wave-2 status

### IDX (wave-1) - COMPLETE + validated
On main, verified chain: b0525edb (Option A: preflight unlock on
BM25-ready + IDX-DISPLAY clamp + facade BM25-commit-before-embed slice +
Idle.embedding backend) -> 3e54ed3e (C-CAP: skip embeddings >2000 files)
-> 326532d9 (chip advances per-file during drain). + @@LaneB frontend
(d124c48d, 6138c623). CK-INDEX-IDLE reached. @@LaneB ran the joint
integration smoke on their own :8822 against a fresh <cap drive and
confirmed my real emit == their consumer shape end-to-end (state stays
idle during bg embed, embedding:{done,total} camelCase done<=total,
preflight unlocks, chip clears on settle). All gated green.
Open (non-blocking, Host's): C-CAP threshold 2000 kept as a ceiling
(canonical repo is 917 files, under it - A already makes that fine);
embed-batch size left as-is (chip fix handles smoothness).

### TOAST audit (wave-2, sequenced first) - COMPLETE
Invariant holds; no code needed. ui.status is the single transient
surface, written only by setTransientStatus (store.svelte.ts:54/:57)
which arms a clear timer; notify() bus routes through it; no bypass, no
separate toast component. Guard test toastAutoDismissSweep.test.ts
already on main (pre-dated my work, 2/2 pass) enforces it. Index pill
correctly exempt (poll-driven). Caveat: guard is static-only. I verified
it is correct + complete; did not write it.

### cs search (wave-2) - BLOCKED on @@LaneD
Appends ShellAction::Search (main.rs) + ControlRequest::Search
(control_socket.rs), both @@LaneD files. Per @@Architect sequencing I
wait for @@LaneD to land their cs CLI increment + ping, then append on
the committed base (no concurrent edits). CK-RENAME already landed
(1b39832b). Idle until poked. Spec: reuse Workspace::search via
routes/search.rs; markdown default, --json compact, --json --pretty
indented (NOT --pretty-json); workspace-wide.

## RESPAWN: empirical QA of the merged cs CLI + IDX surface (2026-05-31)

Recycled fresh after the tooling outage. cs search shipped as e10424a5
(@@LaneD committed it + added the one missing client enum variant). New
assignment from @@Architect: walk the merged surface end-to-end on an
isolated :8842 standalone drive (403 notes, not the heavy clone). Result:
ALL FOUR areas PASS - cs search, cs terminal (new/list/write/restart +
prefix + formats), IDX (preflight-unlock-while-embedding, chip cap, clean
settle, draft no-wedge incl the Drafts bug path), and cs dashboard
--carousel-off. Full evidence in event-lane-c.md.

Two things worth carrying: (1) cs terminal RESTART is a real re-spawn -
shell PID changed 43285->43532 with cwd+env+shell preserved. (2) The
--carousel-off check first looked broken because my LOCAL web/dist was
stale (predated @@LaneB's carousel_off handler); I had skipped npm run
build before cargo build. Grepping the served bundle (carousel_off ABSENT)
diagnosed it as my stale artifact, NOT a product bug - web/dist is
gitignored so CI/release builds it fresh. After a real web rebuild the flag
serialised ar:false correctly. Anchoring on the bundle grep instead of
confabulating a frontend bug was the discipline win. No code change from me
(QA only). Server + drive torn down.

## Team Work + DESKTOP-OPEN + re-smoke QA (round-close gate, 2026-05-31)

Second QA pass, routed by @@Architect. All 7 checks PASS against current main
(cc076e85): cs terminal --tab-group grouping; group-broadcast scoping (teamA
only, not teamB); the TEAM-GROUP dialog (field renders + defaults chan-team +
follows the config path until hand-edited + -N collision teamA->teamA-2 via a
SHELL-member bootstrap, no real agent); cs terminal write --submit (runtime
bytes captured: AB + 1b5b32373b393b31337e = the chord, + source strip-newline);
chan open (non-registered -> chan add guidance; in-workspace -> longest-prefix
root + chan serve guidance; desktop handoff skipped per Architect); cs search
<b>->** markdown re-smoke (json keeps raw). Skipped the real-claude bootstrap
(cores). Two non-bugs caught by grepping before reporting: the "Claude is active
in this tab group" toast is the claude-in-chrome EXTENSION's indicator (not in
chan source), and a transient E0027 was @@LaneD's mid-edit main.rs WIP (now
committed). Server+drive+browser-tab torn down. Last QA gate before round-close.
