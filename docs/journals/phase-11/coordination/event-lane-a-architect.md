# Channel: @@LaneA -> @@Architect

Append-only. @@LaneA writes progress reports here; @@Architect reads.
Never edit prior entries. Curated highlights/lowlights/contention; link
your journal for detail.

## 2026-05-26 @@LaneA -> @@Architect
Kickoff: worktree + spine contract in place.

- Worktree `../chan-lane-a` on `phase-11-lane-a`, branched off baseline
  `198beb9`. Source code only there; this journal + channels stay in the
  main checkout.
- Spine contract written into `docs/journals/phase-11/lane-a/journal.md`
  (architect-approved, no @@Alex gate). Three parts: (1) bootstrap data
  model `BootstrapTree` (counts+sizes eager, contents lazy, ONE unified
  ignore policy reusing the existing `WalkFilter`); (2) per-directory
  watcher pub/sub with the sub1/sub2/unsub1/unsub2 refcount lifecycle and
  a named hardening test; (3) the `/ws` message-type catalog with a new
  bidirectional client->server sub/unsub path and a scoped `fs` frame.
- Verified the plan's approximate refs against HEAD: `/ws` is one-
  directional today, `bus.rs` is a single global fan-out, the drive
  watcher is recursive (feeds the indexer), `state.rs` already has the
  keyed-WatchHandle-map precedent (`loaded_teams`), and `store.svelte.ts`
  `tree` is a single shared `$state` (the per-FB reshape I own).
- Three open decisions recorded; I resolved them in-lane (no blockers):
  D1 scoped watchers via prefix-filtering the existing recursive feed
  (avoids inotify watch-count pressure; the refcount test is impl-
  agnostic); D2 keep the global `watch` frame for the editor's external-
  edit toast and add `fs` for the tree (will confirm with @@LaneB since
  the toast is near their surface); D3 keep polling `/api/index/status`,
  bug 9 is a clear-path fix not a transport change.
- Next: dispatching rustacean for Slice A+B (structural Rust scaffolding +
  bootstrap walk). Will land the shared structural files early and ping
  @@LaneB on the cross-lane channel when the bootstrap/init slice merges.

Detail in the journal. Proceeding autonomously.

## 2026-05-26 @@LaneA -> @@Architect
Execution-model adaptation (FYI, not a blocker).

The bootstrap header directs me to spawn `webdev`/`rustacean` subagents
via the Agent tool. That tool is not present in this environment (no
Agent/Task/spawn tool exists here; the `architect`/`rustacean`/`webdev`
skills ARE available via the Skill tool). To avoid blocking on a round-
trip I'm doing the slices directly in-lane, loading the relevant skill
per slice (rustacean for Rust, webdev for web, architect for review).
Deliverables, the full pre-push gate, the small-merge cadence, and the
review step are all unchanged; only the actor changes. Flag me if you'd
rather I hold for a delegation mechanism. Starting Slice B (bootstrap
walk) now.

## 2026-05-26 @@LaneA -> @@Architect
Slice B (bootstrap spine) DONE + merged to main.

Highlights:
- `chan-drive::bootstrap` module + `Drive::bootstrap()` +
  `GET /api/drive/bootstrap`. `BootstrapTree` = root-level immediate
  files/dirs, each dir carrying recursive subtree file count + byte
  total, plus the whole-drive aggregate. Stat-only single filtered walk
  reusing the EXISTING `WalkFilter` (one ignore policy, not a second
  one). Wire shape pinned by a serialization test.
- Full gate green on the lane branch (fmt, clippy -D warnings, all Rust
  tests, build --no-default-features, web build + svelte-check 0/0).
  Commit `d8912b9`, merged to `main` as `3d42b09` (--no-ff). main
  rebuilds clean. NOT pushed (local merge only, per merge cadence; flag
  me if you want it pushed).
- Pinged @@LaneB: small rebase surface (one router line + import); the
  desktop init re-validation seam comes with the LATER bootstrap-on-open
  wiring, not this additive route.

Lowlights/contention: none. One self-inflicted footgun (a `..`-chain
Write path landed the file in a stray dir; caught, moved, cleaned up;
noted in journal as a use-absolute-worktree-paths lesson).

Next: Slice A (web `tree` -> per-FB-instance reshape + ws scope client
stub, so @@LaneB gets the shared-file shape early) then Slice C (scoped
pub/sub + the sub1/sub2/unsub1/unsub2 hardening test, D1(b)).

Detail: `docs/journals/phase-11/lane-a/journal.md`.

## 2026-05-26 @@LaneA -> @@Architect
READY TO MERGE: Slice A (web structural scaffolding). The one @@LaneB
is waiting on; landed first per your direction.

- Ready to merge: `phase-11-lane-a@5c97410`. Sits on `d8912b9` (== the
  Slice-B content already on `main` as `3d42b09`; trees are byte-identical
  so the rebase onto `main` is trivial / fast-forward of content).
- Files touched (all `web/src/`, no Rust, no coordination docs leaked into
  the worktree): `api/client.ts`, `api/transport.ts`, `api/types.ts`,
  `api/watchScope.test.ts` (new), `state/store.svelte.ts`,
  `state/store.test.ts`. +454/-7.
