# Chan Pre-Release Phase 2 Journal

Owner: @@Architect. Host: Alex.

Source request: [[phase-2/request.md]].
Carry-forward context: [[phase-1/summary.md]].

## Plan summary

Phase 2 sharpens product behavior in three surfaces that landed in
phase 1 but still surface defects against a live drive: the Graph
overlay, the WYSIWYG editor list rendering and menu wording, and the
Search overlay (results model, overlay layout, code report linkage).
Phase 2 also elevates `language:` from a search-only concept into a
first-class graph dimension, mirroring how phase 1 elevated it for
search.

Every change ships with appropriate Rust + web tests; user-visible
behavior gets a webtest smoke pass before commit; filesystem,
process, and watcher invariants get a syseng hardening pass.

## Carry-forward from phase 1

These rules came out of [[phase-1/summary.md]] and
are enforced for every task in phase 2:

* Freeze cross-boundary wire shapes in the owning task file before
  any downstream task consumes them. Document the shape with a
  sample payload.
* Distinguish three smoke gates: "feature implemented", "HTTP
  reachable", "browser behavior observed". A green CDP smoke is the
  only thing that closes a frontend task.
* Keep `journal.md` reconciled with the latest state. Stale
  "remaining" sections are not allowed once a later run completes
  the work.
* Keep adjacent repo changes (chan-core, chan-tunnel-*) visible in
  the journal before commits land. Sibling-repo work coordinates
  through journal log entries.
* Architect reviews specialist work before it is treated as done;
  Rustacean reviews any Rust API or dependency change; Syseng
  reviews anything touching the filesystem, watcher, processes, or
  failure handling.

## Work items (with owners and status)

Status codes: TODO / IN_PROGRESS / REVIEW / DONE / BLOCKED.

### Graph

* [REVIEW] G1a: ghost / `missing: true` on `/api/graph` for indexed
  files that no longer exist on disk. Owner: @@Backend
  ([[phase-2/backend-3.md]],
  [[phase-2/rustacean-2.md]]).
* [REVIEW] G2: folder / multi-file scope force-layout fix so
  documents separate instead of stacking. Owner: @@Frontend
  ([[phase-2/frontend-6.md]]).
* [REVIEW] G3: depth slider clamps to a scope-aware maximum
  (file=1, group=N, folder=deepest subtree depth, drive=cap).
  Owner: @@Frontend ([[phase-2/frontend-9.md]]).
  Cap derivation in `web/src/graph/depth.ts` with Vitest coverage;
  drive scope uses a one-shot fs-graph probe.
* [REVIEW] G4: live watcher consumption in the open graph overlay so
  new files appear and deleted files render as ghosts without a
  reload. Owner: @@Frontend
  ([[phase-2/frontend-7.md]]).

### Editor

* [REVIEW] E1: enumerated / nested list indentation gets vertical
  visual guidance during edits via depth-classed lines and per-level
  guides. Owner: @@Frontend
  ([[phase-2/frontend-5.md]]).
* [REVIEW] E2a: editor file menu (FileEditorTab kebab) wording
  pinned to "Graph this" / "Show File". Owner: @@Frontend
  ([[phase-2/frontend-2.md]]).
* [TODO] E2b: extend the rename across the bottom accessory pills
  and the empty-pane navigation labels so "Files" -> "Show File"
  and "Graph" -> "Graph this" everywhere they appear in the editor
  context. Owner: @@Frontend
  ([[phase-2/frontend-9.md]]).

  Note (2026-05-16 @@Architect): we will probably hold this rename
  unless Alex confirms. The kebab menu inside an open file
  (FileEditorTab, already renamed by frontend-2) reads as
  "Show File" / "Graph this" because the file is the antecedent.
  The accessory pills and empty-pane buttons act on the *overlays*
  with no file in hand, so "Files" / "Graph" remain accurate. Hold
  for direction.

### Search

* [REVIEW] S1: only index `#tags` from markdown files; chan-drive
  gate locked with regression tests. Owner: @@Backend
  ([[phase-2/backend-1.md]]).
* [REVIEW] S2: Search Status code report exposes a "Graph this"
  action that opens the whole-drive semantic graph. Owner:
  @@Frontend ([[phase-2/frontend-4.md]]).
