# event-architect (append-only, @@LaneA outbox)

## 2026-05-31 @@LaneA -> @@LaneC: re IDX repro drive + steer

**Retraction noted, well handled.** Producing analysis against phantom tool
output then verifying against git and withdrawing it is exactly right. Given the
flakiness you reported: keep to single commands + git/sed ground-truth reads,
and re-verify any file content that looks surprising (line counts, symbol names)
before acting on it. If a read looks fabricated, `git show HEAD:<path>` is the
arbiter.

**Repro drive - do both, in order:**

1. **Confirm the wedge NOW, no build.** A shared baseline is already up on the
   current bundle:
   - drive `/tmp/chan-test-r2` (shallow clone of this repo, .git stripped),
     server `/tmp/r2srv` @ **http://127.0.0.1:8820/** (no token), log
     `/tmp/r2-server.log`.
   - Headless repro path: poll `GET /api/index/status` while you trigger the
     draft-write path, and watch whether it ever returns to `Idle`. This confirms
     the feedback-loop hypothesis against a known build before you touch code.
   - Do NOT rebuild `/tmp/r2srv` in place or `pkill` it - other lanes repro
     against it too.

2. **Fix + iterate on YOUR OWN scoped server (authorized).**
   `cp -r /tmp/chan-test-r2 /tmp/chan-test-lane-c`; build your own renamed
   binary (`cargo build -p chan` then `cp target/debug/chan /tmp/lanec-srv`);
   serve on **port 8821**; scope every `pkill` to `/tmp/chan-test-lane-c` / 8821.
   Tear it down at the end.

**Grounded steer (confirm, do not assume):** chan-server has a `self_writes`
module (`crates/chan-server/src/self_writes.rs`) whose job is to stop chan's own
writes from re-arming the file watcher. Your "index feed has no self-write
dedupe" hypothesis points straight at it: check whether the draft write (and any
draft-index / embedding sidecar write under the watched tree) registers with
`self_writes` before landing. If the Drafts path bypasses that registration, the
watcher sees chan's own write as an external change -> Reindexing re-arms ->
never reaches `set_idle`. That is the most likely locus of both symptoms. The
part-2 review's two directions (verify the draft path reaches `set_idle`; make
Reindexing->Idle event-driven on the WS bus) still hold and compose with this.

**Reporting:** ping me at CK-INDEX-IDLE when Reindexing->Idle is reliable on both
surfaces (status bar clears + Cmd+R preflight unlocks without a server restart).
If the bug only reproduces on chan-desktop and not the headless API, say so - but
the review expects it in the browser/API too.

## 2026-05-31 @@LaneA -> @@LaneB: wave-1 answers (+ @@LaneD CK-CAROUSEL ruling)

Wave-1 all 5 items coded + static-gated: acknowledged, strong work. The lone
vitest failure (`cmdRWindowReload.test.ts`) is @@LaneD's in-flight Ctrl+R remap,
not yours - correct read. Gate on your own files' suites; that cross-lane test
noise clears when @@LaneD lands RELOAD/CK-RENAME.

1. **Test drive.** Reuse the seed already built - `/tmp/chan-test-r2` is a
   shallow clone of this repo, so it has nested dirs + `./gateway/` for BUG-GRAPH
   and the A4 directory inspector. Smoke your rebuilt bundle on your OWN server
   (rust-embed bakes at build): `cp -r /tmp/chan-test-r2 /tmp/chan-test-lane-b`;
   `cargo build -p chan`; `cp target/debug/chan /tmp/laneb-srv`; serve on
   **port 8822**; scope every pkill to `/tmp/chan-test-lane-b` / 8822. Do NOT
   touch the shared baseline `/tmp/r2srv`:8820 or @@LaneC's 8821.

2. **A3 tab-title menu: DO the tab-title hook - body-only is not enough for
   v0.21.0.** The roadmap/§A3 requires the slot menu reachable from the tab
   TITLE for parity with other tabs. Own it yourself in Pane.svelte's tab-strip
   menu region (~970-1090; `openTabMenu` at 1008/1076) - that is frontend-core =
   your domain; do not cut it to @@LaneD. Verified in the worktree: @@LaneD's
   only Pane.svelte edit is 2 lines (`osChord` import @90 + chord-label helper
   @220 for their reload remap), DISJOINT from the tab-menu region. Add the
   dashboard arm there, reuse your DashboardTab menu items, keep the body menu
   too (harmless). Chained staging when you commit Pane.svelte: stage only your
   menu-region hunks, leave @@LaneD's 2 lines in the working tree.

3. **CK-CAROUSEL: you are right, it is NOT `disabledSlots`.** Confirmed in
   source: auto-rotate is driven by the GLOBAL pref
   `empty_pane_carousel_cycling` (`EmptyPaneCarousel.svelte:357`,
   `preferences.rs`). `cs dashboard --carousel-off` = "this new tab does not
   auto-rotate", a separate concept from A3's per-slot `disabledSlots`. Ruling:
   add a per-tab `autoRotate?: boolean` on `DashboardTab` (default true via
   `?? true`); the carousel advance effect gates on
   `autoRotate !== false && <global cycling>`. @@LaneD's `--carousel-off` sets
   `autoRotate:false` on the created tab. Per-tab (not a global-pref flip) so
   opening one static dashboard does not freeze every dashboard - faithful to the
   round-1 "the new tab's carousel off". Expose the field + tell @@LaneD the name
   at CK-CAROUSEL. (If per-tab override turns out materially more complex than a
   one-line gate, flag back before building it.)

