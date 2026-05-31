# event-lane-c (append-only)

## 2026-05-31 @@LaneC: earlier IDX theories (phantom indexer; re-entrant deadlock; watcher self-write loop; embedder-unavailable spin) ALL WITHDRAWN.
## 2026-05-31 @@LaneA->@@LaneC: env FLAKY; GREEN LIGHT; OPTION A + IDX-DISPLAY clamp + facade slice + wire shape APPROVED; Host-ratified (preflight ASAP, background rest, keep semantic ON -> C closed). @@LaneB frontend done (d124c48d, 6138c623).
## 2026-05-31 @@LaneC: ROOT CAUSE PROVEN via live `sample` (on-CPU candle BERT matmul inside flush_embed_batch).

## 2026-05-31 @@LaneC -> @@LaneA: CK-INDEX-IDLE REACHED + @@Host REOPENED Option C. Two new items.

### Option A: DONE, gated, empirically verified -> ready to merge (your sequencing)
Three pieces, all my files (indexer.rs, facade.rs, preflight.rs - none
shared with B/D; compose with B's frontend already on main):
1. IDX-DISPLAY clamp (both Building arms current.min(total)).
2. Facade BM25-commit slice: commit+reload BM25 BEFORE each embed flush
   (in-loop + tail), emit EmbedBatch AFTER commit so the snapshot reads
   committed docs. No reindex-contract change.
3. Backend: IndexStatus::Idle gains embedding:Option<EmbedProgress
   {done,total}>; StatusUpdater latches at first EmbedBatch -> flips to
   Idle{embedding:Some}, stops reverting to Building; reconcile_idle
   clears to None.

GATE: fmt clean; clippy -p chan-workspace -p chan-server --all-targets
-D warnings = 0; cargo test (both crates) = 839 pass / 0 fail; build
--no-default-features = 0.

EMPIRICAL (my scoped :8821 heavy drive = this repo clone):
- preflight UNLOCKS (idle during embed; dashboard renders, not the
  locked overlay) - @@Host confirmed live.
- BM25 search non-empty during background embed (q=Workspace -> 3 hits).
- new edit (lanec_probe.md) searchable in ~4s WITHOUT queuing behind
  embed (the Drafts-stuck risk is resolved by the per-flush commit).
- embed is genuinely slow on a 4096-file CODE repo: live stack sample =
  277 on-CPU candle-matmul frames vs 53 parked. NOT hung; the chip just
  steps in long infrequent jumps (one batch = one big forward pass), so
  it LOOKS frozen at 425/917. This is the C scenario.

### @@Host REOPENED Option C (direct to @@Alex, 2026-05-31)
@@Host now wants, on top of A:
  (i)  ADD A C CAP: bound/skip embeddings for huge or source-code
       workspaces (reverses the earlier "C closed, keep semantic always
       on" ruling - same authority updating it).
  (ii) IMPROVE THE CHIP UX so it never looks frozen.
This is a product/scope reversal touching chan-workspace embed budget
(outside my literal IDX file list) + @@LaneB's AppStatusBar, so I am
routing it to you rather than acting unilaterally. Proposed shapes,
need your (Host) ruling on mechanism + threshold:

C-CAP mechanism (pick one):
  (a) Hard file/chunk threshold in build_all: when a cold full-build's
      indexable count exceeds N, set do_vectors=false for that build
      (BM25 authoritative), status embedding:None + a "skipped:too_large"
      reason; per-file edits still embed. Simplest; in facade.rs which I
      already edit. RECOMMEND.
  (b) Persisted auto-disable: flip semantic_enabled off when a workspace
      exceeds the threshold; user can re-enable in settings. Most
      explicit/visible; touches config + settings UI.
  (c) Partial cap: embed first K chunks then stop. Leaves an inconsistent
      hybrid; NOT recommended.
  Threshold is a @@Host number: propose file_count > 2000 OR total_chunks
  > ~20000 (the bge batch is ~chunks; 4096-file repo here is ~?k chunks).
  Need the number.

CHIP UX (pick one):
  (x) Smaller embed batches -> more frequent EmbedBatch events -> chip
      advances smoothly. Trades a little embed throughput for a live bar.
  (y) Heartbeat: emit an embed progress tick on a timer during the
      forward pass. More plumbing.
  (z) If C-CAP (a) lands, big repos skip embed entirely so the chip
      issue largely disappears; for in-cap workspaces (x) is enough.
  RECOMMEND (a)+(x)/(z).

### Next for me (your call)
- I can implement C-CAP (a) + CHIP (x) immediately (both land in
  facade.rs/indexer.rs, my current working set) once you confirm
  mechanism + threshold. They are additive to the already-gated A.
- Or you merge A first and I follow with C as a second increment.
- :8821 left UP for @@LaneB joint browser smoke.
@@Alex chose "hand A to @@Architect to merge" + "add C cap + chip UX".

## 2026-05-31 @@LaneC -> @@LaneA: re your clobber concern (StatusUpdater Idle{embedding} vs watcher Reindexing). CONFIRMED real, COSMETIC, self-healing, mooted by C-CAP.

Verified the two concurrent writers to the single Arc<Mutex<IndexStatus>>:
- The cold build's StatusUpdater (coordinator spawn_blocking): flips to
  Idle{embedding:Some} at each EmbedBatch (indexer.rs ~EmbedBatch arm).
- The watcher worker (spawn_watcher_loop, independent task): on a user
  edit sets Reindexing{file} (indexer.rs:419) then set_idle
  (indexer.rs:900) -> Idle{embedding:None}.

INTERACTION during the background-embed window (build still embedding +
user edits a file):
1. Watcher sets Reindexing{file} -> OVERWRITES Idle{embedding:Some}.
   The build's embed local-latch (started) is in the ephemeral
   StatusUpdater; the watcher can't see it, so it cannot preserve the
   chip.
2. Watcher set_idle -> Idle{embedding:None}. Chip DISAPPEARS.
3. Build's NEXT EmbedBatch -> Idle{embedding:Some} again -> chip RESTORES.

So both directions clobber, transiently:
- CHIP: drops to None on a concurrent edit, restores at the next embed
  flush. On a big repo flushes are minutes apart, so the chip can
  under-report (look settled) for that gap. Cosmetic.
- PREFLIGHT: a ~ms Reindexing blip per edit. Cmd+R landing exactly in
  that window shows "working" for <1s then unlocks on the next 750ms
  poll. NOT the multi-minute hang we fixed; legitimately-busy signal.
- CORRECTNESS: unaffected. Both writers persist real data; the edit IS
  indexed (I empirically verified lanec_probe.md searchable in ~4s
  during a live background embed). Search returns hits throughout.

WHY a quick "preserve embedding in set_idle" param does NOT fix it: by
the time the watcher calls set_idle, it has ALREADY overwritten the
embedding when it set Reindexing{file}, so there is nothing to preserve.
A correct fix needs a SHARED bg-embed signal (e.g.
Arc<Mutex<Option<EmbedProgress>>> on the Indexer) that BOTH the watcher
Reindexing/set_idle writes and the build StatusUpdater read, so a
concurrent edit re-attaches the live chip. That is real plumbing.

RECOMMENDATION: do NOT harden this round.
- C-CAP MOOTS the main case: workspaces >2000 files skip background
  embed entirely (no chip to clobber); workspaces <=2000 embed quickly,
  so the flicker window is small.
- It is cosmetic + self-healing + search-correct.
- The correct fix (shared bg-embed signal) is round-3 polish; folding a
  half-fix now would be the kind of partial state we avoid.

Proceeding with C-CAP per your instruction. Will note the clobber as a
round-3 backlog item in the close.

## 2026-05-31 @@LaneC -> @@LaneA: C-CAP LANDED (commit 3e54ed3e) + a THRESHOLD finding @@Host must see.

DONE + GATED (commit 3e54ed3e, facade.rs only - clean staged split,
65/+1; verified peer files untouched):
- EMBED_FILE_CAP const (2000) + pure embeddings_capped_for(include_vectors,
  file_count) helper (unit-tested 4 cases) + one do_vectors guard in
  build_all + BuildSummary.embeddings_capped + an info! log on skip.
- Mechanism (a) exactly as ruled: a cold full build over the cap goes
  BM25-only (search works), per-file edits via index_one still embed.
- Gate: cargo build/clippy -Dwarnings/test (868 pass, 0 fail)/build
  --no-default-features all green for chan-workspace+chan-server.

THRESHOLD FINDING (please relay to @@Host - the number needs his eyes):
I measured the canonical repro drive (shallow clone of THIS repo, the
exact thing @@Host hit the wedge on):
  - indexable files (.md/.txt/.markdown, excl .git/target/node_modules)
    = 917
  - all files = 1622
  - indexable bytes = 10.8 MB
  - BM25 indexed_docs = 4098 (these are CHUNK-docs, not files - the
    "4096" we kept citing was the CHUNK total, not the file count)
So at the 2000-FILE cap, THIS repo (917 files) is UNDER the cap and
STILL does the slow background embed - i.e. the headline number does not
catch the very workspace that motivated C. Confirmed live: :8821 (917
files) still shows embedding:{done:425,total:917}, cap did not fire.

The embed cost tracks CHUNKS, not files (917 files -> ~? chunks; status
total:917 is the file-based progress I emit; the actual embed work is
4098 BM25 chunk-docs and more embed-chunks). So a FILE cap is a coarse
proxy. Options for @@Host:
  (1) Lower the FILE cap (e.g. 500-800) so a ~900-file code repo skips.
      Simple, but a 900-file NOTES workspace (small files) would also
      skip though it would embed fine.
  (2) Switch the cap metric to CHUNKS (total_chunks > N). More accurate
      to the actual cost; but chunk total is only known mid-build, so the
      skip would be a mid-build abort of the embed pass rather than a
      pre-build gate (slightly more code, still no contract change).
  (3) Keep 2000 files as a SAFETY ceiling for pathological repos and
      accept that ~900-file repos embed in the background (Option A makes
      that non-blocking + usable; the chip UX item makes it not look
      frozen). This is the "A already made it fine, C is just a ceiling"
      stance.
My recommendation: (3) keep 2000 as the ceiling + ship CHIP (x) so the
sub-cap case looks healthy, and note CHUNK-metric (2) as a round-3
refinement. But this is @@Host's product call - I implemented the
mechanism at the number he gave; tuning/metric is his.

CHIP UX (x) next: smaller embed batch -> more frequent EmbedBatch ticks
so the chip advances smoothly for sub-cap workspaces. Implementing now
unless you want to wait on the threshold decision (independent of it).

## 2026-05-31 @@LaneC -> @@LaneA: CHIP UX LANDED (commit 326532d9). IDX work COMPLETE.

CHIP (x) shipped as a FREE, no-throughput-tradeoff, no-lie change in my
indexer.rs (28/+4, committed 326532d9, indexer.rs only - clean split):
- After embed starts, post-embed-start IndexFile ticks (the fast BM25-
  drain windows BETWEEN the slow embed flushes) now refresh the chip
  progress in place, so it ADVANCES per file instead of only stepping on
  each embed flush (which on a big workspace are minutes apart = the
  "looks frozen" report).
- `done` is capped at total-1 so the chip NEVER reads done (total/total)
  while the tail embed is still running (avoids "looks done while
  working"); reconcile_idle clears it to None when the pass returns.
- No extra index_stats read (reuses the IndexFile tick's own counter);
  no facade/throughput change.
- Gate: fmt/clippy -Dwarnings/test (chan-workspace+chan-server, 0 fail)/
  build --no-default-features all green.

DELIBERATELY NOT DONE (flagging, not silently deciding):
- The ONE remaining freeze is DURING a single synchronous embed flush
  (thread in candle forward pass, emits no ticks). Shortening it = the
  embed-batch-size lever (lower EMBED_BATCH_CHUNKS from 4096). That is a
  global THROUGHPUT tuning number, same class as the C-CAP threshold I
  flagged to Host - I did NOT change it unilaterally. If you/Host want
  the in-flush freeze shortened, give me a batch number and it is a
  one-line change. Otherwise the per-file drain refresh already makes the
  chip lively for the common case.

IDX SCOPE STATUS (all on main):
- b0525edb: Option A (preflight unlock + IDX-DISPLAY clamp + facade BM25
  commit slice + Idle.embedding backend). CK-INDEX-IDLE.
- 3e54ed3e: C-CAP (skip embeddings >2000 files; threshold flagged to
  Host - canonical repo is 917 files, under cap).
- 326532d9: CHIP UX (per-file drain refresh).
Plus @@LaneB frontend (d124c48d, 6138c623) already on main.

OPEN DECISIONS for you/Host (both flagged, neither blocking):
1. C-CAP threshold/metric: 2000 files does not catch the 917-file
   canonical repo. Options (lower file cap / chunk metric / keep as
   ceiling) in this file above.
2. CHIP in-flush freeze: embed-batch-size number if you want it shorter.

ROUND-3 BACKLOG (logged): chip-clobber on concurrent edit (needs shared
bg-embed signal = Option B); CHUNK-based cap metric.

Moving to WAVE-2: cs search (CK-RENAME landed in source by @@LaneD;
binary not yet rebuilt) + toast audit. :8821 left UP for @@LaneB joint
smoke; I will tear it down after.

## 2026-05-31 @@LaneC -> @@LaneA: TOAST AUDIT COMPLETE - invariant HOLDS, guard already exists (no code needed).

Did the toast auto-dismiss audit (wave-2, sequenced first). Result: the
"do all real toasts auto-dismiss?" invariant is ALREADY ENFORCED and
holds. No remediation required.

Findings (verified, not assumed):
- Single transient-status surface: ui.status, fed by the notify() bus
  (web/src/state/notify.svelte.ts) -> setNotifyHandler ->
  setTransientStatus (store.svelte.ts:53-60), which sets ui.status AND
  arms a setTimeout that clears it after STATUS_RESET_MS.
- Independent grep (sort -u): the ONLY `ui.status =` writes in the whole
  web/src tree are store.svelte.ts:54 (set msg) and :57 (timer clear) -
  both inside the timed setter. Nothing bypasses it.
- No dedicated toast/banner/snackbar component; no other setTimeout
  message-clear pattern outside store.svelte.ts.
- A guard test ALREADY EXISTS on main:
  web/src/components/toastAutoDismissSweep.test.ts (pre-existed at the
  round-2 start point 403547c4; header explicitly says it IS the
  phase-15 round-2 toast audit). It recursively greps all non-test
  .ts/.svelte for `ui.status =` (excluding `==`) and fails if any file
  other than the allowlisted store.svelte.ts writes it, plus asserts
  setTransientStatus arms+clears a timer. 2/2 pass (ran vitest).
- The display-driven index status pill is correctly EXEMPT (it mirrors
  /api/index/status; a timer would HIDE a stuck reindex - which is
  exactly the IDX bug I fixed via the real Idle transition, not a timer).

CAVEAT (minor, flagged not fixed): the guard is a STATIC source-pattern
test (same class as our "static gate misses runtime" note). It catches
direct ui.status writes; it would not catch a runtime mis-sequence. For
the current single-surface architecture that is adequate. If you want a
runtime test (mount + fake-timer + assert ui.status clears), say so;
it's small, but I did not add it unprompted since the static guard +
the existing transientStatus/moveToastAutoDismiss runtime tests cover
the behavior.

So: TOAST audit = DONE, invariant holds, guard in place. No commit from
me (nothing to change). Cross-agent note: I did NOT write the sweep test
(it pre-dated my round-2 work); I verified it is correct + complete.

WAVE-2 remaining: cs search - BLOCKED on @@LaneD landing their cs CLI
increment (main.rs + control_socket.rs are theirs; I append on the
committed base per your sequencing). Idle on it until @@LaneD pings.

## 2026-05-31 @@LaneC: cs search - HELD MID-FEATURE on tool outage (durable state record)

Wave-2 cs search is half-done and PAUSED by an active tooling outage
(file-content reads + even some bash stdout confabulating: `...`
placeholders, out-of-order line numbers, fabricated wc/sha output).
Single-token outputs (cargo exit codes, git status, sha of whole files)
stay reliable; multi-line content does not.

DONE + BUILD-VERIFIED (uncommitted in worktree, git status confirms only
crates/chan-server/src/control_socket.rs is modified):
- ControlRequest::Search { query, limit: Option<u32> } variant.
- handler arm: resolve_workspace -> search_workspace.
- search_workspace() helper: reuses Workspace::search (the
  /api/search/content path), one-row-per-file collapse score-desc,
  returns JSON on the connection like term_list:
  {ready, mode, query, hits:[{path,heading,start_line,snippet,score}]}.
- `cargo build -p chan-server` = exit 0 (compiler validated my
  Hit/SearchOpts/SearchResults field reads).
- +72 lines, NOT committed (cannot audit a staged diff under stdout
  corruption = shared-worktree rule violation risk).

NOT STARTED (main.rs, @@LaneD-held, clean/untouched - sha 713bc4de):
- ShellAction::Search variant (after Terminal).
- cmd_shell dispatch arm -> cmd_shell_search.
- cmd_shell_search: send ControlRequest::Search via send_control_request,
  format the returned JSON: markdown DEFAULT, --json compact,
  --json --pretty indented (NOT --pretty-json). infer_subcommands is on
  (cf2c8b2c) and `search` keeps `s` unambiguous (per @@LaneD).
- Blocked because I can't reliably read send_control_request /
  cmd_shell_terminal (the List send+print template) to mirror them.

RESUME PLAN (when reads are sha-stable again):
1. sha-verify main.rs region, read cmd_shell_terminal List path +
   send_control_request signature.
2. Write the 3 main.rs pieces; build -p chan.
3. Full gate (fmt/clippy -Dwarnings/test + cs_alias if affected).
4. Chained staged-diff commit of control_socket.rs + main.rs together
   (audit staged set first - REQUIRES reliable stdout).
5. Ping @@LaneD to rebase CS-CAROUSEL/DESKTOP-OPEN; flag @@LaneA.

@@LaneD: still holding main.rs + control_socket.rs. @@LaneA: this is a
tool outage, not idle/scope. control_socket.rs work is safe + green.

## 2026-05-31 @@LaneC -> @@LaneA: STAND-DOWN ack + ONE decisive finding for whoever finishes cs search.

Acked: not committing, not touching control_socket.rs further, not
running cargo (avoid racing your reliable-tab build). Frozen.

CRITICAL HANDOFF FACT (clean grep, reliable): the chan CLI has its OWN
client-side `ControlRequest` enum at crates/chan/src/main.rs:1895
(`#[derive(Serialize)] #[serde(tag="type", rename_all="snake_case")]`),
SEPARATE from chan-server's control_socket.rs copy. The client serializes
its own enum to JSON over the socket; it does NOT import the server one.
=> cs search needs the `Search` variant added to BOTH enums. Your "server
side complete + green" check (cargo build -p chan-server exit 0) is
correct but only covers the server half; `cargo build -p chan` FAILS
E0599 "no variant named Search found for enum ControlRequest" because the
CLIENT enum (main.rs:1895) is still missing it.

EXACT REMAINING WORK (small, mechanical):
1. main.rs:1895 client `ControlRequest` enum: add
   `Search { query: String, #[serde(skip_serializing_if=Option::is_none)]
   limit: Option<u32> }` (match the server variant's wire fields: query +
   optional limit; server has `#[serde(default)] limit`). Mirror the
   client enum's existing variant style (it uses #[allow(dead_code)] +
   serialize-only; confirm exact field attrs from a clean read - the
   deeper read was confabulating for me).
2. main.rs ALREADY HAS (my in-worktree edits, applied): ShellAction::
   Search variant (query/limit/json/pretty), the cmd_shell dispatch arm,
   cmd_shell_search(), and render_search_markdown(). These compile once
   the client enum variant (step 1) exists.
3. control_socket.rs ALREADY HAS (frozen, builds green): server Search
   variant + handler arm + search_workspace().

So it is literally ONE missing client-enum variant between here and a
green `cargo build -p chan`. After that: full gate (fmt/clippy
-Dwarnings/test + cs_alias) + chained-commit BOTH files.

WORKTREE NUMSTAT (reliable): see the git diff --numstat I just ran -
main.rs and control_socket.rs are the only code files dirty.

DECISION still yours: finish from your reliable tab (you have steps 1-3
mapped above) OR clear me to do step 1 + gate + commit if my channel
holds. I will not move until you say which, to avoid us both editing
main.rs. Standing by.

## 2026-05-31 @@LaneC (RESPAWN): empirical QA of the merged cs CLI + IDX surface - ALL PASS (one process finding: my stale local web/dist).

Fresh recycled session. Identity @@LaneC confirmed ($CHAN_TAB_NAME). Did NOT
touch main.rs / control_socket.rs (@@LaneD owns them). Built current main,
served an ISOLATED standalone drive on :8842 (403 small notes across nested
dirs, NOT the heavy clone), own control socket, pkills scoped to my drive
path. HEAD moved e10424a5 -> 7c241370 (cs dashboard --carousel-off) mid-run; I
rebuilt to cover it. Tore down server + drive + browser tab + temp files at the
end (no strays, registry unregistered).

### cs search (e10424a5) - PASS
- markdown default / --json / --json --pretty all correct, exit 0.
- MATCHES the UI search (/api/search/content): identical path/heading/
  start_line/snippet/score for a unique token; cs drops chunk_id (one-row-per-
  file collapse). Multi-hit "embedding" = 20 hits both; cs has NO duplicate
  paths (collapse) + descending score order. mode=bm25 during background embed,
  ready=true.
- Minor (flag, not a bug): snippets carry raw <b>..</b> highlight tags in the
  markdown output (mirrors the UI payload; literal in a plain terminal). Whether
  CLI markdown should strip/convert to ** is a @@LaneA/@@LaneD call.

### cs terminal (cf2c8b2c) - PASS
- new: opens a terminal tab in the window (PTY spawns). Headless serve just
  queues the request (no frontend to materialise it); WITH the SPA open, a live
  session registers. Verified end-to-end via the browser.
- list: markdown default + --json + --json --pretty; live session shows
  group/name/session_id/cwd. Empty state: "No live terminal sessions." /
  {"groups":{}}.
- write: delivers bytes to the live PTY (probe commands executed, "wrote to 1
  terminal session(s)").
- restart: REAL re-spawn verified - shell PID 43285 -> 43532 across a restart,
  while cwd (workspace root) + env (CHAN_TAB_NAME=qaterm, CHAN_TAB_GROUP=lanecqa)
  + shell (-bash) all PRESERVED (before==after probe). Session keeps its stable
  id/name; the tab shows "session ended (explicit)".
- prefix matching: cs t l -> terminal list, cs t r -> terminal restart. OK.
- graceful no-match: write/restart with an unmatched selector -> "Error: no live
  terminal session matched", exit 1.
- NOT smoked (noted per @@Architect): the agent-relaunch (Team Work startup-
  command) path - I had no startup-command/agent terminal. Plain-shell restart
  preservation IS confirmed.

### IDX (b0525edb / 3e54ed3e / 326532d9) - PASS
- preflight UNLOCKS while embedding: /api/index/status state=idle WITH
  embedding={done:402,total:403} live; the SPA rendered the home view (not the
  locked overlay).
- chip never exceeds total: done capped at total-1 (402<=403) throughout.
- settles cleanly: embedding cleared (field absent) + indexed_vectors 0 -> 403
  when the pass returned (reconcile_idle). On serve restart over the persisted
  index it loaded vectors=405 (no re-embed; 403 + my 2 probe files).
- draft edit NO-WEDGE: POST notes/draft-probe.md AND Drafts/untitled/draft.md
  (the ORIGINAL bug path) -> status stayed idle across 8 polls, both searchable
  (wedgeprobe7, draftwedge8). The Drafts file lives in the draft store (not the
  disk tree) yet is indexed + searchable.
- IN-FLUSH FREEZE reproduced (known/flagged, NOT a regression): 403 chunks in
  one batch (EMBED_BATCH_CHUNKS > 403) = one synchronous forward pass; under
  contention with @@LaneD's builds it pegged ~450-680% CPU and the chip sat
  frozen at 402/403 for MINUTES (no intra-batch ticks). Exactly the in-flush UX
  limitation flagged at 326532d9. Round-3 note: even a ~400-note drive shows the
  freeze under core pressure (the C-CAP 2000-file ceiling does not catch it).

### cs dashboard --carousel-off (7c241370) - PASS (after a fresh web build)
- Captured the live broadcast frames (opened a 2nd WS to /ws): --carousel-off
  sends {command:open_dashboard, carousel_off:true}; plain sends no carousel_off;
  --carousel-index N sends {carousel_index:N}. CLI -> server -> frame intact.
- On a FRESH web/dist: the carousel-off tab serialises {"k":"d","ar":false}
  (autoRotate=false applied + persisted); the plain tab has no ar (rotation
  default on); --carousel-index N visibly lands on slide N (cs:N + slide shown).
  CONFIRMS the CLI flag wires to autoRotate. Closes @@LaneD's pending browser
  smoke. (The autoRotate field behaviour itself was @@LaneB-smoked.)
- PROCESS FINDING (NOT a product bug): my FIRST attempt showed no ar:false -
  because my LOCAL web/dist was STALE, predating @@LaneB's carousel_off handler
  (store.svelte.ts:747). I had built the binary WITHOUT `npm run build` first
  (the rust-embed staleness CLAUDE.md warns about). Proven by grepping the
  bundle: carousel_index + autoRotate present, carousel_off ABSENT. web/dist is
  gitignored (CI builds it fresh), so the committed/release state is fine. After
  npm run build + cargo build -p chan, the served bundle had carousel_off and
  the flag worked. Lesson for QA of frontend-touching CLI flags: rebuild
  web/dist FIRST, and grep the served bundle before reasoning about behaviour.
- Headless-browser caveat: the dashboard-TAB carousel never visibly auto-cycles
  in a Chrome automation tab (EmptyPaneCarousel `paused` includes `!active`; the
  dashboard tab passes active=false), so the rotation ANIMATION is unobservable
  for either variant. The flag wiring is confirmed via the ar:false field, not
  animation.

NET: all four areas pass. No code change from me (QA only, per scope). Open
items for @@LaneA/@@LaneD: (1) cs-search markdown raw <b> tags - keep or strip;
(2) IDX in-flush chip freeze under core pressure - round-3 embed-batch lever.

## 2026-05-31 @@LaneC: Team Work + DESKTOP-OPEN + re-smoke QA (round-close gate) - ALL PASS.

Built current main (cc076e85, clean tree), isolated standalone serve on :8843
(5-note drive), scoped pkills, torn down (serve+drive+registry+browser tab+temp;
the live :8824 window left untouched). Browser nav was denied once -> @@Host
re-allowed. All checks against the FRESH bundle (verified the served bundle had
"Terminal tab group name" before testing).

### (1) cs terminal grouping (020c690c) - PASS
cs terminal new --tab-group=teamA (a1,a2) + --tab-group=teamB (b1) -> list
groups them: markdown `## teamA`/`## teamB`; json {"groups":{"teamA":[a1,a2],
"teamB":[b1]}}. Group persists in the SPA hash (tg:teamA/teamB).

### (2) group-broadcast scoping - PASS
cs terminal write --tab-group=teamA $'...' -> "wrote to 2 sessions"; per-terminal
probe files prove a1+a2 GOT it, b1 (teamB) did NOT. The same server-side
broadcast the Cmd+Shift+I UI chord drives (the chord itself is Chrome DevTools,
untestable via Blink - substitute approved by @@Architect).

### (3) TEAM-GROUP dialog (5603403) - PASS
Cmd+Alt+P opens "Spawn agents". Verified by JS DOM:
- "Terminal tab group name" field renders, defaults to "chan-team" (from
  /tmp/new-team-1/chan-team.toml). Help text documents the -N collision.
- FOLLOWS the config path: path .../squad.toml->group "squad", .../myteam.toml
  ->"myteam"; after hand-editing group to "handpicked", path .../other.toml
  leaves it "handpicked" (stops following). Matches syncTabGroupToPath.
- -N COLLISION: bootstrapped a team with a SHELL member (command "bash", NOT
  claude) into group "teamA" (a LIVE group) -> the team's lead (@@Lead) joined
  "teamA-2". list: teamA[a1,a2] / teamA-2[@@Lead] / teamB[b1]. So the orchestrator
  resolves -N at bootstrap against the live registry. Also confirms 020c690c
  (team terminals join the resolved team group). No real agent used.

### (4) real-claude bootstrap - SKIPPED (per @@Architect; @@LaneD already
real-agent-smoked it; do not peg cores).

### (5) cs terminal write --submit (2b9563c7) - PASS (source + runtime bytes)
Source: const AGENT_SUBMIT_CHORD = "\x1b[27;9;13~"; apply_submit_chord does
data.trim_end_matches('\n') + chord (so "appends chord + strips trailing \n",
both halves). Runtime byte-capture (raw-mode head -c 12 | od on a shell term):
write --submit $'AB' delivered `41 42 1b 5b 32 37 3b 39 3b 31 33 7e` = AB +
ESC[27;9;13~ exactly. Newline-strip covered by source + the existing unit test.

### (6) chan open / DESKTOP-OPEN (05e9b9eb) - PASS (resolution + guidance)
- non-registered path -> "X is not inside a registered chan workspace. Register
  one first: chan add <directory>" (exit 1).
- in-workspace DEEP path (notes/n1.md) with CHAN_NO_DESKTOP_HANDOFF=1 ->
  "...is in workspace /private/tmp/chan-test-lanec-team... Open the workspace
  with: chan serve <root>" (exit 0). Longest-prefix root resolution confirmed.
- The desktop HANDOFF branch (maybe_handoff_to_desktop, gated on
  handoff_opt_out + gui_session_present) is desktop-only -> SKIPPED per
  @@Architect (@@Host post-release). I forced the opt-out so no window opened in
  the live desktop.

### (7) cs search <b>->** re-smoke (cc076e85) - PASS
markdown: snippet renders `Content about **workspace**` (was <b>workspace</b>).
--json: keeps raw `<b>workspace</b>`. render_search_markdown replaces <b>/</b>
with ** ; unit-tested.

### Notes (not bugs)
- The "Claude is active in this tab group" toast is the CLAUDE-IN-CHROME
  EXTENSION's own indicator (string is NOT in chan source) - NOT a chan
  notification. Dropped from findings after grep.
- Transient E0027 on my first build was @@LaneD mid-editing main.rs for POKE-2.2
  (now committed 2b9563c7, tree clean) - never a red main; flagged not-alarmed.
