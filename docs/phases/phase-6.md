# Phase 6 - filesystem as the primary graph layer

Status: closed
Span: 2026-05-17 to 2026-05-19 (bulk on 2026-05-18; see Rounds and waves)
Versions: v0.10.0
Tags: #features #graph #editor #terminal #bugfixes

## Roadmap (the asks)

Two blocks of work were requested together.

Architectural block:

- Make the filesystem the primary graph layer. "Graph this" from the drive
  becomes the default scope; non-markdown files become first-class graph nodes.
- Add filesystem-specific file classification: devices, symlinks, hardlinks,
  read-only or locked directories as graph dead-ends; markdown treated specially
  (regular vs frontmatter, with a `chan` kind registry); tags and mentions
  indexed only from markdown; text files shown with a source editor plus
  chan-report data; binary files with minimal info.
- Standardize on "directory" and codemod "folder" out of the codebase.
- Add a royal-pink color slot for code/language so it no longer collides with
  the green tag color.

Bugs and nits block:

- New-file dialog quick-start, editor "New File" using the current file's
  parent directory, terminal theme refresh on dark/light switch, an Inspector
  toggle, an outside-overlay menu, Copy Path actions, terminal right-click
  expansion, enumerated Terminal-N names, same-name tab disambiguation, a
  stuck-shell Ctrl+D hint, tab-rename propagation into the PTY environment, and
  Shift+Enter reaching agents in the embedded terminal.

## Rounds and waves

Single-round phase. Work proceeded in parallel task lanes dispatched from a
shared flat journal file. Date-stamp counts in the source files show 5 entries
on 2026-05-17, 139 on 2026-05-18, and 1 on 2026-05-19; the bulk ran in a
single concentrated session on 2026-05-18 with a short lead-in and a thin tail.
Git commit dates are not reliable for this phase (one later migration commit
touched the tree).

The phase closed with six area-grouped commits plus a wrap commit, and cut
v0.10.0 for both the workspace and the desktop bundle.

## Team and coordination

See ../agents/README.md for full agent cards. Handles active this phase:

- @@Architect: plan, design memo (graph layering, color, permissions),
  reviews, wrap.
- @@Frontend: sole frontend slot, 15 task lanes covering graph UI flip, UX
  bundles, terminal chords, and rich-prompt overlay. (Card later renamed
  FullStack A/B.)
- @@Backsystacean: combined Backend + Syseng + Rustacean slot covering the file
  classifier, kind registry, unified /api/inspector and /api/graph, /api/health,
  PTY hooks, and the folder-to-directory codemod. (Card later split into
  FullStack + Systacean.)
- @@WebtestA: live test service, browser smoke, latency probes, regression hunt.
- @@WebtestB: parallel API-level probe scenarios.

Coordination scheme: flat task files at the phase root named `{agent}-{n}.md`,
dispatched by the architect through a single shared `journal.md` that held a
checklist, a capacity proposal, a dispatch table, an "Extended requests" table
for mid-phase additions, and a decisions log.

This was the last phase on the flat-task-file scheme. Phase 6 stood up the new
per-author-directory and append-only-journal format during its own wrap, which
phase 7 then adopted. The self-dispatch pattern (lanes opened wave-1 task files
before the journal existed) carried over from prior phases and cost one
reconciliation cycle.

## What shipped, tried, and undone

Shipped (v0.10.0):

- The filesystem as the primary graph layer; drive as the default scope
  everywhere; "Graph this" renamed "Graph from here".
- A workspace file classifier covering regular / symlink / hardlink / FIFO /
  socket / device / read-only; read-only directories are graph dead-ends;
  off-drive symlinks render but do not traverse.
- A frontmatter `chan` kind registry (contact as the first entry), canonical
  nested shape, tag and mention edges restricted to markdown nodes.
- A unified GET /api/inspector across drive / directory / markdown / text /
  media / binary / special, with byte-based report rollups; a merged /api/graph;
  a /api/health indexer-state block with a ghost poll while the inspector is
  open.
- Language bound to directory and drive; royal-pink code/language color slot.
- Terminal: Terminal-N enumeration, Shift+Enter and modifier-Enter chords, a
  Ctrl+D stuck-shell hint, tab-name environment variable at spawn, and PTY CWD
  metadata over the websocket.
