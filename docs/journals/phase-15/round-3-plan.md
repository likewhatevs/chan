# Phase-15 round-3 plan

Architect decomposition of @@Host's round-3 backlog comments
(`round-3-backlog-comments.md`), grounded against the current code. Author:
@@LaneA / @@Architect. This is a decomposition + sequencing plan, not an
implementation. Most items are an explicit "let's do"; the calls that are yours
are collected in **Decisions I need from you** below.

Quality rules hold: no em dashes, ASCII tables, WHY comments, full gate before
any push, browser-smoke the runtime-risky bits, no back-compat paths. Push/tag
stays @@Host-only.

## Priority bug (added after your Cmd+R report): RELOAD-HANG

Reframing: this is NOT the terminal Ctrl+R reverse-search (that works). It is the
webview window reload (Cmd+R) hanging the whole session. The backlog filed RELOAD
as a post-release spot-check; your report (it crashed your session) makes it a
session-crashing Wave-1 priority.

Topology: the control socket is owned by chan-desktop itself (pid 31335 =
`/Applications/Chan.app/Contents/MacOS/chan-desktop`). chan-server is EMBEDDED
in-process, so "kill chan-server" = kill the whole app. A stuck webview and a
wedged embedded server are recovered by the same kill, so we cannot tell which
hung from the recovery action alone. We need a thread dump WHILE it is stuck.

ROOT CAUSE - reproduced + confirmed live (2026-05-31):
The SPA boot is gated by a full-screen, no-close, ESC-ignored overlay
(`web/src/components/PreflightOverlay.svelte`) that stays LOCKED while
`GET /api/preflight` reports `locked: true`. The server maps the indexer state
to that gate in `crates/chan-server/src/routes/preflight.rs:106-135`:

    Building   -> Running (locked)
    Reindexing -> Running (locked)   <-- THE BUG
    Idle       -> Done   (ready, unlock)

So ANY incremental watcher reindex (`IndexStatus::Reindexing`) re-locks the
ENTIRE UI. A reload caught during a reindex hard-locks on "Preparing workspace /
Build search index / working..." until the indexer returns to Idle. On a large
workspace a watcher reindex (re-read + re-embed of the changed files) is slow,
and the in-flush chip-freeze pins the status, so the lock lingers and reads as a
permanent hang (server kill required). Small drives reindex in milliseconds, so
the Reindexing window is too short to catch -> they never hang. This matches
@@Host's report exactly: configuring the Dashboard / clicking / flipping panes
writes session+layout files -> watcher -> Reindexing -> then Cmd+R locks.

Reproduced on a 1200-file drive under file churn:
    /api/index/status -> {"state":"reindexing","file":"note-490.md"}  (frozen)
    /api/preflight    -> {"phase":"running","locked":true,"steps":[
                          {"id":"index","label":"Build search index",
                           "state":"running"}]}
    SPA               -> full-screen "Preparing workspace" lock, no escape.
The server HTTP path stayed responsive throughout (`/api/workspace` +
`/api/index/status` answered in ~1.5ms even under an active embed + heavy churn +
14-way CPU saturation), so this is NOT a server deadlock - it is the preflight
gate correctly reporting a state that should not lock the UI.