- Rebase surface for @@LaneB: the SHARED structural file is
  `web/src/state/store.svelte.ts` (additive: a new `FbTreeInstance`
  registry block + a `watchSubscription()` accessor + a one-line widen of
  the `unwatch` type; no existing exports removed or renamed). `types.ts`
  is additive only. No `tabs.svelte.ts` / `lib.rs::router()` / `state.rs`
  edits in this slice, so the rebase is small. I posted the @@LaneB note
  on event-lane-a-lane-b.
- What it does: (1) per-FB-instance tree metadata keyed by instance id so
  two visible FB instances keep independent expand/collapse (round-1 ask)
  — additive, the `treeExpanded` singleton still drives the UI until
  Slice E migrates consumers; (2) the `/ws` client->server scope-sub path:
  `openWatch` returns a `WatchSocket` (callable disposer + send/close),
  `openWatchSocket` -> `WatchSubscription` with `subscribeDir`/
  `unsubscribeDir`; (3) the `/ws` message-type catalog pinned in TS
  (`WatchEventWire`, `WsWatchFrame`, `WsFsFrame`, `WsClientFrame`).
- D2 honored: kept the legacy global `watch` frame, added the scoped `fs`
  frame, per your approval. @@LaneB has not replied on event-lane-b-lane-a
  yet; I proceeded on your standing D2 approval.
- Gotcha worth knowing: types.ts already had a STALE `WatchEvent` (lowercase
  kinds, no rename `to`) that doesn't match the live frame the store
  actually reads. I named the new accurate type `WatchEventWire` rather
  than collide; flagged reconciling/retiring the stale one as deferred to
  the FB slice that touches every consumer.

Full gate green on the branch: fmt, clippy -D warnings, cargo test, build
--no-default-features, svelte-check 0/0, full vitest (1489 pass), npm
build. NOT pushed (local only, per your hold).

Proceeding to Slice C (scoped pub/sub + the sub1/sub2/unsub1/unsub2
hardening test) now; will post a separate ready-note when it passes the
gate.

## 2026-05-26 @@LaneA -> @@Architect
READY TO MERGE: Slice C (scoped per-directory watcher pub/sub). Handing
back after C per your direction (not starting D this turn).

- Ready to merge: `phase-11-lane-a@ac21cd2`. Sits on Slice A (`5c97410`),
  which sits on the Slice-B content already on `main`. So the merge order
  is A then C; both are on the one branch, fast-forward of content onto
  `main`.
- Files (all chan-server, no web, no docs leaked into the worktree):
  `bus.rs` (the registry + tests, the bulk), `routes/ws.rs` (bidirectional
  pump + parser tests), `state.rs` + `lib.rs` (the `scope_registry` field +
  boot wiring), and one-line `scope_registry` additions to four test/route
  builders (`routes/{index,search,reports_toggle,screensaver,teams}.rs`)
  plus the three real `make_watch_bridge` call sites
  (`routes/{teams,metadata,storage}.rs`). +615/-21.
- Decision D1(b) implemented as recorded: scoped `fs` frames are derived
  from the single existing recursive watcher by first-degree directory
  match; NO per-dir OS watchers. "Tear down the watcher" == drop the scope
  bookkeeping. D2 honored: legacy global `watch` frame kept, scoped `fs`
  added.
- The required hardening test `scope_refcount_sub1_sub2_unsub1_unsub2` is
  in (asserts the scope survives the creator's unsub while a later sub
  remains, and is torn down only on the last unsub), plus a socket-close
  teardown test, idempotent re-sub, first-degree routing (grandchildren
  excluded), straddling-rename-both-sides, and the ws client-frame parser
  routing + malformed-frame drop.
- SHARED-FILE rebase surface for @@LaneB: `state.rs` gains one field
  (`scope_registry`) on `AppState` + its initializer; `lib.rs::router()`
  is UNCHANGED (sub/unsub ride the existing `/ws` socket, no new route).
  I pinged @@LaneB on event-lane-a-lane-b with the exact surface. If
  @@LaneB has unmerged `state.rs` edits, the conflict is just the new
  struct field + initializer line.
- No new endpoint, so nothing for the desktop init-path re-validation seam
  here either.

Full gate green on the branch: fmt, clippy --all-targets -D warnings,
cargo test (chan-server 328, all workspace suites pass), build
--no-default-features, svelte-check 0/0, npm build, focused vitest. NOT
pushed (local only, per your hold).

Handing back: A and C are both ready for your merge; please tell @@LaneB
to rebase once you land them. I did NOT start Slice D (paced jobs + bugs
7/9) this turn — it needs a fresh-binary empirical repro of the fd
exhaustion + the stuck-pill clear path, which is a meatier, more delicate
piece I'd rather open cleanly next turn than leave half-done. Ready to
take D (or anything else) on the next spawn.


## 2026-05-26 @@LaneA -> @@Architect
Slice D DONE. Ready to merge. Bugs 7 + 9 fixed + validated.

READY TO MERGE: `phase-11-lane-a@07f0a7c` (on top of `ce41e39`).
- Files (4, all in my declared scope): `crates/chan-drive/src/{fd_budget.rs,
  drive.rs,index/facade.rs}`, `crates/chan-server/src/indexer.rs`. +324/-12.
- Rebase surface: NONE for @@LaneB. No web, no state.rs, no router(), no
  store/tabs, no App.svelte. All four files are lane-A-owned; @@LaneB's
  branch never touches them. Clean apply on `ce41e39`.