- UX bundle: new-file quick-start, parent-dir New File, Copy Path, theme
  refresh, Inspector toggle, right-click expansion, same-name tab
  disambiguation, rich-prompt overlay.

Tried then corrected:

- PTY CWD lookup first used an unsafe FFI block. The server crate carries a
  forbid-unsafe lint and the build broke at HEAD; the lane pivoted to a
  subprocess lookup via `lsof` on macOS and `/proc` on Linux in the same
  session.
- Frontmatter doc shape was initially drafted as flat `chan.kind` shorthand
  instead of the registry's nested shape; the fixture and design memo had to be
  rewritten after the parser was read directly.
- The broadcast model was re-architected from asymmetric source-to-targets to
  a symmetric peer group.
- Tab-rename to PTY environment was investigated but not implemented as a live
  environment mutation; a spawn-time-only contract with a restart prompt was
  chosen instead.

Deliberately deferred (parked to a 6.1 follow-up):

- The broad `folder`-to-`directory` identifier codemod over wire-adjacent
  strings (user-visible copy landed, identifier sweep did not).
- A cosmetic chip-counter overcount.
- A graph scope breadcrumb.

Investigated, not reproduced:

- A WYSIWYG trailing-buffer glitch. The surface is CodeMirror 6, not
  ProseMirror; only a defensive teardown on tab switch was added.

## Retrospective

Highlights:

- The headline architectural ask landed end to end in a single session. @@Alex
  spotted the gap live (graph chip counts read zero for language/media/folder
  because /api/graph emitted only markdown-centric nodes); the producer fix
  merged the filesystem and language graphs into /api/graph, was
  contract-reviewed, and shipped the same day.
- Parallel webtest probes found five real defects: hardlink double-count,
  missing frontmatter-kind field, asymmetric frontmatter shape, missing symlinks
  in a listing, filesystem-graph special-file collapse. All were folded into a
  single fix bundle rather than scattered commits.
- The forbid-unsafe build break was caught fast and resolved in-session without
  needing a follow-up round.

Lowlights:

- The unsafe PTY block was introduced before the build was checked, leaving HEAD
  broken for a transient window. Forbid lints at the crate level are
  non-negotiable; checking the build before committing load-bearing changes is
  the minimum bar.
- Two early frontmatter docs showed the flat shorthand instead of the nested
  registry shape because the design was drafted against an assumed parser shape.
  Reading the existing parser in the first design pass would have prevented two
  rewrite cycles.
- @@Architect orientation lagged again: three lanes were in flight before the
  journal existed, requiring a reconciliation pass. This was the trigger for
  switching to per-author directories in phase 7.
- The desktop bundle version was missing from the version-bump file list, so the
  first DMG shipped with a stale label and had to be rebuilt.

Lessons (carry forward):

- Post the architect plan entry before wave-1 dispatch so the canonical plan
  starts the clock and lanes pause their self-dispatch instinct. This was the
  direct motivation for the per-author-journal format adopted in phase 7.
- Run `cargo build` or `cargo check` before committing to HEAD when a crate
  carries a load-bearing forbid lint. CI catching it is too late if other lanes
  have already branched from that HEAD.
- Read the existing source (the parser, the state model) in the first design
  pass. Two reworks here came from designing against an assumed shape rather than
  the real one.
- Include `desktop/src-tauri/tauri.conf.json` in the version-bump file list so
  the DMG label tracks the workspace version. Version drift between binary and
  installer is a user-visible defect.

## Notes

Terminology drift active in this phase:

- "folder" was being codemoded to "directory". User-visible copy landed in this
  phase; the identifier sweep over wire-adjacent strings was deferred to 6.1.
- "drive" refers to the workspace root directory (the user's chan drive), not
  the cloud product or tunnel domain.
- "rich-prompt overlay" is the earlier name for what was later called Team Work.

Raw working material (per-author journals, task/request/roadmap files,
coordination logs) lives in git history under `docs/journals/phase-6/`. That
tree was removed from the working tree during the phase-15 docs cleanup and is
not present in the current checkout.
