# Phase 2 - graph, editor, and search hardening

Status: closed
Span: 2026-05-16, one working day (estimate; see Duration)

## Initial asks

Source: [raw/request.md](raw/request.md), framed as a "list of items for
this phase of product hardening, correctness, and UX impact". The work
items, as written:

- Graph: clamp the depth slider per scope (single file = 1, N files = N,
  folder by subfolder count, drive knows its max depth); give
  folder-scope nodes force against each other instead of stacking; make
  the filesystem the source of truth before plotting so non-existent
  files are not shown; ghost files deleted while the graph is open; add
  new files that fit the current filter live.
- Editor: vertical guidance for nested enumerated lists; menu wording
  "Graph -> Graph this" and "Files -> Show File".
- Search: index `#tags` only from markdown, never from source code; a
  "Graph this" link in the search code report; collapse search results
  per file and rank headings within the file; show file details (not the
  heading section) in the inspector; move the inspector inside the search
  overlay with the hide control under the close button.
- Language as graph: extend the existing "language" search elevation to
  the graph (whole-drive "Graph this" at max depth, language nodes
  connecting only to folders ranked by file count, a language filter in
  the overlay).

## Team, profiles, and coordination

Legacy handles map to current cards via
[../../agents/README.md](../../agents/README.md).

```
handle       role this phase                           card
-----------  ---------------------------------------   ------------------
@@Architect  plan, dispatch, journal, summary          architect.md
@@Backend    HTTP boundary, content-search collapse,   backend.md
             FS-truth graph, languages endpoint        (-> FullStack A/B)
@@Frontend   largest lane: overlays, list guides,      frontend.md
             force layout, live reload, language UI     (-> FullStack A/B)
@@Syseng     filesystem/watcher/process hardening;     syseng.md
             the pre-architect survey scoped the round  (-> Systacean)
@@Rustacean  Rust quality, Cargo hygiene, polish       rustacean.md
                                                        (-> Systacean)
@@Webtest    test service + smoke runner; ran as A/B    webtest.md
             after a mid-phase agent recycle            (-> Webtest A/B)
```

Coordination scheme: flat task files at the phase root named
`{agent}-{n}.md`, dispatched by the architect through a single shared
`journal.md` (dispatch table, critical path, dated log, decisions).
Events were dated entries in the journal; there was no separate
event-channel file. Architect handoffs and acks were themselves filed as
task files (`architect-1.md` .. `architect-9.md` plus role-suffixed
variants). The startup was non-linear: several lanes produced work in
parallel before the journal landed, and the architect reconciled the
dispatch table to the actual filenames afterward. Mid-phase, the codex
@@Webtest agent crashed the browser extension and was replaced by two
Opus 4.7 agents, one dual-roling as @@Backend.

## Duration

Estimate: a single day, 2026-05-16. Basis: every dated log entry across
`journal.md` and the task files reads 2026-05-16; no other date appears.
Git dates are not usable here because the whole tree lands in one
2026-05-18 migration commit.

## Highlights and lowlights

Highlights:
- Every request item landed.
- The freeze-the-wire-shape discipline held again: the languages payload
  and the FS-truth shape were frozen before the frontend consumed them,
  so the UI lanes landed in parallel with no rework.
- The "tags from source code" complaint turned out to be a frontend glyph
  collision (filesystem folder nodes drew the `#` tag glyph), not a
  backend tag leak. Regression tests proved the backend gate was already
  correct; the frontend fixed the actual UX bug.

Lowlights:
- The architect idle-agent acks accumulated as many separate files and
  crowded out a clean single audit slot.
- The initial journal draft used the wrong agent-name convention and
  briefly forked the dispatch table before a same-turn fix.
- The folder-glyph webtest probe is a luminance-histogram render check,
  not a precise glyph-identity test, so its hard assertion is weak.

## Constructive feedback

- File the audit task first, and route idle handoffs to a single
  `architect-idle-N.md` (or fold them into the journal log) to keep the
  architect lane readable.
- Promote the wire-shape-freeze pattern from convention to an explicit
  checklist.
- Add a frontend-owned test hook so a future probe can assert graph glyph
  identity precisely instead of by histogram.
- Exercise language-graph and depth-cap logic against more than one drive
  shape; this phase tested a single corpus.

## What shipped, tried, and undone

Shipped:
- Graph FS-truth: `/api/graph` stats indexed files and emits stale rows
  as `missing: true`; ghosted on delete; new matching files added live
  via the existing watcher bridge (debounced).
- Multi-focal layout: seed nodes spread in a deterministic ring instead
  of pinning every focal to the origin.
- Scope-aware depth cap clamped per scope and to the route maximum.
- Editor: per-line list decorations with vertical guides at each indent;
  "Show File" / "Graph this" menu wording.
- Search: drop tag tokens for non-markdown paths (regression tests only,
  no code change, since the existing gate was already correct); per-file
  result collapse with a widened candidate pool; inspector relocated
  inside the overlay; a code-report "Graph this" link.
- Language elevation: a new `/api/graph/languages` endpoint (language
  nodes connect only to folders, ranked by file count then SLOC), plus a
  graph overlay language mode and filter.

Tried, decided against, or deferred:
- Extending the "Show File" / "Graph this" wording to overlay-only
  buttons was held, then closed as not-doing: those buttons act on
  overlays with no file antecedent, so "Files" / "Graph" stay accurate.
- The FS-truth fix had two candidate loci; the stat-on-emit option in the
  server was chosen to keep the workspace contract unchanged.
- A chan-report reconcile-on-load gap was left in the backlog; the
  documented workaround is to delete the persisted report file and
  restart.

## Raw material

- Source request: [raw/request.md](raw/request.md)
- Architect journal: [raw/journal.md](raw/journal.md)
- Phase summary (outcomes, highlights, agent ranking, feedback):
  [raw/summary.md](raw/summary.md)
- The syseng pre-architect survey, lane task files, and architect handoff
  files live alongside them in [raw/](raw/).
