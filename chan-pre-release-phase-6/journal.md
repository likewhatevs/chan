# Chan Pre-Release Phase 6 Journal

Owner: @@Architect. Host: Alex.

Source request: [request.md](./request.md). Process: [process.md](./process.md).

## Plan summary

Phase 6 runs parallel tracks across a wide surface: a structural refresh
of how the graph is rooted and inspected, a terminology codemod, a
new color slot for the language layer, and a bag of UX nits across the
file browser, file dialog, panes, overlays, and embedded terminal.

1. **Architectural cleanups (graph + filesystem layering).** Make the
   filesystem the primary graph layer. "Graph this" from anywhere
   defaults to the drive. The drive itself, directories, and files
   (markdown, text, binary) are first-class nodes with proper file
   classification (regular vs symlink vs hardlink vs device, plus
   permissions including read-only / locked directories as dead-ends).
   Frontmatter markdown stays the kind ladder (`chan.contact` today,
   `chan.{other}` later). #tags and @@mentions remain markdown-scoped.
   Language binds to directory in the graph and shows up in every
   inspector (drive included). Code gets its own color slot ("royal
   pink") so it stops colliding with green tags. Terminology codemod:
   "folder" out, "directory" / "dir" in.
2. **Bugs and nits.** New file dialog quick-starts with current dir +
   `untitled.md` pre-selected, no extra Tab step. New File from the
   editor menu opens in the same parent as the current file. Theme
   switch refreshes terminals. PANE right-click gets an Inspector
   toggle. Outside-overlay right-click shows the same two-button menu
   instead of the browser default. File browser and editor menu both
   gain Copy Path. Terminal top-bar status moves into the bubble menu;
   terminal right-click gains copy/paste plus Copy CWD, Show Dir,
   Graph dir, New Terminal, split-pane controls, search, and settings.
   New terminals get enumerated `Terminal-N` titles. Same-name tabs
   disambiguate via shortest common ancestor segment. `^D` on the shell
   detects the stuck state and prints `press ^D to close the tab`.
   Tab rename should propagate into the PTY environment (design TBD).
   Shift+Enter routes correctly to claude / codex / similar TUI
   programs inside the embedded terminal.
3. **Hardening close-out.** End-to-end run-throughs across every lane,
   the standard pre-push gate green, summary with outcomes,
   highlights, lowlights, bugs, coverage, follow-ups, and agent
   rankings.

## Repo state at phase start

* `chan` is on `main` at `12391d8 release: phase 5 wrap + 0.9.0 notes`,
  even with `origin/main` (phase 5 was pushed at close).
* Working tree clean except for the new `chan-pre-release-phase-6/`
  directory.
* Rust toolchain pinned at 1.95.0 per `rust-toolchain.toml`.

## Request checklist

### Architectural cleanups
- [~] Make the filesystem the primary graph layer; "Graph this" from
  any surface defaults to the drive scope.
  ([architect-2](./architect-2.md) design,
  [frontend-4](./frontend-4.md) REVIEW + [backsystacean-9](./backsystacean-9.md)
  REVIEW: server merge landed (`merge_filesystem_layer` +
  `merge_language_layer` fold `directory` / `file` / `media` /
  `language` nodes plus `contains` + `language` edges into
  `/api/graph`; directory ids align across layers; read-only
  dead-ends preserved; standalone `/api/fs-graph` +
  `/api/graph/languages` retained). Contract review PASS by
  @@Architect. Live verification owed on the test service.)
- [x] Drive inspector kept; directories, files, and the drive itself
  carry a unified inspector surface across file browser, graph, and
  search.
  ([backsystacean-3](./backsystacean-3.md) REVIEW payload +
  [frontend-4](./frontend-4.md) REVIEW: shared file inspector now
  consumes `/api/inspector` across file browser, graph, and search;
  directory inspectors use backend subtree counts with tree-derived
  fallback; chan-report COCOMO roll-up preserved alongside.)
- [x] File classifier in chan-drive: regular vs symlink vs hardlink
  vs FIFO vs socket vs device, plus read-only / locked permissions.
  Locked / read-only directories show as dead-ends in the graph.
  ([backsystacean-2](./backsystacean-2.md) REVIEW; contract matches
  [architect-2](./architect-2.md). Bonus fix: PTY spawn now clears
  inherited `CHAN_MCP_*` so `mcp_env=off` is honored under
  re-launch.)
- [~] Terminology codemod: drop "folder", standardize on "directory"
  ("dir" for short) across crates and web.
  ([backsystacean-5](./backsystacean-5.md) REVIEW,
  [frontend-5](./frontend-5.md))
- [x] Markdown layer: regular markdown vs frontmatter markdown.
  Existing `chan.kind: contact` (the contact rendering pill) stays,
  with the kind ladder extensible for future `chan.{other}`.
  ([backsystacean-4](./backsystacean-4.md) REVIEW)
- [x] `#tag` and `@@mention` indexing remains markdown-scoped. Confirm
  in code, document the boundary.
  ([backsystacean-4](./backsystacean-4.md) REVIEW)
- [x] Text files: keep the plain-text / source-code editor surface;
  accumulate chan-report data in the per-file inspector across file
  browser, graph, and search.
  ([backsystacean-3](./backsystacean-3.md) +
  [frontend-4](./frontend-4.md) REVIEW.)
- [x] Binary files: minimal inspector content (size + kind only by
  default).
  ([backsystacean-3](./backsystacean-3.md) +
  [frontend-4](./frontend-4.md) REVIEW.)
- [x] Language binds to directory in the graph and is present in
  every inspector (drive includes a full breakdown).
  Inspector side: ([backsystacean-3](./backsystacean-3.md) +
  [frontend-4](./frontend-4.md) REVIEW.) Graph side:
  ([backsystacean-9](./backsystacean-9.md) REVIEW: language nodes
  + language-to-directory edges fold into `/api/graph`; directory
  ids share the `directory:<path>` namespace so language edges
  land on the same node the filesystem layer renders.)
- [x] Royal-pink color slot for code, distinct from green tags.
  `--chan-color-language` / `--chan-color-code` tokens
  (`#C71585` light / `#FF4DB8` dark) wired through
  `--g-language`; GraphCanvas language fallback updated.
  ([architect-2](./architect-2.md),
  [frontend-4](./frontend-4.md) REVIEW)
- [x] Graph from here: rename "Graph this" to "Graph from here"
  across all surfaces (file tree row context menu, file editor
  menu, graph overlay openers, empty-pane menu). Drive is the
  default scope.
  ([frontend-4](./frontend-4.md) REVIEW)
- [x] Ghost-node indexer-progress: expose indexer state on
  `/api/health` and replace the static "try Reload / chan index"
  hint with the live status when the indexer is busy.
  ([backsystacean-7](./backsystacean-7.md) +
  [frontend-6](./frontend-6.md) REVIEW: ghost inspector polls
  `/api/health` 1/s while open; busy state renders
  `indexer is catching up (N event(s) pending)` /
  `indexer is rebuilding (full pass)`; idle / poll failure falls
  back to the static hint.)

### Bugs and nits
- [x] New file dialog: open directly with `current-dir/untitled.md`,
  stem pre-selected; remove the press-Tab prerequisite.
  ([frontend-1](./frontend-1.md) REVIEW)
- [x] "New File" from the editor menu uses the parent directory of the
  current file.
  ([frontend-1](./frontend-1.md) REVIEW)
- [x] Terminals refresh on dark/light theme switch without needing a
  Reload.
  ([frontend-1](./frontend-1.md) REVIEW)
- [ ] PANE right-click gains an Inspector toggle next to Reload.
  ([frontend-2](./frontend-2.md))
- [~] Outside-overlay right-click shows the same two-button menu
  (Reload + Inspector) instead of the browser default.
  ([frontend-1](./frontend-1.md) routed backdrop through the existing
  menu handlers; [frontend-2](./frontend-2.md) finishes the Inspector
  toggle once the PANE menu carries it.)
- [x] Copy Path action in the file browser (all files and dirs) and in
  the editor's tab menu.
  ([frontend-1](./frontend-1.md) REVIEW)
- [x] Terminal top-bar status row (`[size]` ... `[search] [copy]
  [restart]`) moves into the terminal's bubble menu.
  ([frontend-1](./frontend-1.md) REVIEW; status, missed-bytes, find,
  copy scrollback, restart, resume all relocated, Find opens as a
  transient in-terminal search box.)
- [ ] Terminal right-click: copy / paste basics plus Copy path to CWD,
  Show Dir, Graph dir, New Terminal, split-pane buttons, search
  buttons, settings.
  ([frontend-2](./frontend-2.md))
- [x] Enumerated `Terminal-N` names for fresh terminals.
  ([backsystacean-1](./backsystacean-1.md) REVIEW)
- [x] Disambiguate same-name file tabs with shortest common
  ancestor; full path on hover; existing "show in file browser"
  remains the precise jump.
  ([frontend-3](./frontend-3.md) REVIEW: `tabLabelInPane` groups
  same-basename file tabs and renders the shortest divergent
  segment; deep divergent tails collapse as `x/[...]/foo.md`;
  full-path hover preserved.)
- [x] `^D` on the embedded shell detects the stuck state and prints
  `press ^D to close the tab`, wired up to close the tab.
  ([backsystacean-1](./backsystacean-1.md) REVIEW; UI verification in
  [frontend-2](./frontend-2.md))
- [~] Tab rename propagates into the PTY environment. Alex picked
  option (a) on 2026-05-18: spawn-time-only contract. On rename
  commit with an active PTY session, prompt the user inline to
  restart (`Restart` / `Later`). Restart calls
  `explicitCloseSession + teardown + start` (see
  `web/src/components/TerminalTab.svelte:383`), spawning a fresh
  PTY with refreshed env. If the user picks "Later", a small
  stale-env badge persists near the title until the user
  restarts. Frontend work: [frontend-2](./frontend-2.md). Doc
  note: [backsystacean-6](./backsystacean-6.md).
- [x] Shift+Enter inside the embedded terminal reaches programs like
  claude / codex instead of falling back to Enter.
  ([backsystacean-1](./backsystacean-1.md) REVIEW; live verification
  against a real CLI owed in [webtest-1](./webtest-1.md) or
  [webtest-2](./webtest-2.md).)
- [x] Modifier-Enter chord gap follow-up: Cmd+Enter (macOS) /
  Ctrl+Enter (Linux/Windows) now send CSI-u bytes instead of
  falling through to plain Enter; needed for claude / codex submit
  gestures. Alt+Enter / Shift+Tab left to xterm defaults.
  ([frontend-13](./frontend-13.md) REVIEW)
- [~] Rich-prompt overlay per terminal: markdown composer with
  toolbar + source/render toggle, triggered by Alt+Space and
  right-click "Rich prompt", mid-pane initial height, height-
  only resize, Esc hides + keeps buffer, Cmd+Enter ships raw
  markdown to PTY, image paste through `/api/attachments`,
  right-click "New File from here" saves buffer as a file.
  Per-terminal state in the window session blob.
  ([frontend-14](./frontend-14.md) PARTIAL: functional first cut;
  live/editor hardening and component tests owed.)
- [ ] File browser opens collapsed on first open (no auto-expand of
  every directory).
  ([frontend-2](./frontend-2.md))
- [ ] Terminal right-click gains a "New File" action that opens the
  new-file dialog seeded with the terminal's current working
  directory.
  ([frontend-2](./frontend-2.md))
- [x] File browser overlay header shows the full drive-relative
  path of the selected entry (ellipsis on overflow, full path on
  hover).
  ([frontend-10](./frontend-10.md) REVIEW)
- [x] "Terminal from here" context-menu action on file browser
  rows: directories open a new terminal with CWD = that dir;
  files open with CWD = parent dir **and** seed the prompt as
  `$ <cursor> path` (leading space + path, cursor at start via
  Ctrl+A) so the user can type a command in front of it.
  ([frontend-10](./frontend-10.md) REVIEW; editor tab menu parity
  included.)
- [~] Graph overlay scope navigation polish: breadcrumb
  (`drive / notes / sub`, each clickable to re-scope) for going
  back up + "Graph from here" button on directory nodes in the
  graph inspector for going forward (currently gated to file
  nodes only, `GraphPanel.svelte:1139`). Forward depth keeps
  the current semantics.
  ([frontend-12](./frontend-12.md) REVIEW for directory-node
  "Graph from here"; breadcrumb parked to phase 6.1.)
- [ ] Terminal tab title carries a visible indicator when the tab is
  in broadcast mode (next to the title).
  ([frontend-2](./frontend-2.md))
- [ ] Broadcast-mode in-body status bar reuses the slot the old
  "connected - WxH" strip occupied:
  `[broadcast-icon] [member1 x] [member2 x] ... [off]`. The
  `[broadcast-icon]` acts as a mute toggle (stay in the group
  but pause both in and out flow); icon state reflects
  active/muted. Each member chip carries an `[x]` to remove
  that member, peer-to-peer from any tab in the group. `[off]`
  leaves the group for this tab. Bar dissolves when the group
  has <= 1 member. Requires lifting the broadcast model from
  source/target asymmetric to a symmetric group (frontend picks
  shape).
  ([frontend-2](./frontend-2.md))
- [ ] Broadcast target picker grows a `Select All` / `Deselect All`
  button. `Select All` includes the current terminal tab (the
  source) too.
  ([frontend-2](./frontend-2.md))
- [ ] Terminal bubble menu order: keep MCP items at the top, move
  Broadcast toggle DOWN next to the broadcast target selectors.
  ([frontend-2](./frontend-2.md))
- [~] Markdown WYSIWYG editor sometimes shows a trailing buffer
  below the document end; source-code view is correct.
  ([frontend-7](./frontend-7.md) REVIEW: WYSIWYG surface is
  CodeMirror 6 not Tiptap/ProseMirror; could not reproduce on
  code inspection. Landed a defensive `{#key tab.id}` so editor
  state tears down on file-tab switch. Live verification owed via
  long-doc / short-doc tab-switch sequence; if it returns,
  reopen.)
- [x] File browser overlay dismisses immediately on file click;
  destination tab focuses and shows LOADING placeholder until the
  fetch resolves. Same pattern for graph "Open in this pane".
  ([frontend-8](./frontend-8.md) REVIEW: FileTree double-click no
  longer awaits `api.read` before dismiss; tab state already
  created/focused the loading tab synchronously; FileEditorTab
  surfaces load failures in-tab; graph "Open in this pane"
  already dispatched without awaiting.)

### Closing
- [ ] Pre-push gate green on the final HEAD.
- [ ] End-to-end hardening across all of the above.
- [ ] Summary with outcomes, highlights, lowlights, bugs, coverage,
  follow-ups, and agent rankings.

Statuses on the dispatch table: TODO, IN_PROGRESS, BLOCKED, REVIEW, DONE.

## Capacity proposal

Alex named the team in [request.md](./request.md): @@Architect,
@@WebtestA, @@WebtestB, @@Frontend, @@Backsystacean. Five slots, with
@@Backsystacean carrying the combined @@Backend + @@Syseng +
@@Rustacean profile because phase-6 work spans HTTP boundary, chan-
drive filesystem semantics, and Rust quality together. The combined
profile reduces the per-task review fan-out compared to phase 5.

| Slot          | Profile        | Role for phase 6                                                                                 |
|---------------|----------------|--------------------------------------------------------------------------------------------------|
| @@Architect   | Architect      | Coordination, design memo for graph layering + color + permission, summary, commit groupings.    |
| @@Frontend    | Frontend       | UX bundle, right-click menus, tab disambiguation, graph UI flip, terminology codemod web side.   |
| @@Backsystacean | Backend+Syseng+Rustacean | chan-drive file classifier, frontmatter kinds, chan-report aggregation, terminology codemod, PTY hooks. |
| @@WebtestA    | Webtest        | Live test service owner, end-to-end smoke, regression hunt across rebuilds.                      |
| @@WebtestB    | Webtest        | Parallel scenarios, focused probes (graph default scope, classifier rendering, terminal menus).  |

Profile gaps that the phase will carry:

* @@Frontend is the only frontend slot. If both the graph UI flip and
  the right-click menu pass land in the same session window, expect
  serialized review.
* @@Backsystacean shoulders three review surfaces (HTTP, filesystem,
  Rust quality). Self-review across those is acceptable, but
  @@Architect should flag any task where the surfaces split (for
  example: route + chan-drive + worker thread changes inside the same
  task) and ask for an explicit pause / re-read before commit.

Expected later switches:

* Once the design memo lands, @@Architect picks up commit-grouping
  + summary work and shifts to coordination-only.
* @@WebtestB may run extra parallel scenarios as the bug-fix lanes
  rebuild faster than the architectural lanes.

## Dispatch

Wave 1 fans out the design memo and the parallel implementation tracks
listed above. By the time @@Architect finished orientation, @@Frontend,
@@Backsystacean, and @@WebtestA had already opened their own task files
and made progress, so the dispatch table is reconciled below rather
than written from a clean baseline. Wave 2 will pick up review residue,
hardening, and the commit-coordination + push.

| Task | Owner | Status | Notes |
|------|-------|--------|-------|
| [architect-1](./architect-1.md) | @@Architect | IN_PROGRESS | Coordination, journal upkeep, dispatch + commit groupings + summary at phase close. |
| [architect-2](./architect-2.md) | @@Architect | DONE | Design memo: contracts handed to all wave-1 tracks. All open questions answered by Alex 2026-05-18. |
| [architect-3](./architect-3.md) | @@Architect | DRAFT | Commit groupings for phase close: six-commit shape (chan-drive, chan-report, chan-server, web, docs, release) mirroring phase 5. |
| [frontend-1](./frontend-1.md) | @@Frontend | REVIEW | Self-dispatched. New-file dialog quick-start, editor New File parent dir, file-tree + editor Copy Path, overlay backdrop context menus routed through panel menu handler, terminal theme refresh. `npm run check` + 18-file / 160-test suite green. |
| [backsystacean-1](./backsystacean-1.md) | @@Backsystacean | REVIEW | Self-dispatched. Terminal-N enumerated names, Shift+Enter enhanced-keyboard bytes, Ctrl+D close handling after shell exit, `CHAN_TAB_NAME` at PTY spawn. Tab-rename-to-env propagation flagged as needing a product decision; handed to [backsystacean-6](./backsystacean-6.md). |
| [webtest-1](./webtest-1.md) | @@WebtestA | IN_PROGRESS | Live test service up on `/private/tmp/chan-test-phase6` (PID 56504, port 8787). Baseline smoke (health + editor 200) green. Smoke checklist ready to run as lanes land. |
| [frontend-2](./frontend-2.md) | @@Frontend | REVIEW | PANE Inspector toggle + outside-overlay menu landed; terminal menu expanded with CWD rows and fallback status, New File, copy/paste, find/copy scrollback/restart/new terminal/split/search/settings; bubble menu reordered; broadcast group strip + peer removal + mute + Select All picker; tab-rename stale-env prompt; file browser opens collapsed. Live CWD execution remains backend/session-metadata dependent. |
| [frontend-3](./frontend-3.md) | @@Frontend | REVIEW | Same-name tab disambiguation landed: shortest divergent directory segment in the tab title, full path on hover, Webtest round 4 PASS. |
| [frontend-4](./frontend-4.md) | @@Frontend | REVIEW | Royal-pink tokens, "Graph from here" rename across all surfaces, `/api/inspector` consumption (file browser / graph / search), classifier badges (read-only / symlink / special / hardlink / outside-drive), directory subtree counts with tree-derived fallback, chan-report COCOMO roll-up preserved. |
| [frontend-5](./frontend-5.md) | @@Frontend | PARTIAL | User-visible "directory" copy landed across browser, tree, inspector, prompt, dashboard, import surfaces. Remaining: broad identifier cleanup (`kind: "folder"`, graph filters, persisted scope keys; compatibility pass needed because some are wire-format adjacent). |
| [backsystacean-2](./backsystacean-2.md) | @@Backsystacean | REVIEW | chan-drive `PathClass` API, `/api/files` `path_class` payload, fs-graph classifier metadata, read-only directory dead-end behavior. Also fixed inherited `CHAN_MCP_*` leak in PTY spawn found by full server tests. `cargo test -p chan-drive -- --test-threads=1`, `cargo test -p chan-server`, fmt, clippy, no-default-features build green. |
| [backsystacean-3](./backsystacean-3.md) | @@Backsystacean | REVIEW | Added `GET /api/inspector?path=...`, byte-based chan-report rollups, subtree file/dir/byte/file-kind counts, markdown/text report rows, and minimal binary/special payloads. `cargo test -p chan-report`, `cargo test -p chan-server`, `cargo test -p chan-drive report`, `npm run check`, fmt, clippy, no-default-features build green. |
| [backsystacean-4](./backsystacean-4.md) | @@Backsystacean | REVIEW | Frontmatter `chan.kind` registry landed with `contact` as the only entry; unknown kinds stay markdown files. Tag + mention extraction is pinned markdown-only by chan-drive test and documented. `cargo test -p chan-drive -- --test-threads=1`, `cargo test -p chan-server`, and `scripts/pre-push` green. |
| [backsystacean-5](./backsystacean-5.md) | @@Backsystacean | REVIEW | Crates-side terminology codemod landed: Rust-owned identifiers, CLI help/errors, fs-graph scope/node vocabulary, language graph directory nodes, and docs now use directory/dir. Remaining `folder` hit is rust-embed's required `#[folder = ...]` macro attribute. `cargo build --workspace`, `cargo test --workspace`, clippy, fmt, and no-default-features build green. Frontend wire-shape follow-up remains in [frontend-5](./frontend-5.md). |
| [backsystacean-6](./backsystacean-6.md) | @@Backsystacean | REVIEW | Memo drafted. Recommendation: do not inject `export` into PTY stdin; keep spawn-time `CHAN_TAB_NAME` and use explicit opt-in shell integration if Alex wants runtime shell env refresh. Implementation remains gated on product decision. |
| [webtest-2](./webtest-2.md) | @@WebtestB | IN_PROGRESS | Parallel scenarios across architectural cleanups + bug fixes; coordinates rebuilds with [webtest-1](./webtest-1.md). |
| [backsystacean-7](./backsystacean-7.md) | @@Backsystacean | REVIEW | `/api/health` now includes live indexer state: `idle` / `settling` / `rebuilding` / `error`, queue depth, last watcher event, last settled time, and coalesced rebuild flag. `cargo test -p chan-server health`, `cargo test -p chan-server`, and `scripts/pre-push` green. Unblocks [frontend-6](./frontend-6.md). |
| [frontend-6](./frontend-6.md) | @@Frontend | REVIEW | Typed `/api/health` client; ghost inspector polls 1/s while open; busy state renders `indexer is catching up (N event(s) pending)` / `indexer is rebuilding (full pass)`; idle or poll failure falls back to the static hint. |
| [frontend-7](./frontend-7.md) | @@Frontend | REVIEW | Markdown WYSIWYG trailing-buffer investigation landed a defensive editor-body key; Webtest round 4 tab-switch probe PASS, original glitch still unreproduced. |
| [frontend-8](./frontend-8.md) | @@Frontend | REVIEW | File-browser overlay dismiss + focused LOADING tab behavior landed, including in-tab error body; Webtest round 4 delayed-read probe PASS. |
| [backsystacean-8](./backsystacean-8.md) | @@Backsystacean | REVIEW | Webtest A+B fix bundle landed: inspector hardlink rollups dedupe by inode (`HashSet<(dev, ino)>` via `symlink_metadata`; presentation-layer only so watcher/indexer/search/graph stay path-based), inspector emits `frontmatter_kind` (`contact` or `null`), nested `chan:` frontmatter shape documented, `/api/files?dir=...` includes symlinks with `path_class`, and fs-graph nodes carry `path_class` so FIFO/socket/symlink kinds are distinguishable. Contract review PASS by @@Architect. `cargo test -p chan-server`, `cargo test -p chan-drive list`, `cargo fmt --check`, `scripts/pre-push` green. |
| [backsystacean-9](./backsystacean-9.md) | @@Backsystacean | REVIEW | `/api/graph` now merges semantic + fs-graph + language-graph producers behind optional `scope=drive|directory|file`, `path`, `depth` params. Adds directory/file/media/language nodes, `contains` + `language` edges, `path_class` on filesystem nodes. Overlay drive/global mode consumes the merged endpoint; standalone `/api/fs-graph` + `/api/graph/languages` retained. Contract review PASS by @@Architect: helpers `merge_filesystem_layer` + `merge_language_layer` with tests `merged_graph_layers_emit_filesystem_media_and_language_nodes` and `merged_graph_keeps_read_only_directories_as_dead_ends`. Live verification owed on the test service. |
| [webtestb-idle](./webtestb-idle.md) | @@WebtestB | INFO | Idle after rounds 1+2 in [webtest-2](./webtest-2.md). Next pickups blocked on frontend lanes 2-5 + restart for backsystacean-7. |
| [frontend-10](./frontend-10.md) | @@Frontend | REVIEW | File-browser header shows selected full drive-relative path with hover title; "Terminal from here" in file browser and editor menus opens fresh PTYs with sandboxed `cwd` and file prompt seed. `cargo test -p chan-server terminal`, `npm --prefix web run check`, `npm --prefix web test -- --run`, and `npm --prefix web run build` green. |
| [frontend-11](./frontend-11.md) | @@Frontend | PARKED | OBS-WT6-WTA-9 chip counter mismatch. Cosmetic; underlying data correct. Re-files in follow-up phase per [architect-4](./architect-4.md). |
| [frontend-12](./frontend-12.md) | @@Frontend | REVIEW/SPLIT | Dir "Graph from here" half landed: filesystem graph inspector can pivot directory nodes to `dir:<path>` and file nodes to `file:<path>` via `scopeFsGraphFromHere`, depth reset to 1. Breadcrumb half PARKS to follow-up. `npm --prefix web run check`, `npm --prefix web test -- --run`, and `npm --prefix web run build` green. |
| [architect-4](./architect-4.md) | @@Architect | IN_PROGRESS | Phase-6 wrap plan locked by Alex 2026-05-18: drive frontend-2 + -10 + -12 dir-half + OBS-WT6-L restart, park frontend-5 broad codemod + -11 + -12 breadcrumb to phase 6.1. Six-commit shape stays. |
| [frontend-13](./frontend-13.md) | @@Frontend | REVIEW | Modifier-Enter chord gap closed: Ctrl+Enter emits `\x1b[13;5u`, Cmd/Meta+Enter emits `\x1b[13;9u`, Shift+Enter unchanged. Alt+Enter/Shift+Tab left to xterm defaults. `npm --prefix web run check`, `npm --prefix web test -- --run`, and `npm --prefix web run build` green. |
| [frontend-14](./frontend-14.md) | @@Frontend | PARTIAL | Functional rich-prompt first cut: per-terminal overlay, Alt+Space/right-click trigger, Wysiwyg/Source composer with StyleToolbar mode toggle, height-only resize, Esc hide, Cmd/Ctrl+Enter raw PTY send, New File from here, and per-window session persistence. Component tests now cover close/submit/resize/mode/isolation/New File behavior; live image-paste/CLI verification still owed. |
| [backsystacean-10](./backsystacean-10.md) | @@Backsystacean | REVIEW | Terminal sessions now expose live PTY CWD metadata: Linux via `/proc/<pid>/cwd`, macOS via safe on-demand `lsof` because `chan-server` forbids unsafe FFI, other platforms return unavailable. WebSocket supports initial `ready.cwd` plus `cwd` request/response frames; frontend CWD menu rows now copy/reveal/graph/seed New File from the drive-relative CWD with fallback outside the drive. `cargo test -p chan-server terminal -- --test-threads=1`, `cargo test -p chan-server -- --test-threads=1`, `npm run check` from `web/`, and `scripts/pre-push` green. |
| [frontend-15](./frontend-15.md) | @@Frontend | REVIEW | Window-scoped broadcast invariant pinned: `broadcastTerminalInput` resolves targets through current-window `allTerminalTabs()` only, doc-comment warns against sink-id/server-bus fan-out, and a cross-window sink-id regression test confirms no delivery outside the current layout. `npm --prefix web run check`, `npm --prefix web test -- --run`, and `npm --prefix web run build` green. |
| [frontend-idle](./frontend-idle.md) | @@Frontend | INFO | Idle marker after [frontend-1](./frontend-1.md). Next sequenced pull: [frontend-4](./frontend-4.md). |

## Decisions confirmed by Alex (2026-05-18)

* Royal-pink LGTM: `#C71585` light / `#FF4DB8` dark. Token
  `--chan-color-language` (alias `--chan-color-code`).
* "Graph from here" replaces "Graph this" across all surfaces.
  Default scope is drive.
* Frontmatter kinds beyond `contact` are next phase, not this one.
  [backsystacean-4](./backsystacean-4.md) ships the registry scaffold
  with `contact` only.
* Ghost-node indexer-progress UX gap is in scope this phase. See
  [backsystacean-7](./backsystacean-7.md) +
  [frontend-6](./frontend-6.md).
* Lane sequencing: @@Architect picks, coordinated with @@Webtest A/B
  capacity. Recorded below.

## Original decisions (best reads, since confirmed above)

1. **Default graph scope = drive across all entry points.** Today
   "Graph this" from the file browser scopes to drive; the default
   from the empty-pane menu, the graph overlay opener, and the
   editor's "Graph file" still resolve to the file or directory.
   Phase 6 flips them all to drive by default; per-file / per-dir
   actions become "Graph from here" rather than the implicit default.
2. **Royal-pink token name `--chan-color-language` (or close).**
   Slot lives alongside the existing tag green; existing pink
   variants stay where they are (contacts pill). Final hex value
   picked inside [architect-2](./architect-2.md).
3. **Terminology rule: replace `folder` with `directory`; in tight
   spaces use `dir`.** No softening, full codemod. Doc comments + UI
   copy + identifiers in scope; persisted state keys keep their
   current names to avoid forcing a migration in this phase
   (recorded as a follow-up).
4. **Frontmatter kind contract: keep `chan.kind: contact` exactly as
   today; new kinds use the same `chan.kind: <name>` prefix.** The
   contact pill becomes the reference renderer; the ladder is
   open-ended in code, not exhaustive.
5. **Tag and mention scope stays markdown-only.** Plain-text and
   binary files do not contribute `#tag` or `@@mention` edges to the
   graph. Document the rule in chan-drive's design notes.
6. **Tab-rename to env propagation: investigate, do not commit.**
   POSIX shells cannot have env mutated after spawn; the practical
   options are OSC title escapes (does not actually change `$CHAN_*`
   inside the shell) or a chan-side env sync that the user opts
   into. @@Backsystacean writes a short memo inside
   [backsystacean-5](./backsystacean-5.md); Alex picks the path
   before implementation.

## Extended requests (mid-phase additions)

Tracked here for traceability. Each item came in after the
phase began; cross-linked to the task file that absorbed it
and the journal checklist entry where applicable.

| Date       | Source                                                                                  | Item                                                                                                                                          | Landed in                                                  |
|------------|-----------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------|
| 2026-05-18 | Alex chat                                                                               | File browser opens collapsed on first open                                                                                                    | [frontend-2](./frontend-2.md)                              |
| 2026-05-18 | Alex chat                                                                               | Terminal right-click "New File" seeded with terminal CWD                                                                                      | [frontend-2](./frontend-2.md)                              |
| 2026-05-18 | Alex chat                                                                               | Broadcast-mode indicator next to terminal tab title                                                                                           | [frontend-2](./frontend-2.md)                              |
| 2026-05-18 | Alex chat                                                                               | Broadcast picker Select/Deselect All (including the source tab)                                                                               | [frontend-2](./frontend-2.md)                              |
| 2026-05-18 | Alex chat                                                                               | Broadcast bar reuses the freed-up status-bar slot: `[icon] [members] [off]`                                                                   | [frontend-2](./frontend-2.md)                              |
| 2026-05-18 | Alex chat                                                                               | Member chips get `[x]`; peer-removable from any tab in the group                                                                              | [frontend-2](./frontend-2.md)                              |
| 2026-05-18 | Alex chat                                                                               | Broadcast icon acts as mute toggle (stay in group, pause in/out flow)                                                                         | [frontend-2](./frontend-2.md)                              |
| 2026-05-18 | Alex chat                                                                               | Bubble menu reorder: MCP items up, Broadcast toggle down next to selectors                                                                    | [frontend-2](./frontend-2.md)                              |
| 2026-05-18 | Alex chat + screenshot                                                                  | Markdown WYSIWYG trailing-buffer investigation                                                                                                | [frontend-7](./frontend-7.md) (defensive `{#key tab.id}`)  |
| 2026-05-18 | Alex chat                                                                               | File-browser dismiss immediately on click; tab focuses with LOADING placeholder                                                               | [frontend-8](./frontend-8.md)                              |
| 2026-05-18 | Alex chat + [backsystacean-1](./backsystacean-1.md) flag                                | Tab-rename to env decision (option a + restart prompt)                                                                                        | [backsystacean-6](./backsystacean-6.md) memo + [frontend-2](./frontend-2.md) UI |
| 2026-05-18 | Alex chat (ghost-node screenshot)                                                       | Ghost-node indexer-progress live status surface                                                                                               | [backsystacean-7](./backsystacean-7.md) + [frontend-6](./frontend-6.md) |
| 2026-05-18 | Alex chat (graph chip screenshot)                                                       | Fold fs-graph + language-graph into `/api/graph`                                                                                              | [backsystacean-9](./backsystacean-9.md)                    |
| 2026-05-18 | Alex chat                                                                               | File browser overlay header = full path of the selected entry                                                                                 | [frontend-10](./frontend-10.md)                            |
| 2026-05-18 | Alex chat                                                                               | "Terminal from here" on dirs (CWD = dir) and files (CWD = parent + prompt seed `$ <cursor> path` via leading-space + Ctrl+A)                  | [frontend-10](./frontend-10.md)                            |
| 2026-05-18 | Alex chat                                                                               | Graph overlay scope breadcrumb (parks to 6.1)                                                                                                 | [frontend-12](./frontend-12.md) breadcrumb half            |
| 2026-05-18 | Alex chat                                                                               | "Graph from here" on directory nodes in graph inspector                                                                                       | [frontend-12](./frontend-12.md) dir half                   |
| 2026-05-18 | [webtest-2](./webtest-2.md) OBS-WT6-I / J / K + [webtest-1](./webtest-1.md) WTA-1 / 5  | Inspector hardlink dedupe, `frontmatter_kind` payload, canonical nested frontmatter shape doc, symlinks in `Drive::list`, fs-graph special-file `path_class` | [backsystacean-8](./backsystacean-8.md) (5-item fix bundle) |
| 2026-05-18 | [webtest-1](./webtest-1.md) OBS-WT6-WTA-9                                              | Graph filter chip counter overcount (parks to 6.1)                                                                                            | [frontend-11](./frontend-11.md) (parked)                   |
| 2026-05-18 | Alex chat                                                                               | Modifier-Enter chord gap: Cmd+Enter / Ctrl+Enter (alongside the original Shift+Enter ask)                                                     | [frontend-13](./frontend-13.md)                            |
| 2026-05-18 | Alex chat                                                                               | Rich-prompt overlay per terminal (markdown composer + Alt+Space / right-click + Cmd+Enter submit + image paste + "New File from here")        | [frontend-14](./frontend-14.md)                            |

## Notes / decisions log

* 2026-05-18 @@Architect: journal opened; capacity proposal recorded
  based on the team Alex named in request.md. Tasks fanned out for
  wave 1.
* 2026-05-18 @@Architect: wave-1 backsystacean lanes 1, 2, 3, 4 now
  REVIEW. Architectural cleanups for file classifier, inspector
  payload, frontmatter kind ladder, and tag/mention scope are
  backend-complete; frontend wiring in [frontend-4](./frontend-4.md)
  is the consumer. Frontend lanes 2-5 still TODO; @@Frontend idle
  since [frontend-1](./frontend-1.md) REVIEW.
* 2026-05-18 Alex: spotted a ghost-node UX gap in the graph
  ("not in the current file listing"). Confirmed in scope this
  phase; tracked as [backsystacean-7](./backsystacean-7.md) +
  [frontend-6](./frontend-6.md).
* 2026-05-18 Alex confirmed: royal-pink LGTM, "Graph from here"
  across all surfaces, frontmatter kinds defer to next phase,
  ghost-node UX in this phase, lane sequencing on @@Architect with
  test capacity in mind.
* 2026-05-18 @@Architect lane sequencing:
  * @@Frontend next pull: [frontend-4](./frontend-4.md). Biggest
    single architectural lane; gates the close-out; consumes the
    REVIEW backend payloads from [backsystacean-2](./backsystacean-2.md),
    [backsystacean-3](./backsystacean-3.md), and
    [backsystacean-4](./backsystacean-4.md). While @@Frontend works
    on it, @@WebtestA/B churn through the REVIEW pile in the
    browser.
  * @@Backsystacean next pulls in order:
    [backsystacean-7](./backsystacean-7.md) (small, parallel,
    unblocks [frontend-6](./frontend-6.md)), then
    [backsystacean-6](./backsystacean-6.md) (memo only), then
    [backsystacean-5](./backsystacean-5.md) (codemod, run after
    @@Frontend completes major touches to avoid rebase churn with
    [frontend-5](./frontend-5.md)).
  * @@Frontend after [frontend-4](./frontend-4.md):
    [frontend-2](./frontend-2.md) (right-click + broadcast +
    collapsed file browser), then [frontend-3](./frontend-3.md)
    (tab disambiguation), then [frontend-6](./frontend-6.md)
    (ghost-node UX), then [frontend-5](./frontend-5.md) (codemod,
    paired with backsystacean-5).
  * @@WebtestA holds the live service; rebuilds happen at
    @@Frontend pull boundaries to batch verification. @@WebtestB
    runs parallel probes per [webtest-2](./webtest-2.md).

## Progress

(updated as wave 1 lands)

## Completion notes

(populated at phase close)
