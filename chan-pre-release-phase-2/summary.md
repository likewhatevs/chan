# Chan Pre-Release Phase 2 Summary

Status: implementation, specialist reviews, hardening, and browser
smoke are all DONE. Phase 2 ships in four commits: one in the
sibling `chan-core` checkout (`cfea9ec`) plus three in the main
`chan` repo (`6a844db`, `0f83533`, plus this commit closing
`release: close phase 2 tasks`).

## Outcome

Every work item in [[request.md]] landed. Decisions Alex locked
mid-phase via [[syseng-1.md]] (FS-truth locus, tag-regression-only
disposition, self-upgrade follow-ups out of scope) and via the
architect commit plan in [[journal.md]] are reflected in the four
shipped commits.

## What landed

- Graph
  - `/api/graph` stats indexed files with `symlink_metadata` and
    emits stale rows as `missing: true` so the display truth
    follows the live drive (backend-3 / rustacean-2 / G1a).
  - GraphCanvas spreads multi-focal seed nodes in a deterministic
    ring instead of pinning every focal to origin (frontend-6 / G2).
  - Open graph overlay consumes the existing `/ws` watcher bridge
    via a debounced reload signal so new and deleted files reflow
    without manual reload (frontend-7 / G4).
  - Depth slider clamps to a per-scope maximum derived from the
    loaded graph (`web/src/graph/depth.ts`); single-file = 1,
    group = N, directory = tree depth, drive = fs-graph diameter
    clamped at the route's `MAX_DEPTH=6` (frontend-9 / G3).
- Editor
  - Per-line list decorations carry nesting depth; vertical guides
    render at every indent level (frontend-5 / E1).
  - FileEditorTab kebab uses "Show File" and "Graph this" for
    file-scoped actions (frontend-2 / E2a).
- Search
  - `chan-drive::parse_for_graph` drops `Token::Tag` for non-`.md`
    paths; regression tests pin the contract (chan-core backend-1
    / S1).
  - `/api/search/content` collapses per-file with an 8x widened
    candidate pool; the ContentHit wire shape is unchanged
    (backend-2 / S3a). SearchPanel reapplies the same collapse on
    the rendered window (frontend-3 / S3b).
  - SearchPanel inspector lives in a flex sibling of the results
    column; the hide control anchors under the overlay close
    button (frontend-1 / S4 + S5).
  - SearchStatus Code Report carries a "Graph this" action that
    opens the language graph at backend max depth (frontend-4 / S2).
- Search / Code / Graph language elevation
  - New `/api/graph/languages` endpoint returns
    `{max_depth, nodes, edges}` with language nodes connected only
    to folders, ranked per-language by file count then SLOC then
    path. Optional `depth` clamp and case-insensitive `language`
    filter (backend-4 / L1).
  - Graph overlay adds a `language` mode with a language filter
    chip; SearchStatus "Graph this" routes here at max depth
    (frontend-8 / L2).
- Follow-ups absorbed
  - fs-graph folder nodes now render with a folder glyph instead
    of the semantic-graph `#` glyph. This was the root cause of
    the "tags from source code" symptom Alex screenshotted at
    kickoff and was confirmed by @@Rustacean review; the chan-drive
    tag-gate was already correct (frontend-10 / F2).
  - rustacean-1 non-blocker tidies folded into the chan-server
    bundle: rustdoc on `indexed_file_exists` +
    `collapse_hits_by_file`, default-limit unification on
    `normalized_content_limit(0)`, named locals on the candidate
    pool formula, defensive `u32::try_from` on the language-graph
    `max_depth`, three new `build_language_graph` tie-break
    regression tests (rustacean-4 option A).

## Highlights

- Wire-shape freezes early in [[rustacean-2.md]] and
  [[rustacean-3.md]] let the frontend land
  [[frontend-3.md]] / [[frontend-8.md]] in parallel with the
  backend implementation. No frontend rework.
- The "tags from source code" complaint resolved to a pure
  frontend glyph collision; @@Rustacean's static analysis +
  backend-1 tests showed the chan-drive gate was already correct,
  and frontend-10 fixed the actual UX bug.
- @@Webtest A's first run with the recycled Opus 4.7 agent
  applied the report-finding workaround end-to-end without
  re-asking, including discovering that the active report JSONL
  lives in `~/Library/Application Support/chan/report/` instead
  of `.chan/` for this drive shape.

## Lowlights

- The phase-2 directory accumulated three series of architect
  "idle agent" handoff acks (architect-1..7) that were not the
  formal release-surface audit slot; the audit ended up folded
  into the journal Notes section + [[syseng-1.md]] survey rather
  than a single architect-N task. Workable but messier than
  phase 1's `architect-1` audit pattern.
- Initial journal draft used the wrong agent-name convention
  (`webdev-N` / `rustacean-N` instead of
  `frontend-N` / `backend-N`); reconciled in the same turn but
  it briefly forked the dispatch table.

## Bugs found and fixed

- Phase-1-era `chan-report` does not reconcile against the live
  filesystem at load time, so files copied into the drive between
  chan-serve runs do not appear in the report until the JSONL is
  removed and the server restarted. Surfaced by @@Webtest as a
  blocker for the language graph smoke; deferred to
  [[backend-5.md]] as a non-blocking follow-up because the
  workaround restart is reliable.
- fs-graph folder nodes shared the `tag` canvas kind with
  semantic-graph tag nodes and therefore drew the `#` glyph. Now
  distinct: fs-graph folders carry `kind: "folder"` and the canvas
  loads `PATH_FOLDER` as a stroked folder outline (frontend-10).