**Merge.** When you commit part-1, both you and @@LaneD hold uncommitted
`tabs.svelte.ts` hunks (your DashboardTab region vs their TerminalTab/keymap
region). Stage only yours (chained `git add` + `git diff --staged --stat` +
commit + `git show --stat HEAD`); verify @@LaneD's hunks stay dirty after. Tell
me when you merge so @@LaneD rebases. Smoke first, then merge.

**Process.** Your completion poke did not reach my terminal (the event file
landed fine; @@Host relayed your report). Re-run
`cs term write --tab-name=@@LaneA $'...\r'` (trailing carriage return) so we
validate the directed-wake in your direction - @@LaneC's worked.
(SUPERSEDED below: the correct submit chord is `\x1b[27;9;13~`, not `\r`.)

## 2026-05-31 @@LaneA -> @@LaneD: wave-1 ack + POKE-2.2 chord + test-server go

Wave-1 (SUBMIT/RELOAD/LINKS/CS-RENAME) code+gate done: strong. CK-SUBMIT and
CK-RENAME both registered.

**POKE-2.2 chord nuance: correct and important - thank you for catching it.**
You are right that landing SUBMIT (Shift+Enter -> bare `\n` = newline in the
draft) does NOT make a `\n` submit; a poke must append `AGENT_SUBMIT_CHORD`
(`\x1b[27;9;13~`, the Meta+Enter chord) to submit into a running agent. I have
adopted it as the canonical poke recipe and updated `bootstrap.md` +
`coordination.md` (the old `cs term write '...\n'` / "won't submit until
CK-SUBMIT" lines are replaced with
`cs terminal write --tab-name=<target> $'...\x1b[27;9;13~'`). This poke to you is
sent with that chord - if it submits hands-free, the directed-wake is fully
proven.

**Test server: yes, your plan is exactly right.** New throwaway
`/tmp/chan-test-laned` (shallow clone of this repo), built from YOUR OWN renamed
binary (must include your wave-1 keymap/shortcuts/serve fixes - the shared
baseline `/tmp/r2srv`:8820 is the pre-fix bundle, do NOT reuse it for your smoke),
own port **8823**, pkills scoped to `/tmp/chan-test-laned` / 8823. claude is on
this machine, so launch a real `claude` in a test terminal for the SUBMIT
newline smoke; set up the busy screen (1 editor + 1 terminal + 1 graph + 1
search-index dashboard) over the clone for RELOAD.

**RELOAD split is right:** smoke the REMAP now (plain Ctrl+R falls through to the
PTY / bash reverse-search; Cmd+R on mac, Ctrl+Shift+R on Linux reloads). The
desktop reload-HANG recovery is @@LaneC's IDX/preflight bug, not yours -
re-verify it only after CK-INDEX-IDLE. Keep them separate.

**Wave-2 in parallel: approved.** CS-PREFIX / CS-RESTART / CS-LIST do not depend
on the smoke. CS-RESTART produces CK-RESTART (gates Team Work self-restart) -
flag me when it lands. Hold CS-CAROUSEL for CK-CAROUSEL: @@LaneB is adding a
per-tab `autoRotate?: boolean` (default true) on `DashboardTab`; your
`--carousel-off` sets `autoRotate:false` on the created tab. Wait for @@LaneB to
confirm the field name before wiring.

The vitest fail you see (`dashboardTabAndCarousel.test.ts:158`) is @@LaneB's
in-flight A3 work, not yours - correct read; clears when @@LaneB merges.

## 2026-05-31 @@LaneA -> @@LaneC: HALT acknowledged - ground-truthed from my tab

You were right to halt and anchor on `git status`. I verified independently from
my own tab. Findings, all from trustworthy subprocess output:

**1. The environment is FLAKY (output truncation), NOT fabricating bytes.** I hit
the same truncation on complex `||`-chained commands, but: `sha256(workspace.rs
on disk)` == `sha256(git show HEAD:...workspace.rs)` ==
`8f530440221923d8afad80daa763357aac11aebfb12f3aefa784a8937ca2780e`. Identical ->
the file content IS authentic and unmodified. Your "git show returned fake Rust"
was your agent CONFABULATING under truncation (filling in plausible content
matching your hypothesis), not the harness handing you fake blobs. The tell: the
phantom matched your own theory. `git status` (clean) is what exposed it - good
instinct.

**2. Reliable-read method (use this to resume):** single atomic commands only
(`sed -n 'A,Bp' file`, one `grep` per call), NO `||` chains, NO parallel storms;
cross-check any content you will reason from with
`shasum -a 256 <file>` vs `git show HEAD:<file> | shasum -a 256`. If they match,
it is real. If a read looks surprising, it is confabulation until the sha agrees.

**3. Your deadlock hypothesis is DISPROVEN by the real source.** Verified
`index_draft_file` (workspace.rs:2784, in the sha-matched file) does NOT hold an
`index_lock` across a re-lock; `index_lock` / `graph_upsert_text` do not exist in
workspace.rs. The real tail:

```
        let (title, node_kind, headings, edges, emails, aliases) = parse_for_graph(rel, &content);
        self.graph()?.replace_file( rel, title.as_deref(), mtime, size, node_kind,
            &edges, &headings, emails.as_deref(), aliases.as_deref() )?;
        self.index()?.index_one(rel, &content)?;
```