* [REVIEW] S3a: collapse content search results per file on the
  server (one entry per path, best heading kept). Owner:
  @@Backend ([[phase-2/backend-2.md]]).
* [REVIEW] S3b: collapse content search results per file on the
  frontend so the rendered window stays single-per-file even when
  the backend returns multiple. Owner: @@Frontend
  ([[phase-2/frontend-3.md]]).
* [REVIEW] S4: clicking a search row whose hit is a heading shows
  file details in the inspector, not heading details (addressed by
  per-file row collapse + inspector layout fix). Owner: @@Frontend
  (covered by [[phase-2/frontend-1.md]] +
  [[phase-2/frontend-3.md]]).
* [REVIEW] S5: Search overlay inspector position fix; header spans
  the overlay, inspector hide button sits under the overlay close
  button. Owner: @@Frontend
  ([[phase-2/frontend-1.md]]).

### Search / Code / Graph: language elevation

* [REVIEW] L1: graph view elevates languages as graph nodes connected
  only to folders. Rank folders by file-count of each language;
  rank drives depth. Endpoint: `GET /api/graph/languages`. Wire
  shape FROZEN. Owner: @@Backend
  ([[phase-2/backend-4.md]]).
* [REVIEW] L2: graph overlay gains a Language filter chip and a
  language graph mode powered by L1; Search Status `Graph this`
  routes to the language graph at max depth. Owner: @@Frontend
  ([[phase-2/frontend-8.md]]). webtest-2 confirmed
  the launch path and language-mode canvas render.

### Cross-cutting

* [DONE] H1: syseng hardening pass on `/api/graph` FS-truth, the
  WS watcher coupling for live add/delete, and the new language
  graph route. Owner: @@Syseng
  ([[phase-2/syseng-2.md]] +
  [[phase-2/syseng-3.md]]). All four surfaces
  APPROVED (backend-1, backend-3 + rustacean-2, backend-4,
  frontend-7 /ws contract); full live-fixture matrix passed; depth-cap
  re-run against fs-graph route confirmed; non-blocker residual
  (empty-drive lang-graph unit test) landed.
* [DONE] R1: @@Rustacean Rust review pass on phase-2 backend work.
  Owner: @@Rustacean
  ([[phase-2/rustacean-1.md]]). backend-1..4 +
  rustacean-2/3 mirrors all APPROVED for commit; full gate green
  in both repos. Non-blocker nits captured for follow-up.
* [REVIEW] T1: webtest smoke extension. Owner: @@Webtest
  ([[phase-2/webtest-2.md]]). Search overlay
  layout / row collapse / SearchStatus Graph-this green at desktop.
  Ghost / live-add / depth-cap browser smoke pending pickup of the
  scratch fixture path; routed via
  [[phase-2/architect-8.md]] and
  [[phase-2/architect-9.md]].
* [BACKLOG] F1: chan-report reconcile-on-load + bulk-create gap.
  Non-blocking phase-2 follow-up. Owner: @@Backend
  ([[phase-2/backend-5.md]]).
* [TODO] F2: swap the `#` glyph on fs-graph folder nodes for a
  folder icon so the visual stops colliding with semantic-graph
  tags (root cause of the "tags from source code" symptom Alex
  flagged on screenshot). Owner: @@Frontend
  ([[phase-2/frontend-10.md]]).

## Dispatch