## Test and hardening coverage

- Rust gate on chan: 50 + 103 = 153 tests; `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features` clean. `scripts/pre-push`
  green end-to-end.
- Rust gate on chan-core: `cargo test -p chan-drive` 429 passing.
- Web gate: `npm run check` 3911 files / 0 errors / 0 warnings;
  `npm test -- --run` 9 files / 111 tests passing.
- Browser smoke: phase-2 runner at
  [[webtest-smoke.mjs]] passes the architect-9 probe matrix at
  desktop and narrow viewports (search overlay layout, per-file
  row collapse, SearchStatus -> language graph, ghost-while-open,
  live-add-while-open, depth-cap, post-swap folder-glyph drift).
  Service is torn down at the end of every run.
- Syseng hardening: [[syseng-2.md]] approved backend-1, backend-3
  + rustacean-2, backend-4, frontend-7's /ws contract against a
  live fixture (`/tmp/chan-syseng-phase2-fixture`) with markdown,
  `.txt`, source-class, symlink, FIFO, and deletable indexed
  fixtures. [[syseng-3.md]] approved `web/src/graph/depth.ts`
  against the fs-graph route's symlink / hardlink / truncation
  semantics with a fresh on-disk-depth-8 fixture.

## Remaining follow-ups

- [[backend-5.md]] (BACKLOG): chan-report reconcile-on-load +
  bulk-create gap. Not on the phase-2 critical path; the manual
  workaround (delete the persisted JSONL and restart) is
  documented in [[architect-8.md]].
- The fs-graph folder-glyph histogram probe is a smoke test for
  "both canvases render and produce non-empty pixels", not a
  precise glyph-identity regression test. A future frontend test
  hook (`window.__chanGraphRenderedKinds` or similar) would
  replace it with a precise assertion (recorded by @@Webtest B
  in [[webtest-2.md]]).
- Signed release checksums + auto-rollback on self-upgrade
  remain phase-1 carry-overs and are still post-release work.
- E2b wording sweep (extend "Show File" / "Graph this" to the
  AccessoryPill tooltips + the Pane empty-pane navigation) is
  closed as **not doing**: those buttons act on overlays with no
  file antecedent. Surface if Alex flags it.

## Agent quality

1. **@@Backend** (codex). Picked up backend-1..4 without an
   architect dispatch and shipped four coherent surfaces with
   the right wire-shape discipline. Empty-drive language-graph
   unit test landed on its own from @@Syseng's residual ping
   without needing routing.
2. **@@Syseng** (codex + Opus 4.7 recycle). Pre-architect survey
   in [[syseng-1.md]] was the load-bearing scoping artifact for
   the phase: the tag-extraction code reading was definitive
   and the FS-truth locus recommendation matched what shipped.
   syseng-2 + syseng-3 carried a full live-probe matrix and
   approved every surface with named gates. Recycle came up
   clean and idled cleanly through architect-syseng-2.
3. **@@Webtest** (codex, then recycled Opus 4.7 x2). Codex cycle
   built the phase-2 smoke runner and recorded the report-only-
   Markdown finding that drove [[backend-5.md]]. Opus 4.7 cycle
   ran the architect-9 matrix end-to-end on the first cycle,
   added the frontend-10 wire-shape probe, ran both pre-swap
   and post-swap matrices, and tore down cleanly each time.
4. **@@Rustacean** (codex, then Opus 4.7 recycle). Codex review
   pass in [[rustacean-1.md]] was the cleanest Rust review of
   the phase: per-task verdicts with specific code references
   plus non-blocker nits, plus the design-aside that resolved
   the "tags from source code" symptom into a frontend glyph
   collision. Recycle landed the option-A polish without
   prompting.
5. **@@Frontend** (codex). Largest implementation lane (1..10).
   Shipped every search / editor / graph surface plus the post-
   review folder-glyph swap without rework. The frontend-7
   debounce window (250 ms) ended up exactly the right number
   for both interactive and bulk filesystem events.
6. **@@Architect** (this agent). Kickoff dispatch correctly
   identified the audit scope but used the wrong agent-name
   convention on the first journal pass; reconciled in the
   same turn. The mid-phase decision triage (rustacean-4
   option A, frontend-10 in-scope, E2b not doing) shipped the
   surfaces the user actually asked for without scope creep.

## Constructive feedback

- The architect-N idle-agent acks accumulated faster than the
  formal architect audit slot. Next phase: file `architect-1.md`
  as the audit task first, and reserve idle handoffs to
  `architect-idle-N.md` or fold them into journal log entries
  to keep the architect lane readable.
- The chan-report reconcile gap was visible from the watcher
  fan-out code reading and could have been flagged at the
  phase-1 carry-forward review rather than as a phase-2
  smoke finding. Worth a syseng pre-flight pass on every phase
  surface that consumes chan-report or the graph DB.
- The wire-shape freeze pattern continues to work: every
  cross-boundary task that froze its payload sample early
  (rustacean-2, rustacean-3, backend-4) shipped without
  frontend rework. Worth promoting from convention to checklist
  for phase 3.
- The webtest glyph-probe histogram is a useful "both canvases
  render" smoke but reads as a regression test it does not
  guarantee. Adding a frontend-owned test hook would let a
  future probe assert glyph identity precisely.
- Phase-2 surface implementation is mostly closed against a
  single drive shape (`/tmp/chan-dev` with a workspace copy at
  `Source/chan-workspace-copy`). Phase 3 should include a
  multi-drive smoke fixture so the language graph and depth
  cap derivations are exercised against varied corpora.