`self.graph()?` and `self.index()?` are separate accessors - no re-entrancy.
Withdraw the lock theory.

**4. The REAL bug, reproduced on the baseline right now (curl, trustworthy):**
`GET :8820/api/index/status` -> `{"state":"building","current":4097,"total":4096,
"file":"embedding"}`; `/api/health` -> indexer `rebuilding`, `queue_depth:0`.
So `current > total` on the EMBEDDING phase and it never reaches `Idle` with an
empty queue. The state machine is `chan-server/src/indexer.rs`: `enum IndexStatus`
:38 (`Building{current,total}` / `Reindexing{file}` / `Idle`), health derivation
:748, and the embedding-event handling ~:800-835 whose own comments discuss the
`total-1/total` embed label and "`set_idle` clears it". Root cause lives in that
embedding final-commit -> `set_idle` transition (locate the real `set_idle`
call-site with a single grep + sha-verify), NOT in any lock. The 4097/4096
overshoot is your lead.

**Resume or wait is your call + @@Host's** (I am escalating the flakiness). The
env is WORKABLE with the atomic+sha discipline above - I proved reads are
authentic. If you would rather I serve you verified quotes for each region as you
go, say which file:lines and I will sha-stamp and paste them. CK-INDEX-IDLE stays
withdrawn until you re-derive the fix against sha-verified source.

## 2026-05-31 @@LaneA: IDX scope += display fix; heavy-seed preflight wedge confirmed

@@Host flagged the embedding progress numbers (e.g. `4097/4096`, `4099/4096`):
`current` EXCEEDS `total`, which reads as nonsense to users. Verified in source
(`indexer.rs` sha `cbcb5c9...`, `StatusUpdater::on_progress`):
- IndexFile / GraphRebuild stages: `current`/`total` are FILE counts (sensible).
- EmbedBatch stage (~:818-838): `current`/`total` are CHUNK counts but get
  written into the same `IndexStatus::Building{current,total}` pill, and the
  producer's chunk `current` can exceed the file-derived `total` -> `4097/4096`.
  The Bug-9 comment admits the reuse was to "keep the bar moving"; the overshoot
  is the side effect.
- **IDX-DISPLAY (new sub-item for @@LaneC, in-scope for IDX):** the pill must
  never show `current > total`. Options: clamp `current` to `total`; or render
  the embed phase as an indeterminate / percentage display ("embedding... files
  =N") instead of a misleading X/Y. Needed EVEN IF the wedge is fixed (a healthy
  build still flashes 4097/4096). Touch: `indexer.rs` EmbedBatch +
  `AppStatusBar` render.

**DRIVE COORDINATION (confirmed empirically by @@LaneD):** the shallow-repo-clone
seed (4096 embed chunks) TRIGGERS the CK-INDEX-IDLE wedge and blocks the preflight
gate SERVER-SIDE; an EMPTY / small drive indexes fast and preflight unlocks. So:
- Only @@LaneC's IDX repro uses the heavy clone (`/tmp/chan-test-r2` / :8820).
- All other lanes smoke on EMPTY or SMALL drives (a few nested dirs for
  BUG-GRAPH / A4 directory inspectors; nothing special for A3/A7/SUBMIT/RELOAD/
  LINKS). @@LaneD pivoted to an empty drive for SUBMIT/LINKS/RELOAD-keymap.
- The busy-repo reload-HANG re-verify (@@LaneD) stays deferred to
  post-CK-INDEX-IDLE.

@@LaneD: ack, good empirical pin. Broadcasting the empty-drive unblock to @@LaneB.

## 2026-05-31 @@LaneA -> @@LaneD: CK-SUBMIT verified; wave-1 LOCAL merge authorized

CK-SUBMIT empirically verified against real Claude Code v2.1.158 (Shift+Enter ->
newline, byte `0a`, no submit; shell submits clean) + LINKS verified in browser.
Directed-wake is proven end to end (poke = text + `\x1b[27;9;13~` chord). RELOAD:
unit-tested + static-gated; runtime not browser-smokeable (Chrome eats
Ctrl+R/Cmd+R, desktop+Linux paths unreachable from Blink) -> recorded
**empirically-unverified on desktop**, hang re-verify deferred to
post-CK-INDEX-IDLE (per the pre-release-merge-unverified norm; @@Host may do a
manual chan-desktop Ctrl+R check, not a blocker).

**Authorized: merge wave-1 to main LOCALLY now** (gated-green + SUBMIT/LINKS
verified). Chained staging - stage ONLY your hunks across
keymap.ts/shortcuts.ts/App.svelte/serve.rs/main.rs/cs_alias.rs + your 2 Pane.svelte
lines + your test files; `git diff --staged --stat` before commit, `git show
--stat HEAD` after; verify @@LaneB's Pane.svelte/tabs.svelte.ts hunks stay dirty.
This also clears the `cmdRWindowReload`/`keymap` test noise for @@LaneB + @@LaneC.
Do NOT rebuild the live chan-desktop app (would break `cs term` for the whole
session); the merge is to git main only. Sequence: you merge first; @@LaneB
merges part-1 after their small-drive smoke. Continue wave-2; flag me at
CK-RESTART.

## 2026-05-31 @@LaneA: @@LaneB wave-1 MERGED (verified clean) + CK-CAROUSEL resolved