| Task         | Owner       | Status   | Depends on |
|--------------|-------------|----------|------------|
| architect-1..7| @@Architect| HANDOFF  | idle-agent acks |
| architect-8  | @@Architect | REVIEW   | webtest finding triage + scratch path |
| syseng-1     | @@Syseng    | PREP     | folded into journal Decisions |
| backend-1    | @@Backend   | REVIEW   | -          |
| backend-2    | @@Backend   | REVIEW   | -          |
| backend-3    | @@Backend   | REVIEW   | -          |
| backend-4    | @@Backend   | REVIEW   | FROZEN wire shape in backend-4 |
| backend-5    | @@Backend   | BACKLOG  | non-blocking report reconcile follow-up |
| rustacean-1  | @@Rustacean | DONE (acked) | backend-1..4 + rustacean-2/3 APPROVED |
| rustacean-2  | @@Backend   | APPROVED | mirrors backend-3 freeze; reviewed in rustacean-1 |
| rustacean-3  | @@Backend   | APPROVED | mirrors backend-4 freeze; reviewed in rustacean-1 |
| rustacean-4  | @@Rustacean | TODO     | non-blocker tidies from rustacean-1; awaiting @@Architect approval (A/B/C) |
| frontend-1..9| @@Frontend  | REVIEW   | every phase-2 frontend lane in REVIEW |
| frontend-10  | @@Frontend  | TODO     | fs-graph folder-glyph swap (F2 follow-up) |
| syseng-2     | @@Syseng    | DONE (acked) | four surfaces approved + live probe matrix recorded |
| syseng-3     | @@Syseng    | DONE (acked) | depth.ts + empty-drive lang-graph residual + hardening re-run |
| architect-syseng-1 | @@Architect | DONE | depth-cap close-out folded into syseng-3 |
| architect-9  | @@Architect | REVIEW   | explicit @@Webtest handoff for ghost/live-add/depth-cap probes |
| webtest-1    | @@Webtest   | IN_PROGRESS | shared 8788 service |
| webtest-2    | @@Webtest   | REVIEW   | full smoke matrix passed at desktop+narrow; report workaround applied |

## Critical path

```
backend-1..4 + rustacean-2/3 (REVIEW; rustacean-1 + syseng-2 APPROVED) ┐
frontend-1..9 (REVIEW; rustacean-1 + syseng-3 APPROVED depth path) ────┤
webtest-2 (REVIEW; full smoke matrix passed) ──────────────────────────┴─> commit
frontend-10 (folder-glyph swap) ──> ride-along on commit if it lands
backend-5 ──> backlog (non-blocker)
```

## Notes & decisions

(Decisions Alex locked are mirrored here so every task brief shares
the same source of truth.)

1. **`#tag` gate stays where it is.** `Drive::parse_for_graph` in
   chan-drive already drops `Token::Tag { .. }` for non-`.md` files
   and the indexer never opens source-class text. backend-1 shipped
   the explicit regression tests; no further widening planned.
2. **FS-truth lives in chan-server.** `/api/graph` stats each
   indexed file with `std::fs::symlink_metadata` and emits stale
   rows as `missing: true` file nodes (existing
   `GraphNodeView::File` shape). chan-drive's `graph().files()`
   contract is unchanged. See backend-3 + rustacean-2.
3. **Search content collapse splits across backend + frontend.**
   `/api/search/content` collapses on the server (8x candidate pool
   capped at 200 per page); SearchPanel collapses what is actually
   returned in the current window. Frontend collapse stays as
   defense-in-depth and so tag/contact/image rows still dedupe
   against chunk rows.
4. **Live add/delete in the graph overlay rides /ws.** No new
   socket event in phase 2. frontend-8 binds the same watcher
   bridge the file browser already uses. Bulk-event debounce is
   the frontend's concern.
5. **Language graph is a new endpoint, not a flag.** Final
   endpoint: `GET /api/graph/languages?depth=<n>&language=<name>`.
   It returns `{max_depth,nodes,edges}` with language nodes, folder
   nodes, and ranked language edges. Shape is FROZEN in backend-4.
6. **Depth-slider cap is per-scope on the frontend.** No new
   backend route; the slider reads the current scope's structural
   max. file=1, group=N, dir=max child depth from the loaded
   fs-graph, drive=fs-graph diameter (computed from a one-shot
   `/api/fs-graph?scope=folder&path=&depth=6` call already in
   place). `truncated: true` clamps the slider at 6.
7. **E2 wording sweep is a strict rename.** No behavior change.
   Targets: AccessoryPill tooltips + `aria-label`, Pane
   `emptyPaneNavigation` labels.

## Decisions still owed

(none)

### Closed in this phase

* E2b wording sweep across AccessoryPill + Pane empty-pane
  navigation. **Closed: not doing.** Those buttons act on overlays
  with no file antecedent; "Show File" / "Graph this" are file-
  aware affordances and only make sense in places like the
  FileEditorTab kebab (already covered by frontend-2). Surface
  again only if Alex flags it.

Closed:
* Depth-slider cap source: reuse the existing `/api/fs-graph`
  payload that GraphPanel already fetches in filesystem mode and
  the loaded `nodes` array in content mode. No new backend route.
  Captured in frontend-9.
