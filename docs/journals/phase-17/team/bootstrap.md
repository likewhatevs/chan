# phase-17-team - team bootstrap + round-1 plan

Hand-authored. Launch with `cs terminal team load docs/journals/phase-17/team`
(use `load`, NOT `new`: `new` regenerates this file from config.toml and would
wipe the plan below). On load, each agent is poked to read this file.

## Who we are

- Host: @@Alex (Alex). Sets scope; the only one who acts outside the team;
  reach the host through @@LaneA.
- Lead: @@LaneA. Distributes tasks, sequences the work, owns the full-tree
  gate + commits, aggregates requests for the host.

## Roster

+---------+--------+--------+----------------------------------------+
| handle  | agent  | role   | round-1 lane                           |
+---------+--------+--------+----------------------------------------+
| @@LaneA | claude | lead   | launcher follow-ups + dialogs + coord  |
| @@LaneB | claude | worker | terminal & cs core                     |
| @@LaneC | claude | worker | editor & graph                         |
| @@LaneD | claude | worker | platform (server/CLI/workspace) + docs |
+---------+--------+--------+----------------------------------------+

## How we work

- Workers hold and wait for @@LaneA to distribute tasks. Do not start until
  poked with your task path.
- @@LaneA cuts a task into tasks/task-{from}-{to}-{n}.md (recipient-owned,
  N an atomic increment, append-only) and pokes the recipient.
- On completion cut a task back to @@LaneA in the same place + format, poke.
- Keep an append-only log in journals/journal-{your-name}.md.
- Worker-to-host routes through @@LaneA.

## The poke 1-liner

    cs terminal write --tab-name=<target> --submit=<target-agent> \
        $'poke from <me>: <1-line>; read <path>'

`--submit=claude` appends the submit chord so the poke fires instead of
parking in the compose box. Pokes are 1-line pointers; context lives in the
task file you point to.

## Files

- config.toml   the team config (hand-editable; revalidated on load)
- bootstrap.md  this file (team process + the round-1 plan)
- tasks/        task-{from}-{to}-{n}.md, recipient-owned, append-only
- journals/     journal-{member}.md, owned by each member, append-only
- followups/    followup-{from}-{to}-{n}.md, recipient-owned

================================================================================
# ROUND-1 PLAN
================================================================================

## Source of truth

- Spec: docs/journals/phase-17/round-1/draft.md (+ image*.png). @@Alex's own
  write-up of the bugs, enhancements, docs, and website screenshot plan.
- Launcher smoke follow-ups: docs/journals/round-16/smoke-checklist-LaneD.md.
  @@Alex's inline replies on sections 1 (header), 4 (outbound), 5 (inbound)
  are the S1/S2/S3 items below.
- Predecessor round (the launcher redesign, committed fd27d29d): archived at
  docs/journals/round-16/. Read its README + design doc before touching
  desktop/src.

The file:line anchors below come from a recon pass; treat them as starting
points, re-verify against HEAD before editing (line numbers drift).

## Goal

Land @@Alex's round-1 bug fixes + enhancement + the 3 launcher smoke
follow-ups, then the documentation/website reframe. Release-quality bar: full
gate green, no known bug shipped, empirical smoke of every behavioral change.

## Lane assignment + owned files

Lanes are drawn so each owns a coherent file set. Three files are touched by
two lanes; the coordination rules are in "Shared-file contention" below. Edit
ONLY files your lane owns; if a fix pulls you into another lane's file, STOP
and route through @@LaneA.

----------------------------------------------------------------------
@@LaneA (lead) - launcher follow-ups + dialogs + coordination
----------------------------------------------------------------------
Owns: desktop/src/{index.html,main.js,styles.css},
      web/src/components/TeamDialog.svelte.

- S1 Header order. desktop/src/index.html (~12-32): swap so the theme
  (sun/moon) toggle comes BEFORE [New]: [theme-icon][New]. WKWebView smoke.
- S2 Remote-outbound example. desktop/src/main.js renderOutbound (~641-667):
  after the intro copy add a proper CODE BLOCK example:
      chan serve ./path/to/repo
  then paste the URL; or remotely over ssh:
      ssh user@host -L 8787:localhost:8787 chan serve ./path/to/repo
- S3 Remote-inbound copy. desktop/src/main.js renderInboundForm (~689-726):
  replace the "Bind a loopback port ... (we listen)" line (~711) with:
      Listen for incoming connections on a configurable port, or use 0 to
      let the OS pick one. Then connect to it:
  + a code block:
      chan serve ./path/to/repo --tunnel-url={chan-desktop-listener}