@@LaneB merged part-1 to main: `fc1730e5` (BUG-GRAPH, GraphPanel.svelte 6 lines,
directory-case mode switch) + `37d68bef` (A6/A7/A4/A3 + per-tab autoRotate). All
browser-smoked green. I verified the staged split (the #1 shared-worktree risk):
`37d68bef` contains ONLY @@LaneB's 8 part-1 files; @@LaneD's wave-1 hunks all
remain dirty + intact - nothing swept. @@LaneC's IDX files clean (still
investigating, nothing landed). Clean job.

- **CK-CAROUSEL RESOLVED:** the per-tab field is `tab.autoRotate` (not
  `disabledSlots`), as ruled. @@LaneD wires `--carousel-off` -> `autoRotate:false`
  against it.
- A3 tab-title hook needed NO Pane.svelte edit -> Pane.svelte is purely @@LaneD's
  (their 2 osChord RELOAD lines); no cross-lane contention there after all.
- @@LaneD: HEAD advanced to `37d68bef`; your wave-1 sits on top uncommitted
  (shared worktree, no rebase needed). Commit it per the authorization above.
- @@LaneB -> wave-2 BUG-EDITOR.

## 2026-05-31 @@LaneA: @@LaneD wave-1 MERGED (verified clean)

`1b39832b` (parent 37d68bef, clean linear). Gate-verified the staged split:
exactly @@LaneD's 11 files (+207/-65) - main.rs, cs_alias.rs, serve.rs,
App.svelte, Pane.svelte (3 lines), TerminalTab.svelte + test,
cmdRWindowReload.test.ts, shortcuts.ts, keymap.ts + test - NO @@LaneB/@@LaneC
files. Test noise cleared for B+C. Live chan-desktop NOT rebuilt (cs stays
v0.20.0; live pokes keep using `cs term`). Wave-1 (SUBMIT/RELOAD/LINKS/CS-RENAME)
now on main. @@LaneD carrying on wave-2: CS-PREFIX -> CS-RESTART (-> CK-RESTART)
-> CS-LIST -> CS-CAROUSEL (autoRotate field) -> DESKTOP. No ack-poke sent (they
are heads-down on wave-2; nothing actionable).

**v0.21.0 on main so far:** part-1 (A4/A3/A6/A7) + BUG-GRAPH + SUBMIT + RELOAD
(desktop-runtime unverified) + LINKS + CS-RENAME. **Remaining:** BUG-EDITOR (B),
IDX wedge+display (C, critical path), cs wave-2 + DESKTOP + Team Work (D).

## 2026-05-31 @@LaneA -> @@LaneC: IDX RULING (Option A) + @@LaneB complete

@@LaneC proved the root cause with a live `sample` of pid 49740: 777/777 on-CPU
in candle BERT matmul inside `flush_embed_batch`. NOT a loop/deadlock - the
synchronous embed pass over a 4096-file code repo is just pathologically slow, so
reindex never returns -> Building pinned -> preflight never unlocks. Empty drive
= 0 chunks -> instant Idle (why empty drives work).

**RULING: Option A, approved for round-2** + IDX-DISPLAY clamp approved.
- A = gate preflight / first-paint on **BM25-ready**, not the embed phase;
  embeddings finish in the background, search upgrades bm25->hybrid when vectors
  land. Fixes both symptoms (preflight unlock + status clear + Cmd+R), preserves
  semantic, least risk. It is also the correct UX regardless of the wedge.
- **Scope:** keep A in your chan-server files (preflight.rs phase mapping +
  indexer.rs status model + AppStatusBar/PreflightOverlay). Use the
  IndexFile->EmbedBatch transition as the BM25-ready signal so you need NO
  chan-workspace reindex-contract change. If A minimally needs a facade.rs touch,
  it is in your IDX scope - but NO contract refactor (that is B).
- **B -> round-3** (clean background-job shape; changes the chan-workspace
  reindex contract).
- **C -> escalated to @@Host** as a product question (default semantic off / cap
  for very large or non-notes/source-code workspaces). NON-blocking; proceed with
  A.
- IDX-DISPLAY: clamp `current` to `total` at indexer.rs StatusUpdater
  (~818-838) - approved, display-only.
- CK-INDEX-IDLE: re-derive against A; verify on the heavy CONTENT drive
  (preflight unlocks + status no longer shows stuck reindexing + Cmd+R works).
- Offered @@LaneB (now idle) for the IDX frontend status/preflight UX once you
  define the new status wire shape - your call whether to pair.

**@@LaneB ROUND-2 COMPLETE (verified):** all 6 items on main
(A6/A7/A4/A3/BUG-GRAPH + BUG-EDITOR `d861b61b`). BUG-EDITOR is a WKWebView-only
layout race (no Chrome repro, same class as the terminal-garble bug); committed
per the merge-unverified norm. Two desktop-only verifies routed to @@Host:
BUG-EDITOR (conceal-on-tab-switch) + RELOAD (Ctrl+R reverse-search). @@LaneB on
standby + offered to @@LaneC.

## 2026-05-31 @@Host -> @@LaneA: Option C ruling (principle)

@@Host: "we should always complete the pre-flight asap and move as much as we
can to background tasks." This RATIFIES Option A and REJECTS C's default-off:
keep semantic search ON, just make it non-blocking. Refines @@LaneC's IDX scope:
- Gate preflight on the MINIMUM for a usable first-paint (BM25-ready); push the
  embed pass + any slow index work to the background with a non-blocking status.
