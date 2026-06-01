# Phase 6 - filesystem as the primary graph layer

Status: closed
Span: 2026-05-17 to 2026-05-19, bulk on 2026-05-18 (estimate; see
Duration)

Tags: #features #graph #editor #terminal #bugfixes

## Initial asks

Source: `raw/request.md`. The phase ran parallel tracks
over a diverse item set. The architectural block:

- Make the filesystem the primary graph layer. "From now on, the primary
  layer of the graph is the filesystem, starting from the drive."
  "Graph this" from the drive becomes the default scope.
- Filesystem-specific file classification (devices, symlinks, hardlinks,
  read-only or locked directories as graph dead-ends); markdown treated
  specially (regular vs frontmatter, with a `chan` kind registry);
  tags and mentions parented and indexed only from markdown; text files
  shown with a source editor plus chan-report data; binary files with
  minimal info.
- Terminology: standardize on "directory" and codemod "folder" out.
- A new color slot for code/language (royal pink) so it no longer
  collides with the green tag color.

A bugs-and-nits block followed: a new-file dialog quick-start, editor
"New File" using the current file's parent directory, terminal theme
refresh on dark/light switch, an Inspector toggle, an outside-overlay
menu, Copy Path actions, terminal right-click expansion, enumerated
`Terminal-N` names, same-name tab disambiguation, a stuck-shell `^D`
hint, tab-rename propagation into the PTY environment, and Shift+Enter
reaching agents in the embedded terminal.

## Team, profiles, and coordination

Cards under `../../agents/`, mapped via
[../../agents/README.md](../../agents/README.md).

```
handle           role this phase                       card
---------------  -----------------------------------   ----------------
@@Architect      plan, design memo (graph layering,    architect.md
                 color, permissions), reviews, wrap
@@Frontend       sole frontend slot, 15 task lanes:    frontend.md
                 graph UI flip, UX bundles, terminal    (-> FullStack A/B)
                 chords, rich-prompt overlay
@@Backsystacean  combined Backend + Syseng + Rustacean backsystacean.md
                 slot: file classifier, kind registry,  (-> FullStack +
                 unified /api/inspector and /api/graph,    Systacean)
                 /api/health, PTY hooks, codemod
@@WebtestA       live test service, browser smoke,      webtest-a.md
                 latency probe, regression hunt
@@WebtestB       parallel API-level probe scenarios     webtest-b.md
```

Coordination scheme: flat task files at the phase root named
`{agent}-{n}.md`, dispatched by the architect through a single shared
`journal.md` (checklist, capacity proposal, dispatch table, an
"Extended requests" table for mid-phase additions, and a decisions log).
This was the last phase on the flat-task-file scheme: phase 6 stood up
the new per-author-directory and append-only-journal format during its
own wrap, which phase 7 then adopted. The self-dispatch pattern carried
over (lanes opened wave-1 task files before the journal existed) and cost
one reconciliation cycle.

## Duration

Estimate: 2026-05-17 to 2026-05-19. Basis: in-file date stamps run 5 on
2026-05-17, 139 on 2026-05-18, and 1 on 2026-05-19, so the bulk ran on
2026-05-18 with a short lead-in and a thin tail. Git dates are not usable
(one later migration commit).

## Highlights and lowlights

Highlights:
- The headline architectural ask landed end to end. Alex spotted the gap
  live (graph chip counts read zero for language/media/folder because
  `/api/graph` emitted only markdown-centric nodes); the producer fix
  merged the filesystem and language graphs into `/api/graph`, was
  contract-reviewed, and shipped inside one session.
- The parallel webtest probes found five real defects (hardlink
  double-count, a missing frontmatter-kind field, an asymmetric
  frontmatter shape, missing symlinks in a listing, filesystem-graph
  special-file collapse), folded into a single fix bundle.
- A build break was caught fast: an `unsafe` block collided with the
  server's forbid-unsafe lint; the lane pivoted to a subprocess lookup in
  the same session.

Lowlights:
- That same WIP introduced `unsafe` before the build was checked, so HEAD
  broke for a transient window.
- Two early frontmatter docs showed the flat `chan.kind` shorthand
  instead of the registry's nested shape; the fixture and memo had to be
  rewritten. Reading the parser before drafting the memo would have
  caught it.
- Architect orientation lagged again (three lanes in flight before the
  journal existed).
- The desktop bundle version was missing from the version-bump file list,
  so the first DMG shipped with a stale label and had to be rebuilt.

## Constructive feedback

- With per-author journals arriving in phase 7, post the architect plan
  entry before wave-1 dispatch so the canonical plan starts the clock and
  other agents pause their self-dispatch instinct.
- Run `cargo build` or `cargo check` before HEAD writes when a crate
  carries a load-bearing forbid lint.
- Read the existing source (the parser, the state model) in the first
  design pass; two reworks here came from designing against an assumed
  shape.
- Add `desktop/src-tauri/tauri.conf.json` to the version-bump file list
  so the DMG label tracks the workspace version.

## What shipped, tried, and undone

Shipped (workspace and desktop bundle to 0.10.0, in six area-grouped
commits plus a wrap commit):
- The filesystem as the primary graph layer; drive as the default scope
  everywhere; "Graph this" renamed "Graph from here".
- A workspace file classifier (regular / symlink / hardlink / FIFO /
  socket / device, plus read-only); read-only directories are graph
  dead-ends; off-drive symlinks render but do not traverse.
- A frontmatter `chan` kind registry (contact the only entry), canonical
  nested shape, tag and mention edges pinned markdown-only.
- A unified `GET /api/inspector` across drive / directory / markdown /
  text / media / binary / special, with byte-based report rollups; a
  merged `/api/graph`; a `/api/health` indexer-state block with a ghost
  poll while the inspector is open.
- Language bound to directory and drive; a royal-pink code/language color
  slot.
- Terminal work: `Terminal-N` enumeration, Shift+Enter and modifier-Enter
  chords, a Ctrl+D stuck-shell hint, a tab-name environment variable at
  spawn, and PTY CWD metadata over the websocket.
- A broad UX bundle (new-file quick-start, parent-dir New File, Copy Path,
  theme refresh, Inspector toggle, right-click expansion, same-name tab
  disambiguation, the rich-prompt overlay).

Tried then changed:
- The PTY CWD lookup first used unsafe FFI; reverted to shelling out to
  `lsof` on macOS and `/proc` on Linux.
- The frontmatter doc shape was corrected from flat to nested after the
  parser was read directly.
- The broadcast model was re-architected from asymmetric source-to-targets
  to a symmetric peer group.
- Tab-rename to PTY environment was investigated but not implemented as a
  live environment mutation; the spawn-time-only contract with a restart
  prompt was chosen instead.

Parked to a 6.1 follow-up: the broad `folder` to `directory` identifier
codemod over wire-adjacent strings (user-visible copy already landed), a
cosmetic chip-counter overcount, and a graph scope breadcrumb.

Investigated, not reproduced: a WYSIWYG trailing-buffer glitch (the
surface is CodeMirror 6, not ProseMirror); only a defensive teardown on
tab switch was added.

## Raw material

Raw working material (per-author journals, task/request/roadmap files,
coordination logs) is preserved in git history under this phase's `raw/`
tree; it was removed from the working tree in the phase-15 docs cleanup.

The request file originally embedded one screenshot of the new-file
dialog; per the journals-wide image removal it is now a short text note
in the source request.