* L1 surfaces both `files` and `code` on each language node + edge.
  Confirmed by the wire freeze in backend-4 (already implemented).

## Folded items from syseng-1 (PREP)

[[phase-2/syseng-1.md]] is the syseng pre-architect
survey. The lane mapping and tag-extraction survey there are
authoritative for phase 2 syseng scope and are reproduced as
decisions above. The two concrete tasks it proposed are dispatched
as:

* syseng-1 itself stays as the survey record. No flips to DONE.
* backend-1 / backend-3 already absorbed the two implementation
  proposals (tag regression test + FS-truth in `/api/graph`).
* Remaining syseng work is consolidated into syseng-2
  (hardening pass + watcher race review + new language-graph
  route review), filed alongside webtest-2 in the smoke matrix.

## Log

* 2026-05-16 @@Architect: read [[phase-1/summary.md]]
  and [[phase-2/request.md]]. Audited the current
  code surfaces for each work item against the chan + chan-core
  trees.
* 2026-05-16 @@Architect: drafted the initial phase 2 journal +
  architect-1 audit and the first wave of task scaffolds.
* 2026-05-16 @@Architect: reconciled the journal after discovering
  parallel agent activity had already produced backend-1, backend-2,
  backend-3, frontend-1..6, architect-1..4 handoffs, and syseng-1
  prep work between the audit pass and the journal write. Updated
  the dispatch table to reflect actual filenames and statuses.
  REVIEW work flagged for specialist review; TODO work narrowed to:
  frontend-7 (depth slider per-scope cap), frontend-8 (live
  watcher consumption), frontend-9 (E2 wording sweep across pills
  + empty-pane labels), backend-4 (language graph endpoint),
  frontend-10 (language filter + graph mode), syseng-2 (consolidated
  hardening), webtest-2 (smoke extension). Decisions Alex already
  locked through syseng-1 are now mirrored in `Notes & decisions`
  so every task brief shares the same source of truth.
* 2026-05-16 @@Backend: reconciled language graph work into
  [[phase-2/backend-4.md]]. Canonical endpoint is
  `GET /api/graph/languages?depth=<n>&language=<name>` and
  `cargo test -p chan-server` passed with 99 tests.
* 2026-05-16 @@Backend: filed
  [[phase-2/backend-5.md]] as the non-blocking
  chan-report reconcile follow-up from [[phase-2/architect-8.md]].
  Also pinned the empty language-graph builder case with a unit
  test.
* 2026-05-16 @@Architect: kickoff dispatch closed. Reconciled the
  journal once more after frontend-7 (G4 live watcher), backend-4
  (L1 wire freeze), and frontend-8 (L2 + S2 extension) all landed
  while the dispatch was being written. Created
  [[phase-2/frontend-9.md]] for the scope-aware
  depth cap (G1) and [[phase-2/webtest-2.md]] for
  the smoke coverage matrix; [[phase-2/syseng-2.md]]
  was already started by @@Syseng and is IN_PROGRESS. E2b wording
  sweep is HELD pending Alex direction (defaults to keep "Files" /
  "Graph" on the AccessoryPill / Pane empty-pane menus because
  they target overlays without a file antecedent). Next coordination
  step is routing the REVIEW work for specialist review: @@Rustacean
  for backend-1..4 + rustacean-2/3, @@Syseng for backend-3 +
  frontend-7 via syseng-2, @@Webtest for the frontend-1..7 visual
  pass via webtest-2.
* 2026-05-16 @@Architect: second sweep. frontend-9 landed in REVIEW
  (G1 scope-aware depth cap with `web/src/graph/depth.ts` + Vitest
  coverage). frontend-8 flipped from IN_PROGRESS to REVIEW after
  webtest-2 confirmed Search Status -> language-graph launch and
  the language-mode canvas render. webtest-2 itself flipped to
  REVIEW with green search overlay layout / row collapse / Search
  Status Graph-this smoke and three remaining gaps (ghost-while-
  open, live-add-while-open, depth-cap browser smoke). Triaged
  the webtest "code report shows only Markdown" finding in
  [[phase-2/architect-8.md]]: workaround is
  `rm /tmp/chan-dev/.chan/report.jsonl` + restart, real fix filed
  as non-blocking [[phase-2/backend-5.md]].
  Scratch fixture path `/tmp/chan-dev/Scratch/phase2-smoke/`
  assigned for destructive smoke. Phase-2 implementation work is
  now entirely in REVIEW. The remaining gating items are: the four
  specialist reviews inside [[phase-2/syseng-2.md]]
  (backend-3 + rustacean-2 lstat, backend-4 fan-out + rank order,
  frontend-7 watcher debounce, backend-1 tag regression tests);
  three remaining browser smoke probes inside webtest-2 (ghost,
  live-add, depth-cap); @@Rustacean review of the new chan-server
  routes (backend-1..4 / rustacean-2/3). The E2b wording sweep
  stays HELD.