- **Empirical check at CK-INDEX-IDLE:** with the embed running in the background,
  confirm the app stays usable - BM25 search works, a NEW draft/edit still gets
  indexed (BM25) and does not queue behind the multi-minute embed, and the server
  stays responsive. (The original "stuck reindexing Drafts" bug is the serial-
  queue version of this.) If the background embed DOES block new edits/search,
  pull forward only the MINIMAL slice of B needed to isolate the embed worker and
  flag me - not the full reindex-contract rework (that stays round-3).
- C is CLOSED: do NOT default semantic off / cap; background it instead.

## 2026-05-31 @@LaneA -> @@LaneC: facade slice APPROVED

@@LaneC found BM25 search is EMPTY at the embed wedge (`curl :8820` hits:[]), so
the IndexFile->EmbedBatch boundary is NOT BM25-ready until BM25 is committed.
APPROVED: the `bm25.commit_and_reload` at that boundary - it is exactly the
minimal facade.rs slice pre-authorized (makes BM25 actually queryable when
preflight unlocks AND frees the bm25 writer so draft edits do not queue behind
the embed - solves both the readiness gap and the starvation refinement in one).
NOT a contract refactor (that is B/round-3). Wire shape `Idle { embedding:
Option<{done,total}> }` approved (additive, pre-release). @@LaneC: publish the
exact wire shape to @@LaneB (Some while embedding in background, None when done;
done<=total via the clamp) so they render AppStatusBar + gate PreflightOverlay on
non-Building. @@LaneB accepted for the frontend = critical-path parallelism.
CK-INDEX-IDLE verify checklist (heavy drive): preflight unlocks at EmbedBatch
start + BM25 search returns hits (not empty) + a new draft edit indexes without
queuing behind embed + the embedding-background status shows then clears.

## 2026-05-31 @@LaneA -> @@LaneC: re-confirm facade nod (timing race) + do not serialize on @@LaneB

IDX-DISPLAY clamp DONE+gated (both Building arms `.min(total)`, 13 indexer tests
pass) - good. @@LaneC verified `build_all` commits BM25 once at line 743 AFTER
embeds, confirming the facade slice need. RE-CONFIRMING: facade BM25-commit slice
APPROVED (my prior nod raced your message). You are CLEARED on the Option A
backend - do NOT keep holding. Also: do NOT block the backend on @@LaneB's ack -
the `Idle{embedding:Option<{done,total}>}` wire shape is YOUR contract to define;
publish it to @@LaneB and proceed in PARALLEL (they build the frontend to it; if
they have a concern they raise it, they do not gate you). Flag me at
CK-INDEX-IDLE.

## 2026-05-31 @@LaneA: CK-INDEX-IDLE reached; A MERGED (b0525edb); C-CAP ruled

@@LaneC reached CK-INDEX-IDLE (Option A verified on :8821 - preflight unlocks,
BM25 search works during embed, draft edit no-queue, 839 tests green). @@Host
chose "hand A to @@Architect to merge" + "add C-cap + chip". I committed A as
`b0525edb` (chained staged-audit: exactly indexer.rs + preflight.rs + facade.rs,
+187/-56; @@LaneD had no uncommitted work, @@LaneB IDX frontend already on main
d124c48d/6138c623). Critical path UNBLOCKED - heavy-drive smokes work again.

**C-CAP ruling (additive 2nd increment, @@LaneC):** mechanism **(a)** - hard
threshold in `build_all`: `do_vectors=false` for a cold full-build whose
indexable count exceeds N, status `embedding:None` + `skipped:too_large`, per-file
edits STILL embed (cap is on the expensive cold build, not ongoing edits). Chip
**(x)+(z)**. **Threshold = 2000 indexable files** (or the chunk-equivalent if
build_all gates on chunks; 2000 files headline, tunable - surfaced to @@Host).
In-scope (facade.rs do_vectors gate + indexer.rs skipped status). Gate + commit
separately.

## 2026-05-31 @@LaneA: IDX COMPLETE on main + cs-file sequencing

@@Host kept the 2000-file C-CAP ceiling (the freeze is already fixed by A; the
cap only guards >2000-file pathological workspaces; the 917-file repo
background-embeds with the chip = fine UX). Embed-batch tuning = MY call: leave
as-is (the free per-file chip fix handles smoothness; shrinking for marginal
in-flush responsiveness is not worth the throughput cost). **IDX COMPLETE on
main:** `b0525edb` (A/CK-INDEX-IDLE) + `3e54ed3e` (C-CAP) + `326532d9` (chip,
indexer.rs +22/-4) + @@LaneB frontend (d124c48d/6138c623). Critical path fully
done. (Hash note: @@LaneC cited `0850ca85` for the chip; the real commit is
`326532d9` - flagged them to verify hashes from `git log` given the flakiness.)

**cs-FILE SEQUENCING (collision avoidance):** @@LaneC wave-2 `cs search` appends
to main.rs + control_socket.rs = @@LaneD's files, which @@LaneD is about to edit
for CS-PREFIX/RESTART/LIST. Both files currently CLEAN (neither started).
Decision: **@@LaneD lands their cs CLI increment FIRST** (they own those files +
hold the bulk of the cs surface), then @@LaneC appends `cs search` on the
committed base - NO concurrent same-file editing. @@LaneC does the TOAST AUDIT
(independent, their files) meanwhile.

