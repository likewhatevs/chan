# phase-18 round-1 - execution plan

The Lead's dispatch guide. `bootstrap.md` (generated) holds the team
PROCESS and is what every agent reads on spawn; THIS file holds the round
WORK. The launch poke points @@Lead here.

Self-identify: each agent knows its handle from `$CHAN_TAB_NAME`
(@@Lead, @@LaneA .. @@LaneF). Map it to your lane in the table below. If
`$CHAN_TAB_NAME` is unset or not in the roster, STOP and ask @@Lead.

Source spec: round-1/draft.md (@@Alex's v0.26.0 TODO) + its image*.png.
Read your lane's items there verbatim before starting; the anchors below
came from a recon pass and drift, so re-verify against HEAD.

## Roster -> lane

+---------+-------------------------------------------------------------+
| handle  | lane                                                        |
+---------+-------------------------------------------------------------+
| @@Lead  | coordination, gate, commits, surveys to @@Alex (no code     |
|         | lane); drives the round-close repo/docs cleanup with F      |
| @@LaneA | Editor (markdown lists, scroll, `[[` autocomplete)          |
| @@LaneB | Graph (selection, edges, contact node, auto-reload, links)  |
| @@LaneC | File Browser (context menus, shortcut hints, loading hang)  |
| @@LaneD | Inspector (pill + dropdown redesign across item types)      |
| @@LaneE | Terminal + chan-desktop (focus, copy/paste, UTF-8, preflight)|
| @@LaneF | Repo / docs cleanup (journals -> docs/phases, agents trim)  |
+---------+-------------------------------------------------------------+

Goal: land every item in round-1/draft.md for v0.26.0. Release-quality
bar: full gate green, no known bug shipped, empirical smoke of every
behavioral change. Pre-release: no back-compat, drop legacy outright.

## Lane assignments + owned files

Lanes are coherent domains, not fixed file enumerations (the phase-17
lesson). Edit ONLY files in your domain; if a fix pulls you into another
lane's file, STOP and route through @@Lead. Anchors are starting points;
re-verify line numbers against HEAD.

----------------------------------------------------------------------
@@LaneA - Editor
----------------------------------------------------------------------
Owns: web/src/editor/decorations/blocks.ts, web/src/editor/commands/
      list.ts, web/src/editor/bubbles/{triggers.ts,wiki.ts},
      web/src/editor/widgets/image.ts, web/src/editor/Wysiwyg.svelte.
draft.md "Editor":
- Bullet + hyphen list cursor/indent/click parity with ENUMERATED lists.
  Today arrow-down between unordered items lands the cursor BEFORE the
  glyph. Ordered-list cursor behavior is the reference to match. blocks.ts
  (BULLET_* decorations, ~422-519), list.ts (continueListOnEnter ~81-104,
  indent/outdent ~114-126).
- Restore distinct HYPHEN lists. Phase-17 made bullet lists match Google
  Docs; hyphen lists regressed into bullets. blocks.ts glyph mapping
  (~445-471) currently ignores the source marker char. @@Alex wants
  hyphen (`-`) lists visually distinct again.
- Trackpad free-scroll hang. Scroll stalls / jumps opposite then settles
  when the cursor is far from the scroll target. No editor-wide wheel
  handler found; suspect a CM layout cycle or image-widget remeasure
  (widgets/image.ts ~102 marks wheel; Wysiwyg.svelte scrollIntoView ~598,
  CSS scroll-behavior ~773). Reproduce, then fix.
- `[[` workspace-PATH autocomplete. The bubble exists (bubbles/triggers.ts
  computeBubbleSpec ~86-94, bubbles/wiki.ts openWikiBubble) and queries
  /api/link-targets (filename fuzzy). @@Alex wants it to complete LOCAL
  WORKSPACE PATHS. If the route must return directory/path candidates,
  that is a chan-server touch: flag to @@Lead (cross-lane) before editing
  any route.

----------------------------------------------------------------------
@@LaneB - Graph
----------------------------------------------------------------------
Owns: web/src/components/{GraphPanel.svelte,GraphCanvas.svelte},
      web/src/state/graphData.svelte.ts,
      web/src/state/store.svelte.ts (graph region: watcher ~625-672,
        graph-open fns ~2054-2107, graphReloadSignal ~1915),
      web/src/state/tabs.svelte.ts (GraphTab ~326-351),
      web/src/state/tabMenu.svelte.ts (graph tab-menu region),
      crates/chan-workspace indexer (contact-stamp),
      crates/chan-server graph route (wire kinds).
draft.md "Graph":
- "Graph from here" must SELECT the originating node. graphFromHere already
  sets pendingSelectId + selectedId (GraphPanel ~466-495); verify the
  selection actually renders/highlights post-fetch (the select is dropped
  today per @@Alex).
- Directory nodes plotted with NO visible edge to the workspace root (e.g.
  a Drafts folder outside the workspace, or src/). visibleEdges requires
  both endpoints in scope (~1254-1277); the parent-spine invariant
  (~1142-1150) is not landing. images: round-1/image-5.png, image-12.png.
- Binary / symlink rendered as a CONTACT node (image-10.png). classifyFile
  honors a node_kind:"contact" stamp from the chan-workspace indexer; the
  indexer is stamping a binary/symlink as contact. Cross-crate: Rust
  indexer kind + the TS mirror MUST stay in lockstep.
- STOP the auto-reload. The graph reloads every few seconds / on ANY
  workspace file edit, even a file not at the root and not in the graph.
  store.svelte.ts watcher (~633) calls invalidateGraph() + bumps
  graphReloadSignal unconditionally on every event; GraphPanel debounces
  but does not path-filter (~2150-2169). Gate invalidation on whether the
  changed path is in the current graph scope. This is the highest-signal
  graph bug.
- Enhancement "Copy link to graph": add to the Graph TAB right-click menu
  (replace the "Reload" button) a link that reproduces the graph
  (serialize GraphTab fields: scopeId/depth/mode/filters/selected), and
  make such a link openable from a markdown file.

----------------------------------------------------------------------
@@LaneC - File Browser
----------------------------------------------------------------------
Owns: web/src/components/{FileBrowserSurface.svelte,FileTree.svelte,
        HamburgerMenu.svelte}, web/src/components/menuClamp.ts,
      web/src/state/store.svelte.ts (persist region: persistStateToHash
        ~1569-1598),
      web/src/App.svelte (layout-persist effects ~160-217),
      web/src/state/shortcuts.ts (FB chord additions).
draft.md "File Browser":
- Context-menu regression: the tab right-click menu got merged with the
  docked file-browser menu. Remove "Reload". Below "Expand all
  directories" add (acting from the WORKSPACE ROOT): "New file or
  Directory", "New Terminal", "New Graph". FileBrowserSurface menu
  (~637-748), FileTree in-row menu (~1353-1417), existing handlers
  newFileOrDir/terminalFromHere/graphThis (~552-657).
- Show keyboard-shortcut hints in the context menu (image-6.png): New
  Terminal cmd+t, New Graph cmd+shift+m, Delete = backspace, Settings
  cmd+, . Read them from the central store (shortcuts.ts chordFor; empty
  menu-row-chord spans exist at FileBrowserSurface ~669-706). Record any
  missing chord in shortcuts.ts so it ports to linux/macos/web.
- Loading hang. Expanding a directory stalls on "Loading"; console shows
  "SecurityError: history.replaceState more than 100 times / 10s"
  (image-9.png). Root cause: expand -> persistLayoutToHash effect
  (FileBrowserSurface ~250-259) -> persistStateToHash (store ~1597) calls
  history.replaceState with no debounce. Debounce / coalesce the hash
  write.

----------------------------------------------------------------------
@@LaneD - Inspector
----------------------------------------------------------------------
Owns: web/src/components/{FileInfoBody.svelte,InspectorBody.svelte,
        Inspector.svelte}, web/src/terminal/fromHere.ts.
draft.md "Inspector": replace the flat action buttons with a single PILL
(main action) + dropdown (secondary actions) per item category. Restructure
FileInfoBody actionsSection (~720-823); reuse existing handlers (Open,
Upload, Download, View/Zoom, Show File, Graph-from-here). Per category:
- Directory: main "Open" (new file-browser tab); dropdown: Upload file
  here, Download tarball, New terminal here, Graph from here.
- File (editable): main "Open" (Hybrid Editor); dropdown: Download file,
  New terminal here, Graph from here.
- Media: main "View / Zoom"; dropdown: Download file, New terminal here,
  Graph from here.
- Binary (incl symlinks): main "Download file"; dropdown: Graph from here.
- Editor "Show Details": main "Show file" (file-browser tab, file
  selected); dropdown: Download file, New terminal here, Graph from here.
"New terminal here" seeds the terminal with "{cursor}{space}{relative-
path}". terminal/fromHere.ts (terminalFromHereTarget) already produces a
seeded payload; @@LaneD OWNS any seed-format change there. @@LaneC reuses
the helper as-is; if either needs a signature change, route through @@Lead.