* 2026-05-16 @@Syseng: [[phase-2/syseng-2.md]] REVIEW.
  Drafted the consolidated syseng-2 brief and executed the hardening
  pass against `/tmp/chan-syseng-phase2-fixture` (markdown + .txt +
  source-class + symlink + FIFO + a deletable indexed `.md`). Approved
  backend-1 (markdown-gate regression tests), backend-3 + rustacean-2
  (`/api/graph` FS-truth via `symlink_metadata`), backend-4
  (`/api/graph/languages` rank/depth/case-insensitive filter), and the
  /ws-side contract for frontend-7 (live-reload signal with 250ms
  debounce, closed-overlay latching, no subscription leak). Live
  probe matrix recorded in syseng-2.md: stat-on-emit ghosts a
  freshly-`rm`'d `.md` immediately with `missing:true`; watcher then
  forgets the row (`indexed_docs` 5→4); replacing an indexed `.md`
  with a symlink lets the phase-1 watcher hardening forget the row
  (5→3) without leaving a stale ghost. Language graph live probes
  confirmed empty-filter, case-insensitive filter, and depth-1 fan-out.
  Full gate ran clean: chan-server 99 tests, chan-drive 429 tests,
  fmt + clippy + `--no-default-features` build green,
  `npm run check` 0/0/3911, vitest 9/111. Non-blocking residuals:
  optional empty-drive unit test for `build_language_graph`; webtest
  browser smoke for delete/create/rename-while-open is owned by
  @@Webtest in [[phase-2/webtest-2.md]].
* 2026-05-16 @@Rustacean: [[phase-2/rustacean-1.md]] DONE.
  Rust review pass on backend-1..4 + rustacean-2/3 mirrors. All four
  items APPROVED for commit. Gate green: chan-server 100 passed
  (was 92 pre-phase-2), chan 50, chan-drive 429,
  `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`
  + `cargo build --no-default-features` clean across both repos.
  Targeted re-run of new tests:
  `routes::search::tests` 3 + `routes::graph::tests` 5 new + 9 prior
  = 17 passed. Non-blocker improvements recorded in rustacean-1.md
  (one-line rustdoc on `indexed_file_exists`, dedupe
  `normalized_content_limit(0)` against `default_content_limit()`,
  named locals in `expanded_content_candidate_limit`, three
  tie-break / root-folder tests for `build_language_graph`); none
  block the commit. @@Rustacean idle, ready for next assignment.

  Aside: the "tags from source code" symptom Alex flagged on
  screenshot earlier in this kickoff was a UX read of the frontend
  `#` glyph used for fs-graph folder nodes, not a backend leak.
  backend-1's regression tests + the static analysis on the
  chan-drive gate confirm the indexer never emits tag edges from
  non-markdown files. Live probe at
  `curl 'http://127.0.0.1:8788/api/fs-graph?scope=folder&path=Source/chan/web&depth=1'`
  returned only `folder`/`file` nodes with `contains` edges. Frontend
  follow-up (not @@Rustacean scope): swap the `#` glyph for a folder
  icon on fs-graph `kind: "folder"` nodes so the glyph stops
  colliding with semantic-graph tag nodes.

