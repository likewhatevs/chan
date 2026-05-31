# Phase 15 round 2, part 2: more refinements

Between round 1 and round 2 we had:
- Left over work from round 1 lane A, which is now captured as round 1 part 1 after an audit (this doc is round 2 part 2)
- I've also landed a fix for the chan-desktop MCP server to manage stale sockets, and unified the code in chan-server so that both chan and chan-desktop use the same code instead of a stale old copy
- In the "Team Work" bootstrap, the agent that should be executed **on the same terminal** where the bootstrap is happening, is not executed at all.. all new terminals come up correctly, except for the very terminal running the bootstrap process, which should be the very first to set name and group and restart the shell.

> **Review pass (@@Architect, 2026-05-31).** @@Alex's original asks are kept
> verbatim below; inline `[Architect review]` notes add source grounding, fix
> two bug framings that pointed at the wrong fix, and flag one dropped round-1
> item. New items @@Alex asked to add (the Ctrl+R reload remap) are folded in
> place. All findings were verified against HEAD via a 3-sweep code audit.


## Bugs
- Whenever we open a draft, we keep seeing the status bar showing 'reindexing Drafts/untitled/draft.md' and it won't go away. First question: is it really taking long to reindex that draft file? Second question: can we audit the code to check that all notifications going to the status use our setter function with a timer to disappear? Here's the stuck notification: ![](./image.png#w=250)

  **[Architect review] This is the same root cause as the Cmd+R bug below, and
  the "setter with a timer" framing is the wrong fix for *this* notification.**
  The "reindexing {path}" text is NOT a toast; it is display-driven from polling
  `/api/index/status`, rendered in `AppStatusBar.svelte` while
  `indexStatus.state === "reindexing"`, and cleared when the poll sees `Idle`. A
  timer would just HIDE a genuinely-stuck reindex. Real direction: (a) answer Q1
  by checking the draft path actually reaches `set_idle` server-side
  (`indexer.rs` `apply_watch_change`: `Drafts/<sub>` -> `index_draft_file` ->
  `Indexed` -> `set_idle`); (b) make the Reindexing->Idle transition
  event-driven on the WS bus instead of poll-only. (The broader "do real toasts
  auto-dismiss?" audit is still worth doing, just as a separate, smaller item.)
- Hitting Cmd+R causes the chan-desktop window to hang, and closing/reopening does not help. The only way to recover is to turn the workspace off and on again (restarting the internal chan-server); I suspect this may have to do with rendering graphs upon reload, but am not sure.. try to smoke with a somawhat busy screen with 1 editor, 1 terminal, 1 graph, 1 dashboard of the search index, using a seeded drive containing a shallow clone of this very repo, then hit reload.. this is what I've got: ![](./image-1.png#w=250)  and it never recovered as I explained here, until off/on the workspace; i dont think this is specific to chan-desktop, you can smoke on the browser

  **[Architect review] Not graph rendering: the screenshot is the preflight gate
  stuck at "Build search index / working...".** `PreflightOverlay.svelte` stays
  `locked` until `IndexStatus == Idle` (gated in `routes/preflight.rs`), polled
  every 750ms. Same missing/stuck Idle transition as the reindex bug above:
  reload re-runs the preflight gate while the server (never torn down) is wedged
  in a non-Idle index state, so it never unlocks; "workspace off/on" recovers
  because restarting chan-server resets the indexer to Idle. Treat these two as
  ONE investigation: "indexing never reports complete". The graph-render
  suspicion can be dropped.
- Opening links from the terminal: I can see that on mouse hover the links (e.g. https://google.com) are highlighted on the terminal, but they are not click-able. I don't know if this is a bug only in chan-desktop, or also in the browser/web version, or whether we just never enabled these to be clickable. They should work just like links in the editor, open in a new window from the web version, and on the external browser from chan-desktop.

  **[Architect review] We DID enable them; it is a missing click handler, not a
  missing addon.** `WebLinksAddon` is already loaded
  (`TerminalTab.svelte:613`) but with the default constructor, so hover
  highlights yet the click does not route (especially under Tauri). Fix: pass a
  custom handler that reuses `openExternalUrl()`
  (`web/src/editor/external_links.ts:73`) which already does web
  `window.open(_blank)` vs Tauri external-browser, exactly the editor's link
  behavior @@Alex wants to match.
- **Terminal Ctrl+R steals bash reverse-search (reload remap).** Cmd+R reload
  must map to **Ctrl+Shift+R on Linux/Windows, and NEVER plain Ctrl+R**, so the
  shell's reverse-search keeps working in the Hybrid Terminal. macOS stays Cmd+R
  (no collision: shell reverse-search is Ctrl+R there too).
  - Root cause: `app.window.reload` is `Mod+R` with `escapeTerminal: true`
    (`web/src/state/shortcuts.ts:177-184`); `Mod` = Ctrl on Linux/Windows, so it
    binds Ctrl+R and the escape-terminal path swallows it over a focused
    terminal. The chan-desktop native bridge also grabs it
    (`KEY_BRIDGE_JS` `case 'KeyR': invokeIpc(e, 'reload_window')`,
    `desktop/src-tauri/src/serve.rs:613`).
  - Touch points: the reload descriptor in `shortcuts.ts`; the `onWindowKey`
    matcher in `App.svelte` (pinned by `cmdRWindowReload.test.ts:13,30`); the
    desktop bridge `case 'KeyR'` (+ tests `serve.rs:1014-1024`); regenerate the
    `chan serve --help` table (`node web/scripts/shortcuts-table.mjs` ->
    `main.rs` SERVE_LONG_ABOUT). Verify plain Ctrl+R falls through to the PTY on
    every platform afterward.
  - Model nuance: `shortcuts.ts` says "the keymap doesn't branch on OS, only the
    label does", but this needs per-OS chord divergence (mac Meta vs non-mac
    Ctrl+Shift). Precedent to mirror: the Cmd+Shift+I broadcast is already
    metaKey-gated (`shortcuts.ts:110`), so branch in the keymap rather than
    inventing a new per-OS descriptor field. Rust + frontend -> full repo
    pre-push gate applies.
- **Editor: bold / inline marks revert to raw `**` source after a tab switch
  until you click.** Switching away from an editor tab and back leaves part of
  the doc showing its markdown markers raw (e.g. `**[Architect review]**` shows
  the asterisks, inline-code shows backticks) while the block just above renders
  fine; clicking anywhere or moving the cursor re-renders all of them at once.
  (Reproduced in @@Alex's screenshot of this very doc: the lower `[Architect
  review]` block was raw, the one above it was bold.)

  **[Architect review] Root cause: the conceal decorations are computed only
  over the current `view.viewport`, and nothing recomputes them on a tab-switch
  re-show.** The markdown walker decorates only `view.viewport.from..to`
  (`web/src/editor/decorations/walker.ts:112`) and the ViewPlugin recomputes
  only on `update.docChanged || update.viewportChanged || update.selectionSet`
  (`walker.ts:87-91`). Editor tabs are unmounted/remounted on tab switch
  (`Pane.svelte` renders the single active tab via `{:else if active?.kind ===
  "file"}`; terminals are the exception, kept mounted with `visibility:hidden`).
  So on switch-back the EditorView is reconstructed and `computeDecorations`
  runs once in the constructor (`walker.ts:84`) against the INITIAL viewport,
  before the container geometry has settled, so only the top portion is
  concealed; the lower blocks fall outside that initial viewport and get no
  conceal decorations. The post-layout measure does not reliably fire a
  `viewportChanged` update, so the walker never re-decorates the rest until a
  caret move (`selectionSet`) or scroll (`viewportChanged`). That is exactly the
  "block above fine, block below raw, click fixes it" split.

  This is the SAME class as the terminal "renders garbled until you click/resize
  when a tab becomes active" bug, which already got a fix: TerminalTab reacts to
  its `active` flip and re-converges the renderer
  (`recoverTerminalRendererAfterHostResume()`, `TerminalTab.svelte:313`). The
  editor needs the analogous "re-measure + recompute on becoming visible" hook.
  Candidate fixes (decide empirically):
  - (a) Add `geometryChanged` (and/or `heightChanged`) to the walker recompute
    condition (`walker.ts:88`) so the post-show geometry-settle triggers a full
    re-decorate over the corrected viewport. Lowest-touch; the walk is
    viewport-bounded so cost stays cheap.
  - (b) Force a recompute when the editor tab becomes active/visible: an
    `active`-driven `$effect` in `FileEditorTab` (mirroring the terminal
    pattern) calling a Wysiwyg/Source method that `requestMeasure()`s AND forces
    the walker to recompute (a no-op dispatch / annotation; `requestMeasure()`
    alone does NOT trigger the walker). The existing `focus()` exports already
    pair `view.focus()` + `view.requestMeasure()` (`Wysiwyg.svelte:314`,
    `Source.svelte:145`) but are gated on pane focus, so they miss a pure
    active/visibility flip.
  - (c) Keep the editor mounted-but-hidden like terminals plus an active-flip
    remeasure. Bigger change.
  Frontend-only (Svelte 5 + CodeMirror); browser-smoke required (static gates
  miss CM measure/decoration timing).
- **Shift+Enter still submits to a running agent (round-1 BUG-3 carryover, NOT
  fully fixed).** In the terminal, with an agent (claude/codex) attached,
  Shift+Enter should insert a newline for multi-line composition but still sends
  a plain `\r` and submits. @@Host confirms it is still broken on the latest
  release (v0.20.0).

  **[Architect review] The round-1 fix was incomplete: it added reload
  persistence but no fallback for an agent that never announced its protocol.**
  `41b28e7a` made the keyboard-protocol state survive reload (serialize/restore
  + stop resetting on reconnect), but `terminalMetaKeyBytes()`
  (`web/src/terminal/keymap.ts:48-75`) returns `null` when the protocol state is
  unknown/zero, so xterm falls through to a plain `\r`. The reconnect path
  (`TerminalTab.svelte` ~596-604) does not re-query negotiation, and a
  long-lived agent does not re-emit it, so for the common case (agent already
  running, never observed negotiating) the state stays zero and Shift+Enter
  submits. Fix: land candidate (c) from the round-1 plan, a sane default
  sequence when Shift+Enter is held and protocol state is unknown (and/or
  re-query negotiation on reconnect). The `keymap.test.ts` suite only exercises
  serialize/restore, so it misses this; needs a **real-agent** smoke (a running
  agent in the terminal), not a shell. This gates the poke protocol below: a
  bare `\n` written into a running agent's stdin will not submit until this is
  fixed.
- **Graph: "Graph from here" on a directory plots unrelated markdown, not the
  dir's files; and double-clicking directory nodes does not expand them.**
  Repro: seed a workspace from a shallow clone of this repo in `/tmp`, plot the
  repo graph, then "Graph from here" on `./gateway/`. It does not plot gateway's
  files and instead plots a lot of unrelated markdown. Separately,
  double-clicking directory nodes (the expand/collapse meant to companion the
  depth slider) does nothing.

  **[Architect review] These are ONE bug: the in-graph "Graph from here" never
  switches the graph into filesystem mode.** The in-graph action is
  `graphFromHere(path, isDir)` (`web/src/components/GraphPanel.svelte:390`): it
  sets `graphState.scopeId = "dir:<path>"` and `depth = 1` but never sets
  `graphState.mode` (the only `.mode` references in the file, lines 560/565, are
  the `$derived` reads). So re-scoping inside the default semantic repo graph
  keeps `mode === "semantic"`, `filesystemMode` (`:559`) stays false, and the
  load (`:1700`) falls through to the SEMANTIC/link graph scoped to
  `dir:gateway`. That plots the markdown link-neighbourhood (unrelated `.md`),
  and gateway's actual files (Rust) never appear because they are not notes in
  the link graph - **symptom 1**. Double-click expand `onGraphDoubleClick`
  (`:231`) is gated on `filesystemMode`, so it is a no-op - **symptom 2, same
  root cause**. The expand feature itself was implemented (phase-14 `14f2bd14`)
  and is correctly wired (`GraphCanvas.svelte:1271` -> `onSetAsScope`, bound at
  `GraphPanel.svelte:2305` `onSetAsScope={onGraphDoubleClick}` ->
  `toggleDirExpand`), so it is a bug, not a missing feature.
  - Asymmetry: the FILE BROWSER "Graph from here"
    (`openFsGraphForDirectory`, `web/src/state/store.svelte.ts:1687`) DOES force
    `mode:"filesystem"` and works; only the in-graph `graphFromHere`, which
    re-scopes the current tab, inherits the current (semantic) mode.
  - Fix: set `graphState.mode = "filesystem"` in `graphFromHere` for the
    directory case (parity with the `openFsGraphFor{File,Directory}` helpers the
    function's own comment at `:208-214` already claims to match). One change
    fixes both symptoms. Decide whether to also switch the FILE case (the
    comment implies parity, but it changes "graph from here on a file" inside a
    semantic graph from a link-neighbourhood to a filesystem cohort - a small
    product call). Do NOT change the breadcrumb `rescopeFromHere` (`:200`),
    which intentionally preserves mode for in-graph navigation.
  - Frontend-only; browser-smoke required (semantic graph -> Graph from here on
    a dir now shows files + double-click expands; breadcrumb still preserves
    mode; file-browser Graph from here still works).

## Enhancements
### Common functionality across the chan and chan-desktop binaries
We are now embedding chan-server and the MCP server into chan-desktop so that users have a functional MCP server for their workspace. The next thing we need to add in chan-desktop is the `chan shell` functionality, which is enabling `chan-desktop` to recognise `argv[0] == "cs"` and run that code path. This should enable users from chan-desktop to have a fully functional MCP and Hybrid Terminal shell without the `chan` binary on their machine.

**[Architect review] Dropped round-1 item, on-theme for this section: the
`chan open` desktop integration.** Roadmap round-1 (lines 103-106) wanted
`chan open <path>` (the OS file-association entry, distinct from `cs open`) to
assess whether the path belongs to a known workspace, then offer to turn it
on/open, or reject with guidance on creating one. It never landed: `cmd_open`
(`crates/chan/src/main.rs`) just pushes `OpenPath` to the control socket
assuming `$CHAN_CONTROL_SOCKET` / `$CHAN_WINDOW_ID` are set, i.e. it behaves
exactly like `cs open` and only works from inside a chan terminal. Decide:
action this in round-2 (it is the actual desktop double-click path) or defer
explicitly.

### The cs command line
1. Let's rename `cs term` to `cs terminal` to match `cs dashboard` 
2. Let's make the output of `cs terminal list` always pretty-printed
3. Let's add `cs search` to execute the same search we do from the UI, and print the results in markdown by default, with --json and --pretty-json flags 
  4. TODO: if we have other --json and --pretty-json flags across the codebase, let's try to use the same... these are my top of mind flag names, not a mandate
5. Let's enable prefix match for all sub-commands of the `cs` command, similar to how `iproute2` does it:
  6. e.g. in iproute2 you can either run `ip addr show` or `ip a s` to show the addresses
  7. I'd like us to be able to run:
    8. `cs t` for terminal, and all its subcommands as well, e.g. `cs t n` for `cs terminal new`, and `cs t w` for `cs terminal write` and so on
      9. we are going to add `cs terminal restart [--tab-name=x --tab-group=y]` to allow the current terminal to restart after running the command
      10. This will be used by the Team Work bootstrap later
    9. `cs g` for graph
    10. `cs d` for dashboard
    11. `cs o` for open
    12. `cs s` for search

**[Architect review] grounding + reuse anchors for the cs items above:**
- **Rename (`cs term` -> `cs terminal`).** The clap group is `Term` /
  `TermAction` in `crates/chan/src/main.rs`; the rename must also hit the
  `argv[0]=="cs"` symlink integration test and the help text. Pre-release, no
  alias (drop `term` outright).
- **Prefix match is a clap built-in, not custom work.** Set
  `infer_subcommands(true)` on the root + the nested `terminal` enum. Current
  names disambiguate cleanly (o/g/d/t/s top-level; n/w/l/r under terminal), so
  `cs t r` -> `terminal restart` resolves with no hand-rolled prefix logic.
- **`cs terminal restart` reuses existing restart, but needs a by-name path.**
  `Registry::restart()` + `POST /api/terminal/{id}/restart`
  (`terminal_sessions.rs` / `routes/terminal.rs`) already exist, but restart is
  keyed by **session id** via the SPA. Add a control-socket
  `ControlRequest::TermRestart` that resolves by `--tab-name` / `--tab-group`
  and calls `Registry::restart()` server-side (category-2, like `term write` /
  `term list`). This out-of-band server path is exactly what the Team Work
  self-restart needs: a shell cannot restart the very shell running its own
  script. Confirm `restart_options()` re-applies the per-terminal startup
  command/env so the agent relaunches (the team lead already launches via a
  startup command, not by typing into the shell).
- **`cs search` reuses `Workspace::search()`** via the `/api/search/content`
  path (`routes/search.rs`); a new control-socket command returning data on the
  connection like `term list`. Note search is workspace-wide now (round-1
  removed scope).
- **Output flags (resolves the TODO at item 4).** The existing `chan` CLI uses
  `--json` (compact) in ~7 commands; `--pretty-json` exists nowhere, and
  `cs term list` currently emits compact JSON with no flag. Standardize on the
  established convention: human/markdown default, `--json` for compact machine
  output, `--json --pretty` for indented (NOT a new `--pretty-json`). For
  consistency, make `cs terminal list` markdown/table by default with `--json`
  for machine output, rather than "always pretty-printed JSON" (so both
  list-like commands share one default shape).

There was an item in phase-15 round 1 which got lost, the `cs dashboard --carrousel-off` flag to turn the new tab's carrousel off, and the default is on. We will implement this missing flag here, too.

**[Architect review]** Spell it `--carousel-off` (one r) to match the existing
`--carousel-index` flag in code, not `--carrousel-off`. Note `--carousel-index`
DID land in round-1, so the on/off toggle is the only lost dashboard flag.

### Team work
In the team setup dialog, in addition to 'Path to configuration' we are going to add 'Terminal tab group name' with a default name derived from the filename, e.g. `/tmp/new-team-1/chan-team.toml` -> `chan-team`; if the name conflicts with existing terminal groups at the time of creation, we add -N to the name, where N is a counter.
When we setup the team and create the terminals, we can now use the same code path as `cs terminal new --tab-name=x --tab-group=y`  and consolidate onto the same code for all term-related work.
The bug listed about the team work's main terminal not launching the agent should be fixed by using the new `cs terminal restart` command / API.

**[Architect review]** The `-N` conflict suffix needs to enumerate live
terminal groups at creation time: reuse the registry groups already surfaced by
`TermList` (the same source `cs terminal list` reads), so the dialog and the CLI
agree on what "existing groups" means. The self-relaunch fix is sound given the
out-of-band server-side `TermRestart` described above (the bootstrap script
calls `cs terminal restart` against its own tab-name/group; the server restarts
that session so the new shell launches the agent from its startup command).

### Poke protocol (2.2)

When a worker finishes a task it (a) appends to its event file as today and
(b) pokes the target so the event is picked up immediately:

```
cs term write --tab-name=<target> 'poke from <agent>: check <path-to-event>\n'
```

This is the concrete first step of the dispatch-as-automation blueprint: the
event channel plus a directed wake.

**[Architect review]** Grounding:
- It works on the surface that already shipped (`cs term write`, v0.20.0; it
  becomes `cs terminal write` after the rename above). No new CLI is required
  for the poke itself; it resolves the target by `--tab-name` (single) or
  `--tab-group` (broadcast) via the registry-by-name path.
- **Depends on the agent-submit fix (the shift+Enter bug above).** The poke is
  written into a running agent's stdin; a bare `\n` will not submit to an agent
  until Shift+Enter / agent-submit is corrected. Sequence the agent-submit fix
  before relying on agent-to-agent pokes.
- When the target is **@@Host**, the poke must surface as a survey bubble over
  the Lead terminal rather than land as raw text (see 2.3).

### Host survey bubbles + F -> draft.md (2.3)

Events targeted at @@Host should render as chat-style bubbles over the Lead
terminal (the survey mode the Team Work UI already shows). @@Host's reply is
fed back into the Lead terminal; pressing F places the follow-up content into
the `draft.md` already open in the Lead terminal's Editor.

**[Architect review] This is a REBUILD, not a reconnect.** The survey/bubble
backend and the reply round-trip were deleted on 2026-05-29 (`55179ad9`:
`event_watcher.rs`, `watcherEvents.ts`, the rich-prompt routes).
`BubbleOverlay.svelte` survives but is a static demo with a dead F button, and
`TeamWorkState.draftPath` is a vestigial hook. The working version is
recoverable from git history. Scope:
1. **Event pump.** Surface a Host-targeted poke as a bubble by reviving the
   deleted file-watcher endpoint pattern
   (`/api/terminal/:session/watcher/events`, `c69e2fcf`) reading the event file
   the poke references, rather than parsing the Lead terminal stream. (Key call
   to make: dedicated watcher endpoint vs SPA parsing the terminal output. The
   deleted design used the endpoint; recommended.)
2. **Survey data model.** The bubble renders from / questions / options /
   free-text; the event file an agent pokes @@Host with must be in this shape.
   Pre-release: define the wire shape fresh, no compat with the deleted format.
3. **Reply path.** @@Host's reply is fed into the Lead terminal as input via the
   orchestrator's existing `sendInput` / `{type:"input"}` WS path
   (`TerminalTab.svelte:842-844`).
4. **F -> follow-up.** F places the follow-up content into the `draft.md` open
   in the Lead terminal's Editor (repopulate `draftPath`; the old F path
   `75892d7c` quoted into the prompt buffer, the new target is the draft file).

Reuse anchors: git history (`55179ad9` parents, `75892d7c`, `a8b52a00`,
`c69e2fcf`), `web/src/state/teamOrchestrator.svelte.ts`,
`web/src/components/BubbleOverlay.svelte`, `web/src/state/bubbleStub.svelte.ts`,
`TeamWorkState` (`web/src/state/tabs.svelte.ts`). Depth (full rebuild vs a
minimal poke-to-bubble path) is @@Architect's to scope; backend + frontend, so
the full repo gate plus a browser smoke of the bubble round-trip apply.