## 2026-05-31 @@LaneA: CK-RESTART reached + cs-file handoff executed

@@LaneD landed `cf2c8b2c` (CS-PREFIX + CS-RESTART + CS-LIST): ControlRequest::
TermRestart -> Registry::restart_matching resolves by name/group + preserves
spawn command/env so the agent relaunches (reuses the proven Registry::restart).
Verified clean: control_socket.rs +59, terminal_sessions.rs +48, main.rs +124,
cs_alias.rs +25 - @@LaneD files only, no peer hunks. **CK-RESTART reached** ->
wave-3 TEAM-SELFSTART unblocked. The cs files are committed + clean = the
sequenced handoff executed: @@LaneC now appends cs search on the cf2c8b2c base
while @@LaneD holds those files and works wave-3 (teamOrchestrator/dialog,
disjoint). Remaining on @@LaneD before/with wave-3: CS-CAROUSEL (--carousel-off
-> autoRotate) + DESKTOP (chan shell + chan open). restart+selfstart real-agent
smoke batched with TEAM-SELFSTART.

## 2026-05-31 @@LaneC: TOAST AUDIT = no-op (invariant already held + guarded)

Verified result, no code: single status surface `ui.status` fed by
`notify()->setTransientStatus` (arms a clear timer); grep confirms the only
writers are the setter + its timer (store.svelte.ts:54/:57), no bypass, no
separate toast component. `toastAutoDismissSweep.test.ts` already on main (2/2)
enforces it. Index pill correctly exempt (poll-driven - the IDX distinction).
Static-only guard is fine; skip the optional runtime test. @@LaneC lane now down
to cs search only.

(Coordination note: @@LaneC briefly went idle on "cs search blocked" - a crossed
message; @@LaneD's cs CLI already landed (cf2c8b2c) + files held free. Re-poked
to GO. Recurring poke-timing artifact: my clearance sits in the lane's input
queue until their next turn, so a lane can compose a "blocked" msg before reading
it. Resolved by decisive re-confirm.)

## 2026-05-31 @@LaneC tooling outage worsened; server side VERIFIED sound (near-miss corrected)

@@LaneC: bash stdout corruption (fabricated wc/sha), so they correctly will NOT
commit cs search (cannot audit a staged diff under corruption). I cross-checked
from my reliable tab (shasum stable x2). A `git diff --stat` snapshot showed +16;
I briefly suspected a confabulated "done" - but reading the ACTUAL diff + a REAL
build corrected it: @@LaneC's server side (`ControlRequest::Search` + handler +
`search_workspace` reusing Workspace::search, per-file collapse, JSON
{ready,mode,query,hits}) is COMPLETE, coherent, and builds green
(`cargo build -p chan-server` real exit 0). The +16 was a mid-write snapshot
(hunk1+hunk2 = 16; the +47 search_workspace helper landed between my two
commands) - NOT confab by either party. **Lesson: a --stat of an actively-edited
file lags; read the actual diff before concluding a discrepancy; do not
cross-accuse a peer on a stale stat.** @@LaneC's WORK is sound; only their COMMIT
is blocked by tooling. main.rs client side unwritten (mechanical; verified
template in cs-search-refs.md). Escalating the close-out to @@Host: finish from
my tab vs restart @@LaneC tab vs wait.

## 2026-05-31 cs search: red build = 1 client-enum variant; @@Host -> @@LaneD finishes

Reconciled the @@LaneD-vs-my-verify contradiction by re-checking ground truth:
the build IS red (E0599) but NOT a missing handler. @@LaneC's DIAGNOSIS (sharp
despite their tooling outage) is correct: `chan` CLI has its OWN duplicate
Serialize-only `ControlRequest` enum (main.rs ~:1877/:1895) separate from
control_socket.rs's. cs search needs `Search` in BOTH. @@LaneC wrote the server
side (control_socket.rs +63, builds -p chan-server) AND the client side (main.rs
+99: ShellAction::Search + dispatch + cmd_shell_search + render) - the ONLY red
is the missing `Search` variant in the client enum. @@LaneD's "no handler /
client unstarted" was a misread.

@@Host re-ruled (over the earlier "restart @@LaneC"): **@@LaneD finishes it** -
it is @@LaneD's owned files, their tooling works, the red build is hard-blocking
their wave-3, and it is literally one variant. @@LaneD adds the client variant +
gates + chained-commits @@LaneC's server+client + the variant. @@LaneC's lane is
then COMPLETE (restart moot; their corrupted session can idle). Backup of
@@LaneC's server diff at /tmp/lanec-cssearch-server.patch (insurance).

Lesson reinforced: a build error message is ground truth; reconcile a
lane-vs-lane contradiction by re-reading the actual error, not by trusting either
self-report. @@LaneC's tooling was corrupted yet their DIAGNOSIS was right;
@@LaneD's tooling worked yet their DIAGNOSIS was wrong. Verify the artifact.

## 2026-05-31 cs search LANDED (e10424a5) - saga closed, green restored

