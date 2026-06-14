# Phase 2 - graph, editor, and search hardening

Status: closed
Span: 2026-05-16, one working day (estimate; all dated log entries read 2026-05-16; git dates are not usable because the whole tree landed in one 2026-05-18 migration commit)
Versions: none
Tags: #features #bugfixes #graph #editor #search #indexing

## Roadmap (the asks)

@@Alex framed the asks as "a list of items for this phase of product hardening, correctness, and UX impact":

**Graph**
- Clamp the depth slider per scope (single file = 1, N files = N, folder by subfolder count, drive knows its max depth).
- Give folder-scope nodes force against each other instead of stacking.
- Make the filesystem the source of truth before plotting so non-existent files are not shown; ghost files deleted while the graph is open; add new files that fit the current filter live.

**Editor**
- Vertical guidance for nested enumerated lists.
- Menu wording: "Graph -> Graph this" and "Files -> Show File".

**Search**
- Index `#tags` only from markdown, never from source code.
- A "Graph this" link in the search code report.
- Collapse search results per file and rank headings within the file.
- Show file details (not the heading section) in the inspector; move the inspector inside the search overlay with the hide control under the close button.

**Language as graph**
- Extend the existing "language" search elevation to the graph: whole-drive "Graph this" at max depth, language nodes connecting only to folders ranked by file count, a language filter in the overlay.

## Rounds and waves

Single-round phase. The architect dispatched work through flat task files (`{agent}-{n}.md`) at the phase root. Several lanes produced work in parallel before the journal was finalized, and the architect reconciled the dispatch table to the actual filenames afterward.

Mid-phase, the codex @@Webtest agent crashed the browser extension and was replaced by two Opus 4.7 agents, one dual-roling as @@Backend. All work resolved in a single day.

## Team and coordination

Agent roster is in `../agents/README.md`. Legacy handles used this phase:

```
Handle        Role this phase
-----------   -------------------------------------------------
@@Architect   plan, dispatch, journal, summary
@@Backend     HTTP boundary, content-search collapse,
              FS-truth graph, languages endpoint
@@Frontend    overlays, list guides, force layout, live
              reload, language UI (largest lane)
@@Syseng      filesystem/watcher/process hardening; ran the
              pre-architect survey that scoped the round
@@Rustacean   Rust quality, Cargo hygiene, polish
@@Webtest     test service + smoke runner; ran as A/B after
              a mid-phase agent recycle
```

Coordination scheme: flat task files at the phase root named `{agent}-{n}.md`, dispatched by the architect through a single shared `journal.md` that carried the dispatch table, critical path, dated log, and decisions. There was no separate event-channel file; events were dated entries in the journal. Architect handoffs and acks were filed as their own task files (`architect-1.md` through `architect-9.md` plus role-suffixed variants).

The startup was non-linear: lanes produced work before the journal landed and the architect reconciled the dispatch table to actual filenames afterward. The wire-shape-freeze discipline was applied: the languages payload and FS-truth shape were frozen before the frontend consumed them, enabling UI lanes to land in parallel with no rework.

## What shipped, tried, and undone

**Shipped**

- Graph FS-truth: `/api/graph` stats indexed files and emits stale rows as `missing: true`; files are ghosted on delete; new matching files are added live via the existing watcher bridge (debounced).
- Multi-focal layout: seed nodes spread in a deterministic ring instead of pinning every focal to the origin.
- Scope-aware depth cap clamped per scope and to the route maximum.
- Editor: per-line list decorations with vertical guides at each indent; "Show File" and "Graph this" menu wording.
- Search: drop tag tokens for non-markdown paths (regression tests confirmed the existing gate was already correct; no code change needed); per-file result collapse with a widened candidate pool; inspector relocated inside the overlay; a code-report "Graph this" link.
- Language elevation: a new `/api/graph/languages` endpoint (language nodes connect only to folders, ranked by file count then SLOC), plus a graph overlay language mode and filter.

**Tried, then corrected**

- Extending "Show File" / "Graph this" wording to overlay-only buttons was held and then closed as not-doing: those buttons act on overlays with no file antecedent, so "Files" / "Graph" stay accurate.
- The FS-truth fix had two candidate loci; the stat-on-emit option in the server was chosen to keep the workspace contract unchanged (the alternative would have widened the workspace boundary).

**Deliberately deferred**

- A chan-report reconcile-on-load gap was left in the backlog. The documented workaround is to delete the persisted report file and restart.

## Retrospective

**Highlights**

- Every request item landed.
- The wire-shape-freeze discipline held: freezing the languages payload and FS-truth shape before frontend consumption let UI lanes proceed in parallel with no rework.
- The "tags from source code" complaint turned out to be a frontend glyph collision (filesystem folder nodes drew the `#` tag glyph), not a backend tag leak. Regression tests proved the backend gate was already correct; the frontend fixed the actual UX bug. Diagnosing the real locus before patching saved wasted work.

**Lowlights**

- Idle-agent acks accumulated as many separate files, crowding out a clean single audit slot.
- The initial journal draft used the wrong agent-name convention and briefly forked the dispatch table before a same-turn fix.
- The folder-glyph webtest probe used a luminance-histogram render check, not a precise glyph-identity test, so its hard assertion is weak against future glyph changes.

**Lessons**

- File the audit task first; route idle handoffs to a single `architect-idle-N.md` or fold them into the journal log to keep the architect lane readable.
- Promote the wire-shape-freeze pattern from convention to an explicit checklist step at dispatch time.
- Add a frontend-owned test hook that asserts graph glyph identity precisely rather than by histogram; the histogram approach passes on accidental luminance matches.
- Exercise language-graph and depth-cap logic against more than one corpus shape; this phase tested a single drive.

## Notes

**Terminology drift**

- "drive" in phase-2 usage means the user's workspace folder (the root directory chan operates on). This later standardized to "workspace" and the crate was renamed from `chan-drive` to `chan-workspace`.
- "folder" was used interchangeably with "directory" in this phase's docs.

Raw working material (per-author journals, task/request/roadmap files, coordination logs) lives in git history under `docs/journals/phase-2/`.