----------------------------------------------------------------------
@@LaneE - Terminal + chan-desktop
----------------------------------------------------------------------
Owns: web/src/components/{TerminalTab.svelte,RichPrompt.svelte},
      web/src/state/richPrompt.svelte.ts,
      crates/chan-server/src/terminal_sessions.rs,
      desktop/src/main.js, web/src/components/PreflightOverlay.svelte,
      web/src/state/shortcuts.ts (terminal copy/paste chord additions).
draft.md "Terminal" + "chan-desktop":
- Hide rich prompt -> focus the terminal. Today hiding (menu or
  cmd+shift+p) leaves focus off the terminal. RichPrompt onDestroy
  (~223-228) / richPrompt.svelte.ts / TerminalTab toggle (~880-883) should
  return focus + cursor to the xterm instance on hide.
- Terminal context-menu chords + copy/paste (image-7.png). Find shows
  Cmd+F; Copy / Paste / Copy Scrollback have empty chord spans
  (TerminalTab ~1399-1432). Wire cmd+c / cmd+v copy/paste and show the
  hints; record chords in shortcuts.ts.
- UTF-8 garble in less AND vim (image-14.png less, image-15.png vim):
  multibyte UTF-8 renders as raw bytes. The PTY spawn env
  (terminal_sessions.rs ~813-880) sets TERM/COLORTERM but no LANG /
  LC_ALL / LC_CTYPE. Set a UTF-8 locale on spawn. Verify in BOTH less and
  vim on round-1/ ../../../config-reference.md style content.