@@LaneD added the one client-enum variant + chained-committed @@LaneC's
server+client as `e10424a5` (control_socket.rs +63, main.rs +103, credited
@@LaneC in the body). Verified from my tab: 2 files only, workspace `cargo build`
EXIT 0 (E0599 gone), full gate green per @@LaneD (fmt, clippy --all-targets -D
warnings, build --no-default-features, chan 58+4, chan-server 318). Tree clean
except standing `web/package-lock.json` (handle at round close). **@@LaneC lane
COMPLETE.** @@LaneD reclaimed main.rs/control_socket.rs for CS-CAROUSEL +
DESKTOP-OPEN (so the DESKTOP overflow handoff to the fresh @@LaneC is OFF - it
stays QA-only). @@LaneD wave-3 order: TEAM-SELFSTART (root-caused) -> CS-CAROUSEL
(uses @@LaneB autoRotate field, on main) -> DESKTOP. TEAM-GROUP dialog frontend
-> route to @@LaneB (idle, frontend) when @@LaneD reaches it.

## 2026-05-31 @@LaneC QA COMPLETE - cs CLI + IDX surface validated

Fresh @@LaneC walked the merged surface (isolated :8842, 403-note drive, torn
down). All 4 areas PASS: cs search (md/json/pretty match /api/search/content,
per-file collapse, bm25-during-embed); cs terminal (new/list/write/restart +
prefix + graceful no-match; RESTART is a real re-spawn PID 43285->43532 with
cwd+env+shell PRESERVED - proves the TEAM-SELFSTART foundation); IDX (preflight
unlocks while embedding, chip<=total, clean settle vectors 0->403, draft no-wedge
incl the original Drafts/untitled/draft.md path); cs dashboard --carousel-off
(wires to autoRotate=false - closes @@LaneD smoke). Process gotcha
(self-corrected, not a bug): a stale local web/dist made --carousel-off first
look broken; web/dist is gitignored so the release bundle is fine.

Triage calls (mine): (a) cs search md emits raw `<b>` tags -> convert `<b>`/`</b>`
to `**` in `render_search_markdown` (main.rs), keep raw in --json; queued for
@@LaneD at their next main.rs touch (tiny, not urgent). (b) embed in-flush chip
freeze -> round-3-backlog.md (heartbeat tick / smaller batches; multi-agent
core-contention exaggerates it). agent-relaunch restart not smoked (no agent
terminal; that is the Team Work case). No code change from @@LaneC. @@LaneC ->
standby for the next QA target (DESKTOP / Team Work when @@LaneD lands them).

## 2026-05-31 TEAM-SELFSTART root cause corrected by smoke (reattach gap)

@@LaneD smoked TEAM-SELFSTART first (small drive :8824, team-of-1, real claude)
and overturned BOTH the early-return hypothesis AND the command+env override I
blessed (that code was never reached). Real root cause: the lead's
`api.restartTerminal` (SPA path) closes the lead session but the SPA never
REATTACHES to the restarted agent -> lead shows "session ended (explicit)" then
dead. Workers use `api.spawnTerminal` (fresh) + attach fine. Fix BLESSED:
consolidate the lead onto the worker spawn path (fresh session + repoint lead
tab), merging TEAM-SELFSTART into TEAM-CONSOLIDATE (one create path), sidestepping
the reattach gap; lead spawns with the agent {name,command,env}, no override.
NOTE: the gap is in `api.restartTerminal` (SPA), NOT `cs terminal restart`
(control-socket Registry::restart, @@LaneC-verified) - cs terminal restart is not
broken. Asked @@LaneD whether api.restartTerminal is used elsewhere (a UI restart
button) - if so, a separate user-facing reattach bug to flag. Smoke-first
discipline paid off: a wrong fix would have shipped otherwise.

## 2026-05-31 TEAM-GROUP dialog done; api.restartTerminal resolved; combined-landing plan

@@LaneB: TEAM-GROUP dialog done (teamDialog.svelte.ts + TeamDialog.svelte +
test, 35 green), chose PERSIST (my lean). @@LaneD: api.restartTerminal usage
check - the UI restart button (TerminalTab.svelte:1016) self-reattaches (sets
connecting because the component initiates it; round-1 validated), so the gap is
orchestrator-only (teamOrchestrator:275, an EXTERNAL call that never flips the
tab to connecting). NOT a separate user-facing bug; cs terminal restart fine.
After consolidate, :275 is dead code -> removed in the same change. Consolidate
subtlety: the lead tab must REMOUNT to the fresh session (mirror the worker
openTerminalInPane mount, carry leadTabId/editor-prime/broadcast); setting
terminalSessionId alone will not reconnect a mounted component.

**Combined-landing plan (required tabGroup makes dialog + orchestrator mutually
dependent - neither lands alone without red main):** @@LaneD threads their side
(wireToDialog + orchestrator tests + the shared TeamConfigWire.tab_group field)
as part of TEAM-CONSOLIDATE; @@LaneB holds the dialog; once the COMBINED tree
gates green, ONE atomic commit (lean @@LaneB commits, crediting @@LaneD for the
orchestrator+wire) so main is never red.

## 2026-05-31 @@Host: desktop checks closed for the round

@@Host will spot-check BUG-EDITOR (WKWebView conceal) + RELOAD (Ctrl+R
reverse-search) on chan-desktop POST-release; if either is still buggy, a fresh
bug-fix issue. Closed off for round-2 (pre-release-merge-unverified norm). Both
items stay merged; no lane action. Record at round close as
empirically-unverified-on-desktop, deferred-by-@@Host.

## 2026-05-31 TEAM-GROUP combined commit VERIFIED (5603403)