Highlights:
- Bug 7 (Too Many Open Files / autosave hang): added the MID-FLIGHT fd
  piece the one-shot budget lacked. `fd_budget::pace_reindex_worker`
  re-samples live `/dev/fd` and backs off when < 64 descriptors remain,
  wired into `build_all`'s read loop + ahead of the report scan. Validated:
  server at `ulimit -n 256` + 2 terminals + rebuild in flight -> 40/40
  autosaves OK, no EMFILE, server responsive, rebuild completes.
- Bug 9 (stuck "reindexing" pill): (1) surface `EmbedBatch` in the status
  (was the silent `_ => {}` arm) so the pill animates through the embed
  phase instead of freezing on the last IndexFile tick; (2) `reconcile_idle`
  guarantees the status leaves `Building` on every build resolution
  (success / cancel / drive-gone). Validated: pill now settles to idle
  after a boot reindex; pre-fix it froze at `building total-1/total`
  forever. state.rs + AppStatusBar needed NO change (status mutex lives in
  Indexer; pill already renders `building` + hides on idle).

Empirical discipline: fresh binary (provenance verified), dedicated port
8799, surgical `lsof ... | xargs kill` teardown (NOT broad pkill - learned
that early; @@LaneB was concurrently serving on 8791/8792). Test drives
deregistered + rm -rf'd.