- chan-desktop: the local-disk New-workspace flow still shows the OLD
  pre-flight dialog, conflicting with the SPA boot menu (image-1.png,
  image-2.png). Pre-flight moved to the SPA (PreflightOverlay.svelte) in
  phase-17; remove the desktop-side dialog (desktop/src/main.js renderLocal
  ~560-628, the compute_workspace_preflight scan UI) for the local path.

----------------------------------------------------------------------
@@LaneF - Repo / docs cleanup
----------------------------------------------------------------------
Owns: docs/journals/** (read), NEW docs/phases/**, docs/agents/**,
      docs/archive, .claude, .codex.
draft.md "Repo":
- Consolidate each phase's journals into docs/phases/phase-N.md (the
  phase's roadmap, rounds, waves, retrospective). Capture the ESSENCE so
  new agents can learn from prior execution, successes, and mistakes.
  Phases 1-16 are stable/done: fan out subagents (one per phase) in Wave 1.
  phase-17 and phase-18 fold in at close (phase-18's bus is live this
  round).
- Distill docs/agents into a MINIMAL referenced set + a lessons-learned
  playbook, kept under docs/agents/. Delete the rest.
- Deletions are the FINAL close-out step (see Wave 3): .claude, .codex,
  docs/archive, the trimmed docs/agents leftovers, and docs/journals.
  docs/phases/phase-N.md is text-only by default; keep a screenshot only
  if it is load-bearing. Scrub stale doc-comment path mentions in
  chan-workspace/embeddings.rs, chan-server/routes/graph.rs, pages.yml.
  NOTE: .claude / .codex are untracked (rm -rf); docs/* are tracked
  (git rm). Do NOT delete docs/journals/phase-18 until @@Lead confirms the
  round is committed (the live team bus + gate worktree depend on it).

## Shared-file contention (the only cross-lane coupling)

+--------------------------------+----------+----------------------------+
| file                           | lanes    | rule                       |
+--------------------------------+----------+----------------------------+
| web/src/state/shortcuts.ts     | C, E     | Both append to SHORTCUTS.   |
|                                |          | @@Lead sequences C then E   |
|                                |          | (one array literal). Run    |
|                                |          | `node web/scripts/          |
|                                |          | shortcuts-table.mjs` ONCE   |
|                                |          | after both land to resync   |
|                                |          | crates/chan/src/main.rs.    |
| web/src/terminal/fromHere.ts   | D owns,  | D owns seed-format changes; |
|                                | C uses   | C consumes as-is.           |
| web/src/state/store.svelte.ts  | B, C     | B = graph region (~625-672, |
|                                |          | 2054-2107); C = persist     |
|                                |          | (~1569-1598). Far apart;    |
|                                |          | .ts interleave-safe. @@Lead |
|                                |          | commits the merged file.    |
| web/src/state/tabs.svelte.ts   | B + tab  | .ts interleave-safe; @@Lead |
|                                | creators | merges.                     |
| web/src/state/tabMenu.svelte.ts| B, C     | State only; coordinate if   |
|                                |          | both add fields.            |
| web/src/App.svelte             | C, E     | C = layout effects          |
|                                |          | (~160-217); E = rich-prompt |
|                                |          | handler (~659). Far apart.  |
| crates/chan-server             | B, E     | Different files (graph route |
|                                |          | vs terminal_sessions.rs);   |
|                                |          | same crate. Land any shared- |
|                                |          | signature change in one     |
|                                |          | burst, re-`cargo check -p   |
|                                |          | chan-server` green before   |
|                                |          | pausing.                    |
+--------------------------------+----------+----------------------------+

Everything else is single-lane. The Rust FileClass / indexer kind and its
TS mirror (B) must stay in lockstep. shortcuts.ts additions must be
resync'd to main.rs.

## Wave plan

- Wave 1 (immediate, isolated): all 6 lanes start. A: lists + scroll +
  `[[`. B: auto-reload gate + dir-edges + contact-stamp + select. C:
  context menu + shortcut hints + loading-hang + root actions. D:
  inspector redesign. E: UTF-8 + rich-prompt focus + terminal menu +
  desktop pre-flight. F: consolidate phases 1-16 into docs/phases via
  subagent fan-out. Flag any shared-file touch to @@Lead BEFORE landing.
- Wave 2 (Lead-sequenced convergence + smokes): shortcuts.ts additions
  (C then E) + single resync; fromHere.ts (D owns, C consumes); @@Lead
  merges store / tabs / App / tabMenu. Empirical smokes per area on a test
  server (Chrome pre-granted; ask @@Alex via survey which client for
  WKWebView-specific items).
- Wave 3 (close-out, @@Lead + F): F folds phase-17 + phase-18 into
  docs/phases and distills docs/agents. @@Lead runs the full-tree
  `make pre-push` from the isolated gate.sh worktree, makes per-lane
  atomic commits, THEN the deletions (.claude, .codex, docs/archive,
  trimmed docs/agents, docs/journals) as the final step. Final validation
  pass + local DMG before any tag. NO push without @@Alex.

## Gate + quality bar

- Scoped own-gate before any "done" report:
  - Rust: cargo fmt --check + cargo clippy --all-targets -D warnings +
    cargo test (scoped -p <crate>). Re-check --no-default-features if you
    touch feature gates.
  - Frontend: make web-check (vitest; catches stale ?raw source-pins) +
    svelte-check + npm run build. Browser-smoke any Svelte-5
    $state/$derived reactivity change (static gates miss runtime errors).
  - Desktop (@@LaneE): cd desktop && make build.
- @@Lead owns the full-tree `make pre-push` from an isolated gate.sh
  worktree (gates committed state, immune to peers' WIP). Lanes report
  scoped own-gate-green + a pathspec sha; never block on the main-tree
  gate.
- Empirical smoke: rust-embed bakes the bundle at build time, so rebuild
  (npm run build -> cargo build -p chan) before smoking a frontend change,
  and grep the SERVED bundle (not source) when a flag "looks broken".
  Smoke: list cursor/glyphs/hyphen + free scroll + `[[`; graph
  select-on-from-here + dir-edges + no-spurious-reload + file-icon (not
  contact) + copy-link; FB menu + shortcut hints + no loading-hang + root
  actions; inspector pills per category; terminal hide-focus + copy/paste
  + UTF-8 in less AND vim; desktop local-disk New flow with no double
  dialog (WKWebView hand-smoke is @@Alex's; agents cannot drive WKWebView).
- Pre-release: no back-compat, no migration paths. Commit at round close;
  the coordination tree is distilled into docs/phases/phase-18.md (the
  raw journals tree is deleted, not committed). NO push without @@Alex.

## Open questions for @@Alex (Lead batches into one cs terminal survey)

- `[[` autocomplete: should it return only workspace PATHS, keep the
  existing filename/heading/block link-targets too, or both? (Decides
  whether @@LaneA needs a chan-server route change.)
- Smoke client: which surface to validate the SPA changes on first, Chrome
  automation or WKWebView (chan-desktop)? Some bugs (terminal render) are
  WKWebView-specific.
Survey shape: consolidate, one decision per survey, <=4 options. Workers
route these to @@Lead; @@Lead surveys @@Alex.