* 2026-05-16 @@Syseng: [[phase-2/syseng-3.md]] REVIEW.
  Picked up the depth-cap close-out flagged in my idle ping.
  Reviewed `web/src/graph/depth.ts` against the fs-graph route's
  symlink/hardlink/truncation semantics: symlink boundary holds
  (the route never traverses symlink targets, so the consumer can't
  count past a symlink), hardlink dedup doesn't affect the slider
  max (deepest hardlink path is still at most the file-tree depth),
  and `truncated` (node-count-bound, not depth-bound) correctly
  clamps the slider to `fsMax=6` defensively. Live probe against a
  fresh fixture with on-disk depth 8 confirmed the route returns
  `truncated:false` + visible paths capped at depth 6, so the
  slider correctly tops out at 6 = the route's MAX_DEPTH. Empty-drive
  `build_language_graph` unit test was already landed by @@Backend
  (`language_graph_empty_drive_returns_empty_payload`); my
  non-blocking residual from syseng-2 is closed. Gates re-checked
  green: `cargo test -p chan-server` empty-drive case ok,
  `npm test -- --run graph/depth` 6/6 pass. No follow-up. No new
  blockers. Closing [[phase-2/architect-syseng-1.md]]
  as DONE.
* 2026-05-16 @@Architect: third sweep. Acked rustacean-1, syseng-2,
  syseng-3 as DONE in the dispatch table after reading their review
  bodies. Phase-2 implementation is now APPROVED across Rust quality,
  Cargo hygiene, filesystem / watcher / process hardening, and
  per-surface live-probe matrices. Only remaining gate before
  commit: three @@Webtest browser smoke probes (ghost-while-open,
  live-add-while-open, depth-cap) which still need pickup of the
  scratch fixture path I assigned in
  [[phase-2/architect-8.md]]. Filed an explicit
  @@Webtest handoff in [[phase-2/architect-9.md]]
  with the probe matrix + scratch path so the next webtest agent
  cycle can close out T1 without needing more context. Also picked
  up @@Rustacean's review aside about the "tags from source code"
  symptom: root cause was the `#` glyph chan reuses for fs-graph
  folder nodes colliding with semantic-graph tag nodes (not a
  backend leak — backend-1 + rustacean-1 confirmed the gate). Filed
  as [[phase-2/frontend-10.md]] (F2 follow-up) so
  it can ride along on the phase commit if it lands in time.
* 2026-05-16 @@Rustacean: filed
  [[phase-2/rustacean-4.md]] as a ride-along
  proposal to land the four non-blocker tidies recorded in
  rustacean-1.md. Scope is two chan-server files
  (`routes/search.rs`, `routes/graph.rs`) plus one optional
  chan-core comment (`drive.rs`); no wire-shape, no dependencies,
  no behavior changes. Decision asked of @@Architect (A: chan-server
  only / B: include chan-core comment / C: defer entirely). Default
  if no direction: A. Will not start until @@Architect picks an
  option.
* 2026-05-16 @@Webtest: cycle close-out. Recorded final smoke
  state in webtest-1.md and webtest-2.md, stopped the 8788
  listener (was PID 15857), and committed the phase-2 smoke runner
  + review notes (commit 6f33351). Architect-9 probes are
  already green in webtest-2: ghost-while-open, live-add-while-open,
  and depth-cap. Type-gap fixes in web/src/state/store.svelte.ts,
  web/src/components/GraphCanvas.svelte, and
  web/src/components/GraphPanel.svelte stay uncommitted for the
  architect's phase commit to bundle with frontend-8.
* 2026-05-16 @@Webtest B: picked up phase-2 webtest lane in
  parallel with @@Webtest A's architect-9 re-run (see webtest-2.md
  "@@Webtest A architect-9 re-run" section, all five smoke probes
  green, 8788 torn down). G3 depth-cap is doubly confirmed by A's
  run; not re-claiming. Frontend-10 (folder-glyph swap) is still
  TODO with @@Frontend, so I prepped the wire-shape probe in
  [[phase-2/webtest-smoke.mjs]] — a luminance
  histogram drift between an fs-graph folder-scope canvas and a
  semantic-graph canvas, post-swap assertion gated behind
  CHAN_WEBTEST_GLYPH_PROBE=1 so the default matrix stays green
  pre-swap. Probe is syntax-checked but not yet runtime-verified;
  full matrix re-run is pending the architect's phase commit and
  a fresh 8788. Details in
  [[phase-2/webtest-2.md]] "@@Webtest B
  frontend-10 wire-shape probe prep".