Theories examined and DISCARDED on the way (recorded so we don't revisit them):
  - broadcast-channel send blocking the PTY reader: FALSE, tokio broadcast send
    is non-blocking/lossy (`terminal_sessions.rs:906`).
  - lsof-in-async / `Cwd` storm on reload: a real LATENT bug (macOS `process_cwd`
    shells out to `lsof` inside the async handler, `terminal_sessions.rs:955`),
    but NOT this hang - the SPA requests cwd only on right-click, not on attach.
    Keep as a separate cleanup, not the reload fix.
  - Embedder mutex / blocking-pool exhaustion blocking boot requests: ruled out
    empirically - boot endpoints answered in ~1.5ms under embed + churn +
    saturation.

FIX (minimal, high-confidence): in `routes/preflight.rs`, an incremental
`IndexStatus::Reindexing` must NOT lock - map it to `Done` (ready) like `Idle`.
Only the cold initial `Building` (first build / `indexed_docs == 0`) should lock
the boot overlay. This honors the stated indexer design intent
(`indexer.rs:853-856`, "avoid re-locking preflight on every interleaved
reindex"). Secondary hardening (Theme 5): the chip-freeze fix + Option B shorten
how long any Reindexing/embedding lingers, but the preflight mapping is the
direct cure. Wave-1 priority; small and verifiable (curl `/api/preflight` stays
`locked:false` during a reindex; a reload no longer locks). The desktop capture
script (`diagnose-reload-hang.sh`) is now superseded by this diagnosis.

## Decisions I need from you

1. **Round-3 shape / resourcing.** Another multi-agent team round (architect +
   ~4 lanes, like round-2), a smaller 2-3 lane round, or solo with you driving?
   The volume here points at a team round; your call on size.
2. **Wave-1 priority.** My recommendation is foundations first (cs-shell
   extraction + per-agent submit map), because that code underpins both the
   survey command and the desktop refactor and it churns `main.rs` broadly.
   Alternatives: editor/search UX first (daily friction), team-work + survey
   first, or IDX embeddings first.
3. **Docs/journals cleanup destructiveness.** You said the raw data "is in the
   git history anyway." Confirm: delete raw round data and keep only
   summaries + hashtags, OR summarize but keep the raw this round, OR defer the
   cleanup to a later round. This one is destructive (cited URLs/IDs in journals
   become dead links), so I want it explicit.
4. **Deferred sub-decision (resolve inside Theme 2, not blocking now):** the
   Linux AppImage `cs` story. An AppImage is a single file with no in-bundle
   symlink, so options are (a) chan-desktop installs a `cs` wrapper into
   `~/.local/bin` on first run, (b) ship the `chan` binary alongside (the odd
   double-tool dependency you flagged), or (c) a tiny `cs` shim. I lean (a).

## Direct answers to the questions in your comments

- **bug-editor "remind me how to test."** BUG-EDITOR is the WKWebView
  conceal-on-tab-switch glitch and is desktop-only (WKWebView, not Chrome/Blink).
  Repro in chan-desktop: open a workspace, open a note so the editor content is
  visible, switch to another tab/window (e.g. a terminal tab), then switch back
  to the editor and watch whether the content blanks/conceals on return and
  whether it self-recovers. Not reproducible via Chrome automation.
- **reload / desktop-open.** You confirmed both work post-release. No round-3
  work; keep as shipped.
- **"What happened to link to markdown sections?"** Your image-3 shows the `[[`
  popup WITH the hint and a Welcome suggestion, and `web/src/editor/bubbles/
  wiki.ts` has heading mode (`fetchHeadings`, ~257-284) and block mode
  (`fetchBlocks` / `parseBlocks`, ~297-329) wired with commit logic (~358-419).
  So the feature most likely still EXISTS. One exploration pass claimed the hint
  text is gone from the code, which contradicts your screenshot, so I am treating
  that as unverified and will browser-smoke it before assuming a regression. The
  real gaps are probably (a) it writes `[[target#anchor]]` wiki-links to disk
  instead of relative markdown, and (b) search not understanding the anchors.
  Both fold into Theme 3 and Theme 4.

## Grounded item map

### Theme 1 - Team Work in-workspace + Survey rebuild (the spine)

Current state:
- Team config lives OUTSIDE the workspace at `/tmp/new-team-1/chan-team.toml`
  (`web/src/state/teamConfigPath.ts:7`), written via `POST /api/team-config/write`
  -> `crates/chan-server/src/routes/team_config.rs` (atomic temp+rename, absolute
  path allowed outside the sandbox). Schema `TeamConfigWire`
  (`web/src/api/client.ts:1121`), dialog state `TeamDialogConfig`
  (`web/src/state/teamDialog.svelte.ts:111`), orchestrator `runTeamBootstrap`
  (`web/src/state/teamOrchestrator.svelte.ts:335`). Member cap is already
  `TEAM_MAX_SIZE=9` (`teamDialog.svelte.ts:159`), validated in `validateTeamConfig`
  (~188-213).
- Survey / bubbles are currently a STATIC STUB: `web/src/state/bubbleStub.svelte.ts`
  + `web/src/components/BubbleOverlay.svelte` (no network, no reply). The real
  backend (event-pump + reply round-trip + F->draft) and the "Rich prompt" widget
  were deleted in commit `55179ad9`; `TeamWork.svelte` (~442-498) still has
  "Collapse/Expand prompt" and bubble-mode menu entries to remove.
- `cs terminal` plumbing: `TerminalAction` (`crates/chan/src/main.rs:490`),
  `cmd_shell_terminal` (~2240), `ControlRequest::TermWrite` client (~1940) and
  server (`crates/chan-server/src/control_socket.rs:33`), session fan-out
  `Registry::write_input_matching` (`terminal_sessions.rs:368`). A new
  `cs terminal survey` slots in as a new `TerminalAction` + `ControlRequest`
  variant that raises the SPA overlay and blocks for the reply.

Round-3 work:
1. Move the team into the workspace under a user-chosen `{team-name}/` dir with
   `config.toml`, `bootstrap.md`, `tasks/task-{from}-{to}-{n}.md`,
   `journals/journal-{member}.md`, `followups/followup-{from}-{to}-{n}.md`. Route
   reads/writes through `Workspace::{read_text,write_text}` (sandbox + atomic),
   not the current outside-sandbox path.
2. Validate config on reload (reuse the <=9 cap plus structural checks).
3. Generate `bootstrap.md`: the process for all members, the roster, reveal
   @@Host and @@Lead, the poke 1-liner protocol, and the hold-for-@@Lead
   distribution flow.
4. Delete the "Rich prompt" widget and all dead survey/bubble-stub code
   (bubbleStub, stub BubbleOverlay payloads, TeamWork menu entries, leftover
   rich-prompt references).
5. Build `cs terminal survey` (raise bubbles over a tab or a group of tabs):
   single-question mode, markdown problem body, up to 4 vertically aligned
   options plus `[F]` follow-up. Returns the chosen option to the caller; `[F]`
   creates a pre-populated `followups/followup-...md` (header/title, date+time,
   "Agents: this is a follow up, not ready; check again later", the original
   prompt, and @@Host comment placeholders) and returns its path.
6. Rebuild the SPA survey overlay for real (reply round-trip), replacing the stub.

Couples to: the per-agent submit map (team config needs each member's agent
TYPE) and the cs-shell extraction (the `cs terminal survey` command).

Architect design call (for confirmation, not blocking): `cs terminal survey` is a
SYNCHRONOUS control-socket call. The agent's CLI blocks until you pick an option,
then prints the option (or the new followup path) to stdout. This matches your
"the tool returns that option" wording.

### Theme 2 - Desktop / CLI consolidation

- **cs-shell extraction (DESKTOP-SHELL).** Lift the cs CLIENT into a new
  `chan-shell` crate so both `chan` and `chan-desktop` depend on it: `ShellAction`
  / `TerminalAction` (`main.rs:435-554`), `cmd_shell` / `cmd_shell_search` /
  `cmd_shell_terminal` (~2082-2345), `send_control_request` (~2389), the client
  `ControlRequest` / `ControlResponse` (~1911-1967, a DUP of
  `chan-server/src/control_socket.rs:33-95`), `open_env` / `control_socket_env`
  (~1969-2009), the render helpers (~2177-2386), `AGENT_SUBMIT_CHORD` (2227), and
  the `argv[0]=="cs"` rewrite (`parse_cli`, ~769-786). RISK: cross-crate clap
  derive + serde tags must stay byte-identical or every cs command breaks at
  runtime (gate-blind), so wire-smoke EVERY cs command, not just a green build.
  Bonus: this kills the client/server `ControlRequest` duplication.
- **chan-desktop `shell` + `argv[0]=="cs"`.** chan-desktop depends on `chan-shell`
  and dispatches when invoked as `cs`, so desktop users get cs + MCP without the
  `chan` binary. chan-desktop already depends on `chan-server` / `chan-workspace`,
  not the `chan` binary.
- **Remove `chan open`.** Today `chan open` (`main.rs:160-165`, `cmd_open`
  ~2021-2063) does double duty: inside a terminal it sends `OpenPath` over the
  control socket; outside (the OS file-association entry, commit `05e9b9eb`) it
  does the registry longest-prefix match + `maybe_handoff_to_desktop` (~1379-1425).
  Move the file-association + handoff entry into chan-desktop (the GUI app becomes
  the file handler directly); the inside-terminal "open in current window" stays
  as `cs open` (`chan shell open`).
- **Per-agent submit-encoding map.** One shared map across THREE consumers:
  `main.rs apply_submit_chord` (2232), `terminal_sessions.rs
  SubmitMode::submit_chord`, and `web/.../submitMode.ts encodeForAgentSubmit`.
  Shape `--submit=<agent>` (claude=`\x1b[27;9;13~`, codex=`\r`, gemini=TBD probe
  live); unset = pure bytes (already the default). Team config grows a per-member
  (or team-wide) agent field so bootstrap pokes use the right encoding. You said
  "we need proper smoke tests for the team work plumbing here", so this includes
  real codex + gemini auto-submit smoke tests, not just a green build.

### Theme 3 - Editor link & cursor UX

- **Relative-markdown links on disk.** `[[` completion currently writes true
  wiki-links `[[path#anchor]]` (`web/src/editor/bubbles/wiki.ts` commit ~358-419,
  `linkTargetRef` ~374-379) while images already write relative markdown
  `![](./x.png#w=...)` (`bubbles/image.ts:290-318`, `image_drop.ts:250-262`). New
  rule: ALL new insertions from `[[` (files, images, docs, headings, blocks) emit
  relative markdown `[](./path.md#anchor)` via `links.ts wikiLinkToMarkdown`
  (~19-48, which already relativizes). KEEP existing wiki-links only when the file
  already contains `[[` (per-file mode detected on load). New files always
  relative. When `[[` is hit in a file that has not been indexed, prioritise the
  current file's directory for suggestions.
- **`[[` stuck on "Indexing...".** The popup reads cached `indexStatus.value`
  (`bubbles/wiki.ts` + `bubbles/empty_state.ts:19-39`, `indexInProgress`) and can
  sit on "Indexing... searched 0 documents" until Enter/paste because the empty
  state is not re-invalidated when indexing finishes mid-query. Fix the
  invalidation.
- **Heading `#` / block `^` section links.** Verify end-to-end (browser smoke),
  ensure they round-trip on disk as relative-markdown anchors, and ensure search
  resolves the anchors. Anchors: `wiki.ts` 257-336, `triggers.ts classifyQuery`
  85-94.
- **Click-to-place-cursor anywhere on a row / past EOL.** Clicks in the blank
  space after text fail to place the caret; image widgets are atomic ranges
  (`web/src/editor/widgets/image.ts` ~290-296, `posAtCoords` ~741-742) and there
  is no line-level click handler mapping an empty-space click to the line's last
  position. Add cursor placement that maps a click anywhere on a row to the
  nearest text position (and past-EOL to the end of the line's text).

### Theme 4 - Search understands mentions / paths

- BM25 (tantivy default tokenizer, `crates/chan-workspace/src/index/bm25.rs`
  ~291-356) splits on `@`, `/`, `.`, so `@@mention` becomes `mention`,
  `path/to/file` becomes `path`/`to`/`file`, and `file.md` becomes `file`/`md`.
  Semantic (BGE) search embeds raw chunk bodies (`index/facade.rs` hybrid/RRF
  ~1088-1111), so it matches these by meaning but cannot distinguish a mention
  node from the bare word.
- Round-3: PROBE first (your "maybe we already are with semantic search"). Run a
  live search for `@@handle`, `path/to/file`, and `.md`. If semantic coverage is
  adequate, no work. If exact-token matching is wanted, add a tokenizer/analyzer
  (or a query rewrite) that preserves `@`, `/`, `.`. Anchors: `routes/search.rs`,
  `SearchPanel.svelte`.

### Theme 5 - IDX embeddings hardening

- **Option B: embeddings as a proper background job.** Change
  `reindex_with_aggression` (`crates/chan-workspace/src/workspace.rs` ~2142-2235)
  so the embed step is a fully separate background job with its own status,
  instead of inline-in-`build_all` (`index/facade.rs` ~500-755). Status enum
  `IndexStatus` + `EmbedProgress` (`crates/chan-server/src/indexer.rs` ~39-79).
- **Chip clobber.** The watcher sets `Reindexing` then `set_idle{embedding:None}`
  (`indexer.rs` ~429-436, `set_idle` ~983), dropping the embed chip. Needs a
  SHARED bg-embed signal independent of the reindex status (couples to Option B).
- **In-flush chip freeze.** The candle BERT forward pass in `flush_embed_batch`
  (`facade.rs` ~860-924) blocks the progress thread, so the chip can sit frozen
  inside one flush. Levers: a heartbeat tick during the forward pass, and/or a
  smaller `EMBED_BATCH_CHUNKS` (`facade.rs:1145`).
- **Metal hang follow-up (new).** GPU/Metal was forced to CPU because the candle
  Metal backend hangs in `waitUntilCompleted` (`index/embeddings.rs` ~347-418,
  `CHAN_ENABLE_GPU` gate). Add a follow-up to investigate the hang and re-enable
  Metal on macOS.

### Theme 6 - Docs/journals cleanup + graph hygiene (separate agent, LATE)

- A dedicated agent: delete the raw round data in `docs/journals/` (preserved in
  git history), summarize each phase (including phase 15) into essence docs,
  transcribe + delete the images, and tag outcomes with hashtags (`#reliability`,
  `#features`, `#bugfixes`, ...).
- Graph hygiene when chan's own source is the workspace: no ghost nodes
  (unresolved link targets) and far less doc clutter. Likely BOTH a graph-render
  change (drop unresolved-target nodes) AND the data cleanup. Couples to the
  relative-link rule (Theme 3); run AFTER that lands so the cleanup emits relative
  links. DESTRUCTIVE: cited URLs/IDs in journals become dead links, so scope is a
  @@Host call (see decision 3).

## Recommended sequencing (waves)

- **Wave 1 (foundations).** cs-shell extraction (`chan-shell` crate) + per-agent
  submit-map design [Theme 2 spine]. In parallel and independent: editor
  relative-link rule + `[[` stuck bug [Theme 3]; IDX Option B scoping [Theme 5].
- **Wave 2.** Team-work-in-workspace + survey rebuild + `cs terminal survey`
  [Theme 1] (needs cs-shell + the submit map); remove `chan open` + desktop cs
  path [Theme 2]. In parallel: click-to-cursor + heading/block verify [Theme 3];
  search mentions/paths probe [Theme 4]; IDX chip fixes [Theme 5].
- **Wave 3.** Docs/journals cleanup + graph hygiene [Theme 6] (after relative
  links land); Metal hang investigation [Theme 5]; full smoke + release prep.

Rationale: cs-shell touches `main.rs` broadly and underpins both the survey
command and the submit map, so it leads and de-risks downstream churn.
Editor / search / IDX are independent and parallelize. Docs cleanup is
destructive and depends on the relative-link rule, so it runs last.

## Verification (per theme, end-to-end)

- Full gate before any push: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo build --no-default-features`, web svelte-check + `npm run build`; the
  release gate additionally builds the gateway workspace.
- Theme 1: serve a test drive, open Team Work, bootstrap a team, confirm the
  `{team-name}/` tree lands inside the workspace; run `cs terminal survey` from a
  terminal tab and confirm bubbles render, an option returns to stdout, and `[F]`
  writes a followup file and returns its path. Real-agent smoke for any submit
  path.
- Theme 2: wire-smoke EVERY cs command after the extraction (new + alias forms);
  smoke codex + gemini auto-submit; confirm `chan open` is gone and the desktop
  handles the OS file-association + handoff; AppImage `cs` per the chosen option.
- Theme 3: browser-smoke that `[[` completion writes relative markdown on disk,
  the stuck-Indexing bubble resolves, heading/block links round-trip, and clicking
  blank space / past EOL places the caret. (Static gates miss Svelte-5 runtime +
  CodeMirror timing, so this needs a running server.)
- Theme 4: a live search probe for `@@handle`, `path/to/file`, and `.md` before
  and after any change.
- Theme 5: reindex a drive and watch the chip advance through an embed flush
  without freeze or clobber; confirm the Option B status; test the Metal re-enable
  behind the gate.
- Theme 6: graph render on chan-source shows no ghost nodes; spot-check the
  summaries + hashtags; confirm the raw-data deletion scope per decision 3.