- B3 Team-load path UX. TeamDialog.svelte refreshDirSuggestions (~129-142):
  today suggestions only populate after the user types a `/` (parent is the
  substring before the last `/`; bare `foo` lists root and matches nothing).
  Make a bare prefix suggest matching dirs (e.g. `foo` -> `foo/`) so loading
  an existing team does not require typing `/` first.
- E1 Spawn auto-assign. TeamDialog.svelte (~519-598): after the layout
  shape picker (2x2 / 1x4 / 4x1 ...), add an auto-assign button on the same
  row, right side, with a robot icon; it distributes unassigned members
  across the selected layout's empty cells (reuse config.realEstate.slots +
  cellOfMember). See draft.md image-2/image-3. Sequence AFTER B3 (same file).
Lead duties: cut tasks + sequence the waves, own the full-tree `make
pre-push` via an isolated gate.sh worktree, commit at round close, no push
without @@Alex.

----------------------------------------------------------------------
@@LaneB - terminal & cs core
----------------------------------------------------------------------
Owns: web/src/App.svelte, web/src/components/{RichPrompt.svelte,
      TerminalTab.svelte,BubbleOverlay.svelte,Pane.svelte},
      web/src/state/richPrompt.svelte.ts, web/src/terminal/submitMode.ts,
      web/src/state/tabs.svelte.ts (pane-mode region only),
      crates/chan-shell/* (cli.rs, submit.rs),
      crates/chan-server/src/control_socket.rs (pane-exec region only).

- B1 Rich prompt per-terminal. Today cmd+shift+p toggles a WINDOW-global
  state (App.svelte ~659-665 -> richPrompt.svelte.ts ~18-24) so it affects
  all terminals and focus lands on the last terminal, not the focused pane.
  Make it per-terminal: scoped to the focused tab in the focused pane; do
  NOTHING if no terminal is selected; on show, put focus+cursor in the
  prompt. Allow resizing the prompt's TOP up to the top of the terminal
  (mirror the bottom margin; RichPrompt.svelte ~197-210). Survey bubbles must
  stack ABOVE the rich prompt (BubbleOverlay z:39000 vs RichPrompt z:20 -
  verify the focused-prompt case does not invert this).
- B4 cs pane split/close. (1) `cs pane split` direction must be RIGHT and
  BOTTOM to match the hybrid hamburger (Pane.svelte ~484-500); chan-shell
  SplitDirArg today is Left|Bottom (cli.rs ~188-202) - align to right/bottom.
  (2) One-shot `cs pane` commands must NOT enter hybrid-nav transaction mode
  (tabs.svelte.ts enterPaneMode vs enterPaneModeTransaction ~2353-2376) and
  must NOT steal focus from the sending terminal UNLESS the command's purpose
  is to change pane/tab focus (paneModeSplit hardcodes activePaneId to the new
  pane ~2618-2631). Fixes the "stuck in transaction mode" + "lost focus after
  split/close" report.
- B8 cs --submit codex. `cs terminal write --submit codex` writes the text
  then a newline but does not submit. The chord map has codex="\r"
  (chan-shell/submit.rs ~54-78; mirror web/src/terminal/submitMode.ts ~19-23).
  REPRODUCE empirically first (naked write + candidate chords against a live
  codex), find the chord that actually submits, fix both the Rust source +
  the TS mirror in lockstep. Related to B5 (codex startup); coordinate notes.

----------------------------------------------------------------------
@@LaneC - editor & graph
----------------------------------------------------------------------
Owns: web/src/editor/decorations/blocks.ts, web/src/editor/Wysiwyg.svelte,
      web/src/components/PathPromptModal.svelte,
      web/src/components/GraphPanel.svelte, web/src/state/store.svelte.ts,
      web/src/state/tabs.svelte.ts (saveDraft region only).

- B2 Unordered list glyphs. Refine bullet glyphs per nesting level to match
  Google Docs (draft.md image.png). blocks.ts ~437-519 (BULLET_* decorations,
  bulletMarkerDecoration), Wysiwyg.svelte CSS ~1021-1060 (.cm-md-ul-* glyph
  content/size).
- B6 Save-dialog autocomplete. Saving a draft that is a DIRECTORY (has
  images) opens PathPromptModal in folder mode WITHOUT path autocomplete
  (tabs.svelte.ts saveDraftTabToWorkspace ~2064-2126, folder branch ~2085-94
  omits what the file branch passes). PathPromptModal already computes
  directory suggestions (~200-251) - make folder-mode drafts get the same
  autocomplete. draft.md image-10.
- B9 Graph bugs (single lane; interconnected). GraphPanel.svelte +
  store.svelte.ts:
    (a) New graph (cmd+shift+m) opens in semantic mode; double-click on a
        directory is a no-op until "Graph from here" flips to filesystem mode
        (onGraphDoubleClick ~231-240). Make directory expand work from the
        fresh graph.
    (b) After expand/collapse, the depth slider stops working
        (seedExpandedToDepth ~352-376 seeds the set but does not re-run
        layout). Make the slider expand all directories FROM the currently
        selected node onward (root + max = whole workspace; a node 2 deep =
        only its subtree).
    (c) "Graph from here" on a directory drops the initial layers
        (graphFromHere ~390-414 forces filesystem-only). Keep ALL layers:
        the directory spine, files with edges to their directory, markdown
        link/backlink + hashtag + contact/mention edges, and language edges
        to files (scopedNodeIds ~861-999, RenderedEdgeKind ~444-449). Re-read
        draft.md graph bullets verbatim - @@Alex re-describes the expected
        layer model precisely.

----------------------------------------------------------------------
@@LaneD - platform (server / CLI / workspace) + docs/website
----------------------------------------------------------------------
Owns: crates/chan-server/src/{terminal_sessions.rs,lib.rs},
      crates/chan-server/src/control_socket.rs (spawn-options region only),
      crates/chan/src/main.rs (cmd_serve),
      crates/chan-workspace/src/{fs_ops.rs,indexer.rs} + wire kinds,
      web/src/state/{kinds.ts,fileTypes.ts} (B11 SPA "pending" kind),
      README.md, web-marketing/*.

- B5 MCP env off by default + never touch user config. (1) Confirm + keep the
  invariant that chan NEVER writes a user's MCP/agent config files (recon
  found no such writes today - keep it that way). (2) Default new terminals to
  MCP env vars DISABLED: terminal_sessions.rs CreateOptions.mcp_env (~84) is
  hardcoded `true` at ~20 spawn sites incl control_socket.rs team spawn (~702);
  flip the default to off (a server-config/preference toggle to opt back in).
  set_mcp_env ~1402-1425. `cs search` + friends must still work with MCP off.
  This is the likely root cause of "MCP server keeps failing when we start
  codex" (codex needs file config, not just env). Coordinate control_socket.rs
  region with @@LaneB (B4 uses the pane-exec region; you use spawn options).
- B10 chan serve progress. `chan serve .` on a huge tree (e.g. a shallow
  linux-kernel clone) is silent for a long time even with --verbose. Print
  concise progress (indexing phase / counts) to stderr BEFORE the ready URL.
  cmd_serve crates/chan/src/main.rs ~1093-1242; the silent window is
  build_app in chan-server/src/lib.rs ~302-537 (indexer spawn ~367, "chan is
  ready" ~537); indexing progress originates in chan-workspace (boot/indexer +
  the existing progress.rs event stream). Not excessive; phase + progress.
- B11 Editable-text by content, not just extension. The gate is extension-only
  (chan-workspace/src/fs_ops.rs classify ~342); .zshrc, *.service, and other
  extensionless/odd text files are refused by the editor + file browser. This
  is the phase-15-deferred "content magic detection" (see
  docs/journals/phase-15/round-4-wave-4.md "Round-5 (deferred)"). Approach is
  pre-decided: a hand-rolled "first N bytes valid UTF-8 + no NUL -> Text"
  sniff, NO new dependency (honors single-binary; do NOT add libmagic/infer).
  Touches fs_ops.rs ~313-394 (the editable/binary gate), the indexer async
  hook, the wire kind (files.rs/graph.rs), and a SPA "pending" kind
  (kinds.ts/fileTypes.ts). Honor the phase-15 architect call on .md-vs-text
  (md = document/graph; other text = editable+searchable, not a document
  node). Keep Rust FileClass + the TS mirror in lockstep.
- D1 README + website reframe. (1) README.md + web-marketing home should OPEN
  with a usage example (curl|bash install -> `chan serve ./repo` -> IDE in the
  browser), per draft.md "Documentation". (2) web-marketing must LINK the
  released packages (/dl/cli + /dl/desktop from release.yml's /dl metadata)
  and add a chan-desktop section (macOS .app / Linux AppImage; remote attach
  inbound/outbound; lima-vm + ssh-tunnel examples) and a Chan gateway section
  (gateway/ services: identity/profile, db + CLI admin, workspace-proxy for
  `chan serve --tunnel-url`, OAuth, self-deploy). Site source is
  web-marketing/ (NOT web/); cross-link gateway/README.md. (3) AUDIT + TEST
  every command before publishing (@@Alex's explicit requirement) - so D1
  finalizes AFTER B10 + the launcher follow-ups land and their commands are
  verified. Draft early, verify late.

## Shared-file contention (the only cross-lane coupling)

+----------------------------+----------------+-------------------------------+
| file                       | lanes          | rule                          |
+----------------------------+----------------+-------------------------------+
| web/src/state/tabs.svelte  | B (pane region | Different functions, far      |
|   .ts                       |  ~2353/2618),  | apart. Each edits ONLY its    |
|                            | C (saveDraft   | region. .ts interleave-safe   |
|                            |  ~2085)        | for compile; lead commits the |
|                            |                | merged file at round close.   |
| web/src/App.svelte         | B (B1 rich     | App.svelte is B's. B9's fix   |
|                            |  prompt        | should live in store.svelte   |
|                            |  ~659-665);    | .ts/GraphPanel, NOT the       |
|                            | C (B9 graph    | cmd+shift+m handler. If B9    |
|                            |  cmd+shift+m   | MUST touch ~654-658, C routes |
|                            |  ~654-658?)    | that one line through @@LaneA.|
| chan-server/control_socket | B (pane-exec   | Same crate (chan-server) as   |
|   .rs + the chan-server     |  ~102-109),    | D's terminal_sessions/lib.    |
|   crate                     | D (spawn opts  | Make any shared-signature     |
|                            |  ~685-706)     | change in one burst, re-run   |
|                            |                | cargo check -p chan-server    |
|                            |                | green before pausing (shared- |
|                            |                | tree compile window).         |
+----------------------------+----------------+-------------------------------+

Everything else is single-lane. submit.rs/submitMode.ts (B8) must stay
byte-for-byte in sync. FileClass (Rust) and kinds.ts/fileTypes.ts (B11) must
stay in lockstep.

## Sequencing

- Wave 1 (start immediately, fully isolated, no shared file): A: S1/S2/S3,
  B3. B: B8 (repro first). C: B2. D: B11, B10.
- Wave 2 (shared-file items; lead sequences tabs.svelte.ts and
  control_socket.rs edits): B: B1, B4. C: B6, B9. D: B5.
- Wave 3 (depends on the above stabilizing): A: E1 (after B3). D: D1 docs +
  COMMAND AUDIT (after B10 + launcher commands verified).

## Gate + quality bar

- Scoped own-gate before any "done" report:
  - Rust: cargo fmt --check + cargo clippy --all-targets -D warnings +
    cargo test (scoped -p <crate>; run the workspace tests your change
    can break). Re-check the gnu build assumption with --no-default-features
    if you touch feature gates.
  - Frontend: make web-check (vitest - catches stale ?raw source-pins),
    svelte-check, npm run build. Browser-smoke any Svelte-5 reactivity change
    (static gates miss runtime $state/$derived errors).
  - Desktop (S1/S2/S3): cd desktop && make build.
- Lead owns the full-tree `make pre-push` from an isolated gate.sh worktree
  (gates committed state, immune to peers' WIP). Lanes report scoped
  own-gate-green + a pathspec sha; never block on the main-tree gate.
- Empirical smoke (test server; ask @@Alex / @@LaneA which client):
  rich-prompt scoping+focus, pane split/close+focus, graph expand/slider/
  layers, codex startup with MCP off, `cs --submit codex`, opening a .zshrc /
  *.service file, the launcher S1-S3 (WKWebView hand-smoke by @@Alex - agents
  cannot drive WKWebView). rust-embed bakes the bundle at build time: rebuild
  (npm run build -> cargo build) before smoking a frontend change; grep the
  SERVED bundle, not source, when a flag "looks broken".
- Pre-release: no back-compat - drop legacy outright, no migration paths.
- Commit at round close (coordination tree as docs(phase-17)); the launcher
  follow-ups + bug fixes are product commits. NO push without @@Alex's
  explicit ask.

## Open question for @@Alex (surface via @@LaneA, do not block Wave 1)

- B5 default: ship MCP env OFF for ALL agents by default, or keep it ON for
  claude (which consumes the env cleanly) and OFF only for codex? @@Alex said
  "disabled by default"; confirm whether that is global or codex-specific.