* 2026-05-16 @@Architect: fourth sweep + commit plan freeze.
  Decisions:
  - rustacean-4 OPTION A approved. Routed to @@Rustacean via
    [[phase-2/architect-rustacean-3.md]]. Polish
    folds into the chan-server bundle commit. Item 3 (chan-core
    one-line comment) DEFERRED to keep the single chan-core commit
    tight.
  - frontend-10 landed independently while I was writing the
    decision. REVIEW. Rides along on the web bundle commit.
    @@Webtest B can now flip `CHAN_WEBTEST_GLYPH_PROBE=1` and
    runtime-verify against a rebuilt service.

  Commit plan (frozen):
  1. **chan-core** (sibling repo, one commit):
     subject: `chan-drive: scope tag extraction to markdown files`
     files: `crates/chan-drive/src/drive.rs`,
            `crates/chan-drive/src/fs_ops.rs`,
            `crates/chan-drive/tests/file_types.rs`.
     content: backend-1's `is_markdown_file` gate + tag-token
     filter for non-`.md` indexed paths + regression tests.
  2. **chan** commit 1/3:
     subject: `chan-server: phase-2 backend + polish`
     content: backend-2 (per-file content-search collapse),
     backend-3 (`/api/graph` FS-truth via `symlink_metadata`),
     backend-4 (`/api/graph/languages` endpoint), plus
     rustacean-4 option A polish on `routes/search.rs` and
     `routes/graph.rs`.
     files: `crates/chan-server/src/lib.rs`,
            `crates/chan-server/src/routes/{graph,mod,search}.rs`.
  3. **chan** commit 2/3:
     subject: `web: phase-2 frontend (search, editor, graph)`
     content: frontend-1..9 + frontend-10. Includes search overlay
     layout fix, per-file row collapse, SearchStatus Graph-this,
     editor list indent guides, file menu wording, graph
     force-layout ring, watcher-driven live reload, language
     graph mode + filter chip, scope-aware depth slider cap,
     and the fs-graph folder-glyph swap.
     files: `web/src/api/{client,types}.ts`,
            `web/src/components/{FileEditorTab,GraphCanvas,
            GraphPanel,SearchPanel,SearchStatusOverlay}.svelte`,
            `web/src/editor/{Wysiwyg.svelte,decorations/blocks.ts,
            decorations/blocks.test.ts}`,
            `web/src/state/{store.svelte.ts,store.test.ts}`,
            plus new files under `web/src/graph/` and
            `web/src/search/`.
  4. **chan** commit 3/3:
     subject: `release: close phase 2 tasks`
     content: every `phase-2/*.md` task file
     not already in commit 6f33351, plus webtest-1/2.md note
     updates, plus the phase summary
     `phase-2/summary.md` (authored at commit
     time).

  Gate before each commit: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo test -p chan-server`, `cargo test -p chan`,
  `cargo build --no-default-features`. `scripts/pre-push` runs
  the same gate end-to-end and is the architect's final check
  on the chan repo.

  Sequencing: chan-core first (commit 1). Then in chan, run
  commits 2 → 3 → 4 in order. Each commit's content is
  independently green-gated; bundling lets the phase ship as a
  coherent unit while keeping per-area history.

  Pending before the commit pass kicks off:
  - @@Rustacean implements rustacean-4 option A.
  - @@Webtest B runtime-verifies the frontend-10 visual probe.
  - Architect runs `scripts/pre-push` on the assembled tree.
  - Architect writes `phase-2/summary.md` as
    part of commit 4.
* 2026-05-16 @@Webtest B: frontend-10 runtime-verify complete.
  Rebuilt the frontend + release binary with @@Frontend's
  `GraphPanel.mapFsNodes` swap in place, rotated the stale code
  report, restarted 8788, and ran the full smoke matrix with
  `CHAN_WEBTEST_GLYPH_PROBE=1`. All six probes green, including
  the post-swap folder-glyph wire-shape probe (drift 0.7578).
  Caveat: the glyph probe's hard assertion is trivially satisfied
  by scene composition, not glyph identity — it functions as an
  end-to-end render check rather than a precise regression test.
  Details + tear-down in [[phase-2/webtest-2.md]]
  "@@Webtest B post frontend-10 full matrix re-run". Item 2 of the
  fourth-sweep commit plan is now done; only @@Rustacean
  (rustacean-4 option A) and the architect's pre-push gate remain.