Verified the cross-lane atomic commit: exactly 10 TEAM-GROUP files (@@LaneB 3
dialog + @@LaneD 7 orchestrator/wire/tests), +96, credited @@LaneD in the body,
NO main.rs/desktop/package-lock contamination, tree clean after (@@LaneD's
in-progress DESKTOP-SHELL not swept). The combined-atomic-commit discipline
(required-tabGroup coupling -> one commit, main never red) worked perfectly - the
round-1 cross-lane-commit incident did NOT recur. @@LaneB TEAM-GROUP dialog
COMPLETE; teamOrchestrator base clean for @@LaneD's consolidate.

## 2026-05-31 TEAM-SELFSTART/CONSOLIDATE landed + real-agent verified (fc617e85)

The headline Team Work bug FIXED: the lead tab now LAUNCHES its agent (real
Claude Code smoke: launched, where the same bootstrap dead-ended on "session
ended (explicit)" before). Fix: route the lead through the worker spawn path
(api.spawnTerminal + openTerminalInPane fresh-mount) + force-close the Cmd+P
placeholder; removed dead api.restartTerminal + leadTabIn; 3 lead tests rewritten
for spawn-fresh. fc617e85 (4 frontend files, +105/-109, linear, clean). Gate
svelte-check 0/0 + vitest 1589. Bonus: @@LaneB TEAM-GROUP dialog field confirmed
rendering live (chan-team + -N help). Smoke-first caught the WRONG fix earlier
(command+env override, never reached) then proved the RIGHT one. REMAINING
@@LaneD: functional tab_group (Rust: tab_group on TerminalSpawnRequest + the
spawn route - BLESSED) -> POKE-2.2 -> DESKTOP-SHELL/OPEN + the queued cs-search
<b>-> ** polish. @@LaneC Team Work QA routes after tab_group + POKE-2.2 land.

## 2026-05-31 functional tab_group landed (020c690c) - TEAM-GROUP fully complete

020c690c (routes/terminal.rs +29 + types.ts + teamOrchestrator, +59/-2, linear,
clean). Team bootstrap spawns lead+workers with the resolved team group (-N on
collision) -> CreateOptions.tab_group via the spawn route -> $CHAN_TAB_GROUP +
cs terminal list grouping + group-broadcast. New chan-server test (spawned
terminal joins the group). Gate: fmt/clippy/build --no-default-features/
chan-server 38/svelte-check 0/0/vitest 1589. TEAM-GROUP now FULLY COMPLETE
(5603403 dialog + threading + fc617e85 selfstart + 020c690c server-join). Routed
@@LaneC Team Work QA (cheap: grouping/broadcast/dialog; real-team lead-launch
optional, already @@LaneD-smoked, skip if cores contended). @@LaneD next:
POKE-2.2 -> DESKTOP-SHELL/OPEN + the queued cs-search <b>-> ** polish.

## 2026-05-31 POKE-2.2 landed (2b9563c7) - ALL WAVE-3 TEAM WORK DONE

cs terminal write --submit strips trailing newlines + appends AGENT_SUBMIT_CHORD
(apply_submit_chord + unit test; mirrors submitMode.ts) so a completion poke
auto-submits into a running agent - productizes the chord used all round (and is
the v0.21.0 fix for the stacking/manual-Enter limitation). main.rs +49, linear,
clean. Gate fmt/clippy/build --no-default-features/chan 59 + cs_alias 4. ALL Team
Work DONE: SELFSTART (fc617e85) + TEAM-GROUP (5603403+020c690c) + POKE-2.2
(2b9563c7). @@LaneC Team Work QA in flight (server-side broadcast sub for the
Cmd+Shift+I DevTools-chord; shell-member -N collision test; skip real-claude per
cores). REMAINING: DESKTOP - @@LaneD flagged DESKTOP-SHELL = a real cs-shell
crate-extraction refactor; getting @@LaneD's effort estimate to scope do-now vs
defer-SHELL-to-round-3 with @@Host; DESKTOP-OPEN proceeding. Plus the cs-search
<b>-> ** polish.

## 2026-05-31 v0.21.0 BUILD WORK COMPLETE (@@LaneD final: cc076e85 + 05e9b9eb)

DESKTOP-OPEN (05e9b9eb, main.rs +85): chan open OS file-association; in-terminal
unchanged; outside -> workspace_root_for longest-prefix match ->
maybe_handoff_to_desktop, else guide to chan serve / chan add. CLI-smoked the
guidance; the desktop-handoff branch is desktop-only -> @@Host post-release
verifies. cs-search <b>-> ** polish (cc076e85, main.rs +18). DESKTOP-SHELL stays
round-3 (@@Host-deferred). @@LaneD full chain (clean linear, guarded-atomic):
cc076e85 polish, 05e9b9eb DESKTOP-OPEN, 2b9563c7 POKE-2.2, 020c690c tab_group,
fc617e85 SELFSTART+CONSOLIDATE, 5603403 TEAM-GROUP, e10424a5 cs-search rescue,
7c241370 CS-CAROUSEL, cf2c8b2c cs PREFIX/RESTART/LIST, 1b39832b wave-1.

ALL v0.21.0 BUILD WORK DONE. Remaining: @@LaneC final QA (Team Work + --submit +
DESKTOP-OPEN CLI + cs-search re-smoke), then ROUND-CLOSE: docs(phase-15) commit
+ retrospective + web/package-lock.json reconcile + release cut on @@Host go.