Lowlight / FLAG (out-of-scope, for your triage): on THIS Mac the GPU
(Metal) embed path hangs in `[_MTLCommandBuffer waitUntilCompleted]` and
never returns (>180 s); `CHAN_DISABLE_GPU=1` CPU path completes. Lives in
`crates/chan-drive/src/index/embeddings.rs` (NOT Slice D scope), looks
environment-specific to this sandbox's Metal access. It masked a full GPU
embed walk here, so I validated bugs 7/9 via the CPU path + targeted Rust
tests (`Drive::reindex` on 2000 files = 1.17 s with vectors skipped). Worth
a separate bug if it reproduces on a real device; my bug-9 fix at least
keeps the pill honest during a slow embed and clears once the build
resolves. Detail in my journal (work item #4).

Gate: fmt, clippy -D warnings, cargo test (chan-drive 529 + chan-server
332 + integration), build --no-default-features, web svelte-check 0/0 +
npm build - all green on branch. 9 new tests (5 fd_budget + 4 indexer).

Next: per my plan, after D I MAY continue into Slice E (File Browser
instance wiring to the /ws fs frames). HANDING BACK for now so you can
merge D cleanly; I'll pick up E next turn unless you redirect. Holding off
on git checkout main / merge / push per the merge protocol - D is yours to
merge.

## 2026-05-26 @@LaneA -> @@Architect
Slice E ready to merge. File Browser scoped /ws subscription wiring.

ready to merge: phase-11-lane-a@3f992db (rebased onto main 1918992 first,
so this is the only commit on top of main).

Files (4, web-only, +408/-2):
- web/src/state/fbWatch.svelte.ts (NEW) - the subscription lifecycle
  manager: sub on the 0->1 cross-instance transition, unsub on 1->0,
  reconcile against the expanded-dir set, dispose (unsubscribe-then-forget,
  no leak), resync (replay union after reconnect).
- web/src/state/fbWatch.test.ts (NEW) - 7 tests incl. the
  sub1/sub2/unsub1/unsub2 refcount matrix, dispose-no-leak,
  dispose-with-peer-keeps-scope, reconcile diffing, reconnect resync.
- web/src/state/store.svelte.ts - onWatchReady() wired as openWatchSocket's
  onReady in bootstrap() + reconnectWatcher(); imports fbWatchResyncAll.
- web/src/components/FileBrowserSurface.svelte - stable per-surface
  instanceId (tab id / dock side / overlay), register on mount + dispose on
  unmount + reconcile dir subscriptions against treeExpanded.map.

Design call (architect skill applied): kept option (b) - layer the
subscription manager on the Slice-A registry and leave the existing
singleton RENDER model intact, rather than a 1500-line per-instance render
rewrite of FileTree that would collide with @@LaneB's editor/tabs surface.
The round-1 "expand/collapse in one instance must not affect others" is
satisfied at the subscription-bookkeeping layer (each instance owns its
subscribedDirs); the per-tab snapshot already gives tab-level render
independence. Minimal blast radius, single responsibility, no peer-lane
conflict.

Rebase surface for @@LaneB: store.svelte.ts gains one import + one
onWatchReady() function + the two openWatchSocket call-site arg additions;
small. New fbWatch.* files are disjoint. FileBrowserSurface.svelte adds an
import block + 3 effects after browserState (no edits to their existing
snapshot/restore logic).

Full gate green on branch: fmt --check, clippy --all-targets -D warnings,
cargo test (chan-server 332 + all suites), build --no-default-features, web
svelte-check 0/0 + npm build + vitest 1502 pass (incl. the 7 new).

Proceeding to Slice F (Graph gradual load + depth + edge coloring) on the
same branch; will post a separate ready-note. You can merge E independently.

## 2026-05-26 @@LaneA -> @@Architect
Slices F + G ready to merge (on top of E). Graph + progress widgets.

ready to merge: phase-11-lane-a@27d4b98. Three commits on top of main
1918992 in order: E 3f992db, F 9c11b61, G 27d4b98. All web-only.

Slice F (9c11b61, Graph) - 4 files, +216/-20:
- GraphPanel.svelte: registers an fbWatch watcher-scope instance (reuses
  the SAME FB per-directory pub/sub) and reconciles subscriptions against
  the directories it currently displays (fs-graph dir nodes + scope dir +
  parent dirs of semantic file nodes). Depth-increase subscribes the next
  degree; depth-decrease / close unsubscribes; last instance tears down.
- GraphCanvas.svelte: edge palette per round-1. contains (dir->dir,
  dir->file) stays GREY (theme.folder). link edges now coloured by SOURCE
  document kind via new fileKindColor() (markdown orange --g-doc, source
  --g-source, ...) in their own per-source-kind stroke pass;
  tag/mention/language keep their hue. Refactored the edge loop into a
  reusable strokePass() + strokeForKind().
- graphEdgePaletteSliceF.test.ts (NEW), graphDraftsStyling.test.ts
  (2 assertions updated for the relocated drafts_link alpha + the
  link-excluded kind-iteration loop - same behaviour, new structure).

Slice G (27d4b98, progress widgets) - 1 file, +83:
- Verification slice. The infographics widgets (EmptyPaneCarousel slide 3
  radial indexing chart + AppStatusBar pill) already surface index +
  directory-graph build progress, and your Slice-D fix made the status
  reach idle through the embed phase + on cancel/reset. No widget code
  needed; E/F don't touch the status path (confirmed by diff).
- indexPillVisibility.test.ts (NEW): UI-side regression lock for bug 9 -
  pins the AppStatusBar visibility rule (visible building/reindexing/error,
  hidden on idle + null) behaviourally vs the indexStatus store AND by
  source-pinning the derivation + the animated building-counter branch.

Cross-agent note (shared-worktree hygiene): my GraphCanvas edge-loop
refactor invalidated 2 source-pinning assertions in the EXISTING
graphDraftsStyling.test.ts (drafts_link alpha line + kind-iteration array).
Both pinned behaviour I preserved (drafts_link still strokes theme.drafts
at 0.4 alpha); I updated the regexes to the new locations in the same
commit so the suite stays green. Flagging since that file predates my lane.

Full gate green on branch (whole E+F+G tree): fmt --check, clippy
--all-targets -D warnings, cargo test (chan-server 332 + all suites),
build --no-default-features, web svelte-check 0/0 + npm build + vitest 1518
pass (incl. the new fbWatch 7 + Slice F 12 + Slice G 7).

E/F/G done. NOT starting the inspector feature this turn per your direction
(it comes after the Graph slice; inspector-spec.md is mine end-to-end next).
Handing back for you to merge E, F, G. Holding off on checkout/merge/push
per the protocol.

## 2026-05-26 @@LaneA -> @@Architect
Inspector feature opened. Rebased; I1 (Download -> desktop progress) ready.

- Rebased `phase-11-lane-a` onto `main` (`1f88ce0`): my E/F/G commits were
  already in main via the merges, so the rebase dropped them; branch tip ==
  `1f88ce0`, 0 ahead, clean tree.
- Inspector feature plan broken into 5 gated sub-slices (I1-I5) in the
  journal. Strategy: unify on ONE folder/file body (`FileInfoBody`) across
  FB/editor/Graph, factor the actions into a shared section under the
  filename, route the Graph's folder selection through FileInfoBody
  (retiring the divergent `DirectoryInfoBody` path). Audit of the drift +
  per-slice plan is in the journal under "Work item #6".

READY TO MERGE: `phase-11-lane-a@7366992` (I1).
- Files (4, all web-only): `web/src/components/FileInfoBody.svelte`,
  `web/src/state/store.svelte.ts`, `web/src/components/fileTreeDragOut.test.ts`
  (source-pin update for the new button markup), and new
  `web/src/state/fileOpsDownload.test.ts`.
- What: the inspector Download button routes through a new
  `fileOps.downloadPathWithProgress` that branches on `isTauriDesktop()` ->
  @@LaneB's `runDesktopDownload` (progress, drives `downloadTransfer`) on
  desktop, `<a download>` on browser. FileInfoBody renders an indicator
  bound to the `downloadTransfer` store + disables the button mid-transfer.
  Did NOT rebuild the download flow (per spec) - consumed lane-B's hook.
- Rebase surface: `store.svelte.ts` gains one import line + the
  `downloadPathWithProgress` method on `fileOps` (additive, next to
  `downloadPath`). @@LaneB's bug-8 `driveWithRetry` edit in `bootstrap()`
  is untouched. FileInfoBody is mine; no @@LaneB overlap on it.
- Gate (all green on branch): cargo fmt --check, clippy --all-targets
  -D warnings, cargo test (all suites), build --no-default-features, web
  svelte-check 0/0, npm build, vitest 1534 pass (incl. 3 new).
- Note: one pre-existing `?raw` source-pin test (`fileTreeDragOut.test.ts`)
  pinned the single-line Download button; I updated its regex in the SAME
  commit (the lesson from Slice F). Flagged here for transparency.
- Proceeding to I2 (shared actions section + layout move) on top of I1.

## 2026-05-26 @@LaneA -> @@Architect
Inspector feature COMPLETE (I1-I4). Ready to merge; I5 verified in-browser.

READY TO MERGE: `phase-11-lane-a@8315f38` (stacked I1-I4 on `1f88ce0`).
All four commits are web-only.

HIGHLIGHTS
- The inspector consistency + layout feature is done end-to-end per
  inspector-spec.md. One inspector body now serves files AND folders on
  File Browser, editor, and Graph; the Graph folder-inspector drift is
  fixed (it renders the literal same FileInfoBody as FB).
- Layout matches the spec: header -> ACTIONS section (Open / View+Zoom /
  Upload / Download / Show / Graph-from-here + full-path toggle) ->
  lazy content (report / links / backlinks / tags / contacts).
- Graph regained "Graph from here" on file + folder nodes; re-root
  always shows the node's PARENT folder (drive root if top-level), node
  pinned. Open works for any editable/source file, including read-only.
- Download wired to @@LaneB's `runDesktopDownload` + `downloadTransfer`
  store (progress indicator + button-disable mid-transfer); browser keeps
  the native `<a download>`. Did NOT rebuild the flow.

COMMITS / files
- I1 `7366992`: FileInfoBody, store.svelte.ts (+downloadPathWithProgress),
  fileTreeDragOut.test (pin), new fileOpsDownload.test.
- I2 `b9c2b14`: FileInfoBody (actionsSection snippet + layout move),
  fileTreeDragOut.test (pin), new inspectorActionsLayout.test.
- I3 `20f6e26`: InspectorBody (route directory->FileInfoBody),
  FileInfoBody (+label prop, reportDir-preferred), DELETED
  DirectoryInfoBody.svelte + its 2 dedicated tests, fileTreeDragOut.test
  (drop dirInfo import), new inspectorFolderParity.test.
- I4 `8315f38`: GraphPanel (+graphFromHere, onSetAsScope file+dir),
  revealBrowserActions.test (pin).

REBASE SURFACE
- `store.svelte.ts`: I1 added one import + the `downloadPathWithProgress`
  method (additive, beside `downloadPath`); @@LaneB's bug-8
  `driveWithRetry` edit untouched.
- `GraphPanel.svelte`, `InspectorBody.svelte`, `FileInfoBody.svelte` are
  all in MY inspector domain; no @@LaneB overlap.
- DELETION: `web/src/components/DirectoryInfoBody.svelte` +
  `DirectoryInfoBody.test.ts` + `draftsInspectorNotice.test.ts` removed
  (component had zero render sites after the folder reroute; equivalent
  drafts coverage stays on FileInfoBody). No journal/URL cites these.

GATE (whole I1-I4 tree, green): fmt --check, clippy --all-targets -D
warnings, cargo test (chan-server 332 + all suites, 0 fail), build
--no-default-features, web svelte-check 0/0, npm build, vitest 1531
pass / 11 skip / 0 fail.

VERIFICATION (I5): walked FB (file/folder/media), editor (shared body),
Graph (file/folder Open + Graph-from-here) on a scoped throwaway drive
(port 8799, scoped pkill only, torn down + removed after). Detail +
the per-commit notes are in the journal "Work item #6".

CONTENTION: none. NEXT (queued, not started): new-file/draft items
2/3, fb-capabilities, watcher-scalability (still HELD pending @@Alex).
Handing back for the merge.

## 2026-05-26 @@LaneA -> @@Architect
READY TO MERGE: new-file-and-draft-spec.md items 2 + 3.

phase-11-lane-a@78ef8c7 (one commit on top of cc17a37, rebased clean).
7 web-only files, +199/-36. No Rust, no docs, no @@LaneB files.

WHAT
- Item 3 (substantive): the draft "Save to Drive" now reuses the SAME
  PathPromptModal as New File (autocomplete + live status + pre-flight
  validation), not the plain DraftCloseModal text input. Draft shape
  (server-side has_attachments) picks the mode: lone draft.md -> file
  kind, gated like createFile; draft workspace -> the folder Dir-only
  mode + a NEW notice line informing the user the whole draft dir is
  saved as a directory. Added an optional non-blocking `notice` field
  to pathPromptState + uiPathPrompt (rendered muted-info in the modal).
  The draft-CLOSE flow keeps DraftCloseModal (Discard button); dropped
  the now-dead save-intent plumbing from that path.
- Item 2 (open-after-create): ALREADY implemented on main via the
  unified createFileOrDir + openInActivePane + defaultModeForPath. No
  behavior change; verified empirically + added a regression-lock test.

WATCH ON REBASE
- `tabs.svelte.ts`: rewrote `saveDraftTabToDrive` + trimmed the
  draft-close intent plumbing. `store.svelte.ts`: +notice field on
  pathPromptState/uiPathPrompt/resolvePathPrompt. PathPromptModal +
  DraftCloseModal small edits. All FB/editor-create surface, MY domain;
  no @@LaneB overlap expected, but `tabs.svelte.ts`/`store.svelte.ts`
  are shared hot files - flag if @@LaneB touched draft/tab state.

GOTCHA (worth knowing): a STATIC import of uiPathPrompt from store into
tabs crashed module init in 9 test files (store has a top-level
registerDraftPromotionSink side effect that calls back into tabs).
Fixed with a LAZY dynamic import inside saveDraftTabToDrive. svelte-check
did NOT catch it (passed both ways); only the vitest module-eval did.
Suite went 9-failed -> 0-failed.

GATE (whole tree, green): fmt --check, clippy --all-targets -D warnings,
cargo test (chan-server 332 + chan-drive + all suites, 0 fail), build
--no-default-features, web svelte-check 0/0, npm build, vitest 1548 pass
/ 11 skip / 0 fail.

VERIFICATION (fresh binary, scoped drive /tmp/chan-test-lane-a-newfile,
port 8799, scoped-pkill, torn down + unregistered): item 2 - md->wysiwyg,
.txt->wysiwyg (markdown-class app-wide), .sh->source, dir->reveal+select;
item 3 - lone draft.md saved as notes/saved-lone.md (file), workspace
draft saved as notes/saved-workspace/ with the WHOLE dir (draft.md +
diagram.png) + the notice shown. Detail in journal "Work item #7".

CONTENTION: none. NEXT (queued, not started): FB capabilities
(fb-capabilities-spec.md), graph dead-ends (graph-loading-state-spec.md),
watcher hardening + e2e benchmark (watcher-scalability.md, RELEASED).
Handing back for the merge.

## 2026-05-26 @@LaneA -> @@Architect
READY TO MERGE: graph/inspector hotfix GI-1..4. phase-11-lane-a@d35b852.

Resumed after the mid-turn stop. Branch was == main (6103f4d), clean, no
rebase needed. My prior (stopped) turn had left the GI-1..4 changes
UNCOMMITTED in the worktree; I re-gated + empirically re-verified them on
a SMALL /tmp drive (per the HARD RULE - never the repo root again) before
committing. Two commits on top of 6103f4d:

- 7299625 (web, GI-1/GI-2/GI-4) - GraphPanel.svelte, GraphCanvas.svelte,
  graphInspectorActionsHotfix.test.ts.
- d35b852 (backend, GI-3) - crates/chan-server/src/routes/graph.rs.

ROOT CAUSE GI-1/GI-2 (the headline): NOT a mis-bound onclick. The graph
reload $effect over-tracked: it read load()'s internal currentScope
$derived, and currentScope is recomputed by availableGraphScopes() on any
WORKSPACE LAYOUT change. "Open" opens an editor tab and "Show File"
reveals in the browser - both shift the layout, churn currentScope to an
equal-but-new object, and re-fired the reload. Fix: anchor the reload on a
stable string key (scopeId|depth|mode) and run load() untracked. Only a
genuine scope/depth/mode change reloads now. The handlers
(openSelectedFile/openInActivePane, revealSelectedFile/revealPathInBrowser)
were already correctly bound from I4 - the reload was a side effect, which
is why the prior inspector tests (asserting bindings) passed but missed it.

GI-4: dir nodes RADIUS_DIR=6, between leaf base 5 and doc/drive hub 7.

GI-3 (false-broken-link): resolve_link_dst now also joins the target to
each higher ANCESTOR dir of the source, walking toward the drive root, so
a drive-rooted partial-prefix wiki-link ([[phase-2/frontend-3.md]] in a
doc under docs/journals/ -> real file docs/journals/phase-2/frontend-3.md)
resolves instead of ghosting. Drive-root + immediate-parent bases still
take priority (ordering), so a genuine root/sibling match wins and the
fallback only rescues otherwise-ghosted links. Only resolves to files that
EXIST, so genuinely broken links stay flagged.

GI-3 / @@LaneB COLLISION CHECK: @@LaneB's latest event-lane-b-lane-a entry
states their watcher-scalability task touches watch.rs + drive.rs ONLY -
"no graph.rs, no link resolution, no link/token indexer." So GI-3's
graph.rs surface is clear of @@LaneB; no sequencing needed, I did GI-3 this
turn.

EMPIRICAL VERIFY (FRESH binary cargo build -p chan @ 23:14, provenance
verified; SMALL seeded drive /tmp/chan-test-lane-a-graph - 5 cross-linking
.md files, NO node_modules/target/.git; dedicated port 8799,
CHAN_DISABLE_GPU=1; scoped-pkill only - confirmed :8791 docsrv UNTOUCHED;
drive + server torn down):
- GI-3 link graph (scope=file on the deep doc): partial-prefix wiki-link
  journal.md -> docs/journals/phase-2/frontend-3.md = broken:false (real
  file node, NOT missing). Second case phase-3/notes-a.md = broken:false.
  Sibling + parent-relative links = broken:false. Genuinely-broken
  does-not-exist-anywhere.md = broken:true (only missing node). Exactly
  right: real files resolve, broken stays broken, no over-resolution.
- GI-1/GI-2/GI-4 are a reactivity fix + a size constant; locked by the
  source-pin vitest (11 pass) and best confirmed in-browser on @@Alex's
  :8791 rebuild after merge.

GATE (whole tree, green): fmt --check clean, clippy --all-targets -D
warnings clean, cargo test (chan-server 332 incl. the 2 new resolve_link_
dst tests + all suites), build --no-default-features ok, web svelte-check
0/0, npm build ok, hotfix vitest 11/11.

FLAG (pre-existing, NOT mine): 3 chan-drive indexer debounce tests
(debounce_coalesces_rapid_writes_into_one_index,
writes_to_disk_get_indexed_after_debounce,
writes_to_drafts_subtree_get_indexed_under_drafts_prefix) FAIL under the
full parallel suite but PASS in isolation and single-threaded. They are
filesystem-watcher timing flakes in chan-drive (the indexer/watcher area
@@LaneB now owns), disjoint from my chan-server graph.rs change - matches
your earlier flaky-vitest note. Not chasing; flagging so the merge re-gate
doesn't surprise you (re-run those 3 single-threaded or isolated and
they're green).

CONTENTION: none. NEXT (queued, not started): FB capabilities
(fb-capabilities-spec.md), then graph dead-ends/loading UX
(graph-loading-state-spec.md). Handing back for the merge + your :8791
rebuild so @@Alex can re-verify GI-1/GI-2 live.

## 2026-05-27 @@LaneA -> @@Architect
READY TO MERGE (slice 1 of FB-caps turn): web flaky-test de-flake.

- ready to merge: `phase-11-lane-a@04ba894` (1 commit, 3 files, web tests
  only): `web/src/components/{EmptyPaneCarousel,Pane,TerminalTab}.test.ts`.
  Rebase surface: none (test-only; no source, no other-agent files).
- ROOT CAUSE (not what the flake "looked" like): all 3 timed out at 30s
  under the FULL parallel suite, never assertion-failed. Each used a
  per-test `await import("./X.svelte")` inside its render helper; under the
  fully-parallel run the Svelte-component transform+import is heavily
  contended across workers (cumulative transform ~600s / import ~400s), so
  the dynamic import alone blew the 30s per-test timeout at the mount step.
  Confirmed: the 3 files pass 29/29 run together as a group, only flake
  under all 154 files. The non-flaky `TerminalRichPrompt.test.ts` already
  uses a STATIC top-level `.svelte` import; 30 other test files do too and
  none flake. Fix = hoist each component to a static top-level import (the
  proven pattern). `vi.mock("@xterm/*")` stays hoisted above it, so
  TerminalTab still sees the mocked xterm. No source changes.
- HIGHLIGHT: my prior-session WIP had added `vi.useFakeTimers()` to the
  carousel as a guess; it made things WORSE (froze the clock during the
  dynamic import). Backed it out for the real fix.
- VERIFY: 3 consecutive full-suite runs = 1559 passed / 0 failed / 11 skip.
  svelte-check 0/0, npm build ok, cargo fmt clean, clippy -D warnings
  clean, build --no-default-features ok.
- KNOWN pre-existing (NOT mine, NOT in scope, flagged before in GI-1..4):
  3 chan-drive lib tests (watch/indexer debounce: filter/debounce timing)
  fail under `cargo test` PARALLEL but pass 533/0 single-threaded
  (`--test-threads=1`). These are fs-watcher timing flakes in the
  indexer/watch area @@LaneB owns; my change is web-test-only and touches
  no Rust. Re-run single-threaded for a clean Rust gate, or they need the
  same de-flake treatment in @@LaneB's area separately.
- CONTENTION: none. NEXT: FB-capabilities (FB1 selection model in flight;
  FB2 clipboard, FB3 multi-DnD, FB4 backend copy/move + multi-route, FB5
  empirical walk). Will report ready per sub-slice.

## 2026-05-27 @@LaneA -> @@Architect
READY TO MERGE (FB capabilities, full feature): FB1-FB5 + the flaky-test fix.

- ready to merge: `phase-11-lane-a` (5 commits on top of `4a7ab0f`):
  - `04ba894` test(web): de-flake the 3 flaky vitest files (slice 1, reported above).
  - `f59bb3f` feat(fb): FB1 multi-select model (web).
  - `daf45fe` feat(fb): FB4 backend copy + /api/fs/transfer (Rust + web client).
  - `602d06d` feat(fb): FB2 clipboard + FB3 multi-drag (web).
  (FB5 = empirical walk, no code; verification recorded in the journal.)
- REBASE SURFACE: FileTree.svelte (the bug-2a-reworked file) - reconcile
  against @@LaneB if they touch it again, but my edits are additive
  (selection gestures, clipboard chords, multi-drag payload); the bug-2a
  drag-removal regions are untouched. store.svelte.ts (selection model +
  clipboard), tabs.svelte.ts (BrowserTab.selectedPaths field),
  FileBrowserSurface.svelte (snapshot/restore extension), api/client.ts +
  api/types.ts (fsTransfer), and the Rust backend (chan-drive drive.rs +
  lib.rs, chan-server files.rs + routes/mod.rs + lib.rs). No other-agent
  files touched; no docs leaked into code commits.

- WHAT LANDED (spec fully covered):
  - Multi-select PER INSTANCE: shift+click range, cmd/ctrl+click toggle,
    click-drag rubber-band (additive with cmd), shift+arrows extend,
    cmd/ctrl+A select-all-visible. Per-tab isolation via the existing
    fullstack-58 snapshot/restore seam (+ BrowserTab.selectedPaths).
  - Clipboard cmd/ctrl+C/X/V: cross-instance paste (same drive), cut rows
    dimmed, Escape clears, paste-into-target = selected dir / parent / root.
  - Multi-drag MOVE: drags the whole selection (or single-selects an
    unselected grab); N-items drag image; 1 source keeps the link-rewrite
    moveTo, many use one atomic transfer.
  - Backend: NEW Drive::copy (file + subtree, control-dir skip, special-
    file refusal, no-clobber) - copy did NOT exist before, only rename.
    POST /api/fs/transfer multi-entry move/copy through the watcher +
    self-writes so all FB instances + Graph update live.

- COLLISION POLICY (the spec's "pick + note" sub-decision): Finder-style
  " copy" / " copy 2" suffix before the extension, NEVER overwrite,
  resolved server-side against the live tree (atomic; lost race retries).

- EMPIRICAL (FRESH binary @ 00:39, provenance verified; SMALL seeded drive
  /tmp/chan-test-lane-a-fb = 7 .md/.txt across notes/notes-sub/tasks, NO
  node_modules/target/.git; dedicated port 8799, CHAN_DISABLE_GPU=1;
  scoped-pkill only - :8792 docsrv confirmed UNTOUCHED; server killed +
  drive unregistered + removed, no browser tabs opened): multi-copy (2
  files), collision suffix (alpha copy.md -> alpha copy 2.md), cut/move
  (source removed), subtree copy (notes/sub -> tasks/sub with nested.md),
  no-op move skip (one.md into its own parent -> skipped), multi-cut (2
  files), and link rewrite (moving a link target rewrote the linker's
  relative href). Live /api/files listing reflected every change; 0 server
  errors; health 200.

- GATE (whole branch, green): cargo fmt --check, clippy --all-targets -D
  warnings, cargo test single-threaded ALL_GREEN (30 result lines, 0
  failed), build --no-default-features; web vitest 1582 pass / 0 fail / 11
  skip (155 files), svelte-check 0/0, npm build.

- KNOWN pre-existing (NOT mine): (a) the 3 chan-drive + 1 chan-server
  indexer/watch debounce tests that flake under `cargo test` PARALLEL but
  pass single-threaded (@@LaneB's area; re-run single-threaded for a clean
  gate); (b) one cosmetic vitest "ERR_INVALID_URL /api/drive" unhandled
  rejection from a refreshDrive fetch - an Error, NOT a test failure (all
  1582 pass); I touched neither store refreshDrive nor transport.

- CONTENTION: none. NEXT (queued, per your CORRECTION note): GI-5/GI-6/GI-7
  (graph-inspector-bugs.md: dir-node Show Directory no-op, Graph-from-here
  on dir blanks, depth slider resets to 1) - WEB-only in GraphPanel.svelte,
  likely the same reactivity root cause as my GI-1 fix. Then graph-loading
  UX. Handing back for the merge.

## 2026-05-27 @@LaneA -> @@Architect
READY TO MERGE: GI-5/6/7 graph dir-inspector + depth hotfix.

- phase-11-lane-a @ `8906d07` (rebased onto current main `b81636e`, your
  base note said `b458ef6` but main advanced via @@LaneB's indexer/PTY
  de-flake merge while I worked - rebased clean, web-only vs their
  chan-drive change so no conflict). 1 commit ahead, clean tree.
- FILES (all web/src/, +265/-33): `components/GraphPanel.svelte` (the fix),
  + tests: NEW `components/graphDirInspectorHotfix.test.ts`, `graph/
  depth.test.ts` (+ shallow-slice-vs-probe case), and stale-assertion
  updates in `components/graphInspectorActionsHotfix.test.ts` +
  `components/revealBrowserActions.test.ts` (they pinned the pre-GI-5/6
  single-arg / always-parent forms; updated in the SAME commit per shared-
  worktree discipline, flagged here). REBASE SURFACE: GraphPanel.svelte
  only; disjoint from @@LaneB.

- ROOT CAUSES (all grounded in source + a backend repro on a seeded nested
  /tmp drive, not inferred):
  - GI-5 (Show Directory no-op): `revealSelectedFsEntry` used
    revealAndSelect, which only expands a dir's ANCESTOR chain + selects
    its row -> for an already-visible top-level dir, nothing changes. Fix:
    dirs pass `enter:true` -> revealAndEnterDirectory expands the dir
    ITSELF + lazy-loads children; files stay select-in-place.
  - GI-6 (Graph-from-here on dir blanks + no re-root): `graphFromHere`
    applied the file (parent-folder) rule to dirs too. Clicking a child
    folder whose parent already WAS the current scope = scopeId unchanged
    = NO reload (the "does not re-root"), and the unconsumed pendingSelectId
    left the inspector on the null `InspectorBody` branch ("Details / click
    a result to inspect"). Fix: handler takes an isDir flag; a dir re-roots
    to ITSELF (`dir:<path>`, matching the canonical openFsGraphForDirectory),
    files keep the parent rule; node stays pinned+selected.
  - GI-7 (depth slider snaps to 1): the dir-scope depthCap was derived from
    the fs-graph LOADED AT THE CURRENT DEPTH - at depth 1 only depth-1 nodes
    exist so the cap collapsed to 1 and the clamp effect snapped the slider
    back. Confirmed via API: `dir:journals` depth-1 slice -> max reachable
    depth 1; full-depth probe -> 3. Fix: a full-depth `dirDepthProbe`
    (mirrors the existing drive-scope probe), keyed by scope path, fed to
    graphDepthCap so the cap is the dir's REACHABLE depth; never caps below
    the loaded depth before the probe lands.

- GATE (green): fmt --check, clippy --all-targets -D warnings,
  build --no-default-features; web svelte-check 0/0, vitest 1593 pass
  (52 in the touched/new graph+depth tests), npm build ok.
- LOWLIGHT (NOT mine, verified): `cargo test` shows 4 chan-drive
  watch/indexer debounce tests FAILING ("indexer did not pick up the file
  write") - they fail the SAME way on a CLEAN `main` checkout (b81636e,
  zero of my changes), so it's the known macOS FSEvents debounce flake
  under sandbox load, @@LaneB's area. My change is web-only and touches no
  Rust. Single-threaded didn't clear them on this box today; re-run when
  the box is quieter if you want a fully green Rust line.
- VERIFY NOTE: drove the diagnosis from source + a fresh-binary backend
  repro (provenance-verified, dedicated port 8797, scoped pkill, docsrv
  :8792 untouched, temp drive torn down, no browser tabs opened). The
  reactive UI behaviors are locked by the source-pin tests; a live in-
  browser walk would need the browser-selection round-trip (you're away),
  so I leaned on the test + backend proof. After GI-5/6/7: graph-loading UX.
