# journal-LaneD (phase-17, round-1)

Append-only. Owner: @@LaneD (worker - platform: server/CLI/workspace + docs).

## 2026-06-02

- Bootstrapped. Read team/bootstrap.md, confirmed identity from
  $CHAN_TAB_NAME=@@LaneD. Lane = platform (server/CLI/workspace) + docs/website.
- Round-1 lane items: B5 (MCP env off by default), B10 (chan serve progress),
  B11 (editable-text by content), D1 (README + website reframe).
  Wave 1 for me: B11, B10. Wave 2: B5. Wave 3: D1 (draft early, verify late).
- Holding. Waiting for @@LaneA to cut a task into team/tasks/ and poke.
- Got task-LaneA-LaneD-1: B11 + B10 (Wave-1).

### B11 - editable-text by content (DONE, own-gate green)
- Recon: classify() is path-only; editable gate in workspace.rs read_text/
  write_text(+stat/chunked/if_unchanged); wire kind in files.rs project_kind;
  TS mirror kinds.ts/fileTypes.ts; api/types.ts kind union.
- SCOPE FLAG to @@LaneA (followup-LaneD-LaneA-1, non-blocking): "searchable" =
  BM25 = path-based is_indexable_text across ~15 hot-path sites incl the index
  walk B10 targets + pollutes BM25 with .S/Kconfig noise. Shipped editable-core
  (editor opens + browser shows-as-text); recommend deferring BM25-of-sniffed.
  Proceeded on the core (correct under either decision). Still unanswered; not
  blocked.
- Impl: fs_ops::{TEXT_SNIFF_BYTES, looks_like_text} (NUL + UTF-8, tolerate
  trailing partial multibyte; no dep). workspace::sniff_is_text + private
  editable_text_gate; 5 read/write gates now sniff. files.rs: project_kind
  Other->"pending"; per-dir listing resolves pending->text/binary (bounded,
  browser lists one dir); read route falls back to binary only on
  NotEditableText. TS: "pending" added to kind union + display maps; classifyPath
  stays binary-fallback (can't sniff client-side). Dropped the async "pending
  refinement push" - per-dir sniff is cheap+synchronous (browser is per-dir).
  graph.rs untouched (sniffed text never a doc node; gated on is_markdown_file).
  inspector.rs file_kind_label left path-only "binary" (aggregate stat; sniffing
  per-file for counts = the whole-tree cost we avoided) - minor cosmetic gap,
  flagged.
- Tests: looks_like_text (text/NUL/PNG/truncated-multibyte), workspace gate,
  read_file_sync sniff, list_files_sync pending resolution. Updated 1 existing
  test (archive.zip path-kind now "pending").
- Empirical (API smoke, /tmp/b11test): .zshrc + app.service -> kind "text" +
  read returns text(200); note.md -> "document"; PNG-bytes-no-ext -> "binary" +
  octet-stream (real binary STILL refused, no false positive). Backend verified.
  Browser-VISUAL (editor render + tree icon) needs a client - flagged for @@LaneA.

### B10 - chan serve progress (DONE, own-gate green; + a bigger finding)
- Empirical first: URL prints at ~0.1s even on a 93k-file tree (open is lazy,
  indexer is background spawn_blocking). So the "silent window" is NOT pre-URL
  indexing as framed.
- DEEPER ROOT CAUSE (timing-probed): on a content-heavy FIRST serve the ~13s
  pre-URL stall is `workspace.watch()` (notify recursive watch), NOT indexing
  (num_indexed=35ms, indexer.spawn=5ms). 9000-md vault: watch()=13.0s. Source
  trees index sub-second (60k files/2400 md = 0.68s), so per-file progress is
  sparse there; the walk that finds indexables is uninstrumented.
- Shipped (low-risk): a cold-gated heads-up printed BEFORE watch() (at 0.06s) so
  the pre-URL stall isn't silent ("preparing this workspace... URL prints below
  when ready, indexing continues in background"); + a stderr tee of the existing
  ProgressEvent stream (throttled 750ms, self-gated >800ms elapsed, -v adds
  labels) so a content-heavy build streams "chan: building graph N/9000 (NN%),
  ~Ns left". Verified on the 9000-md vault. ServeConfig.verbose plumbed from
  cli.verbose; tunnel/host set false.
- ESCALATE to @@LaneA: eliminating the 13s itself = making watch() setup async
  (URL immediate, watcher primes in background). That's chan-workspace watcher
  surgery (correctness window) - recommend a deliberate follow-up, did NOT do it
  under release pressure. Heads-up addresses "silent"; the 13s remains.
- Gate: fmt --check clean; clippy -p chan-workspace -p chan-server -p chan
  --all-targets -D warnings clean; cargo test (ws 58 + server 398 + chan 537 +
  subcrates) green; make web-check + svelte-check(0 err) + npm build green;
  cargo build --no-default-features green.
- Files (pathspec sha256 of my diff = b29ba5241fd8d224): chan-workspace/src/
  {fs_ops,workspace}.rs; chan-server/src/{lib,host,routes/files}.rs;
  chan/src/main.rs; web/src/{api/types,state/fileTypes,state/kinds}.ts.
  NOTE: B11 gate lives in workspace.rs (not in my explicit fs_ops/indexer list);
  no peer touches workspace.rs this round (@@LaneA acknowledged this WIP).
- Reported to @@LaneA via task-LaneD-LaneA-1.md.

### B5 - MCP env off-by-default + team toggle (Wave-2, DONE my core)
- @@Alex confirmed: GLOBAL off for ALL agents + opt-in. Recon: TeamConfig is
  chan_workspace::teams.rs (chan-shell carries raw TOML, not the typed struct);
  CreateOptions has no Default (each site sets mcp_env); prod true-sites =
  control_socket spawn_team:702 + routes/terminal.rs WS(181)/HTTP-create(249);
  terminal.rs:714 is a TEST; only 1 TeamConfig literal (team_config.rs sample).
- CHECKPOINT 1 (poked @@LaneA): landed the coordination struct + default-off
  core, compiles green. Field = chan_workspace::TeamConfig.mcp_env: bool
  (#[serde(default)] => false). Non-team pref = ServerConfig.terminal.mcp_env:
  bool (#[serde(default)] false). spawn_team reads config.mcp_env; WS/HTTP create
  default to terminal.mcp_env (off), ?mcp_env=on overrides. So B + A could build
  their surfaces against the landed field.
- Invariant (part 1) CONFIRMED + kept: set_mcp_env only does cmd.env(...) (no
  disk writes); grep found NO writes to any agent config path (.codex/mcp.json/
  claude/codex/gemini config). chan never writes user MCP/agent config.
- Cosmetic (folded in per @@LaneA): team_config.rs::submit_chord_literal - split
  codex from gemini, codex now reads "bracketed-paste + \r" (B8 paste-wrap);
  updated the doc + the pinning test (806).
- Fixed 2 test literals for the new fields (team_config sample_config + config.rs
  round-trip fixture).
- Own-gate green: fmt --check clean; clippy -p chan-workspace -p chan-server
  -p chan-shell -p chan --all-targets -D warnings clean; cargo test (server 398
  incl mcp_env_off_omits_chan_mcp_vars, shell 34, chan 537) green;
  --no-default-features build green.
- Empirical: search WORKS on an MCP-off server - /api/search/content?q=pineapple
  returned the hit (mode bm25), the same Workspace::search cs proxies to via the
  control socket (orthogonal to MCP env). (Installed `cs` connect ENOENT in this
  sandbox is an environmental binary-version artifact, not a B5 regression: the
  socket exists, server alive, HTTP search works.)
- Files (pathspec sha b5 = 59b83eb761346208): chan-workspace/src/teams.rs;
  chan-server/src/{config,control_socket,routes/terminal,routes/team_config}.rs.
  control_socket.rs touch = spawn_team region (~702) ONLY, not the pane-exec
  region (~102) @@LaneB's B4 uses - lead serialized my burst before B4.
- @@LaneB (cs new/load mcp_env in TOML) + @@LaneA (TeamDialog toggle) build the
  remaining surfaces on my landed field. My core is checkpoint-2 green.
- Reported via task-LaneD-LaneA-2.md.

### D1 (README + website reframe) + R2-1 (attribution) - Wave-3 + round-2, DRAFTED
- D1 task-LaneA-LaneD-3: draft early, verify/publish late (@@Alex away).
- KEY FINDING: the About/attribution page is IN-APP (EmptyPaneCarousel.svelte
  slide 0 "about-licenses"), NOT web-marketing. story.html is a narrative.
  So R2-1 edits a frontend component (outside my platform/docs lane) - uncontended
  (no peer WIP on it, not in any task file); authorized by the R2-1 assignment.
- D1 done (drafts + verifiable audit):
  - README.md: new "## Quickstart" opening with the curl|bash install ->
    git clone -> `chan serve ./chan` -> IDE flow + the in-browser/desktop note.
  - web-marketing home.html: a "quickstart" section after the hero (same
    usage example) so the site OPENS with it.
  - /dl links: already present (install.html cli-linux-x64/arm64 + macos-arm64
    tarballs + desktop cards; home desktop downloads) - verified, no change.
  - NEW docs/manual/desktop.md: chan-desktop install + local + remote attach
    (outbound HTTP/2 + inbound reverse tunnel) + ssh -L + lima notes. WKWebView
    New->Remote click-paths marked PENDING @@Alex hand-smoke (agents can't drive
    WKWebView).
  - NEW docs/manual/gateway.md: gateway services table (identity/workspace-proxy
    /profile/admin) + how `chan serve --tunnel-url` uses it + self-deploy;
    cross-links github.com/fiorix/chan/tree/main/gateway. Grounded in
    gateway/README.md (NOT chan-writer - that org is gone).
  - docs/manual/index.md: nav links to both new pages.
  - web-marketing `npm run build` + `npm run check` GREEN (renders both new
    pages, local-link check passes, install.sh sh -n).
- COMMAND AUDIT: all cited flags exist (--tunnel-url/--tunnel-token/--port/
  --no-browser/--standalone + CHAN_TUNNEL_TOKEN). `chan serve` VERIFIED
  (B10/B11). install.sh syntax-checked by the gate. FLAGGED not-run-here:
  curl|bash (needs live site), git clone (repo PRIVATE pre-release), ssh -L,
  lima remote serve, `chan serve --tunnel-url` E2E (needs live gateway). @@Alex
  hand-smoke: WKWebView New->Remote->Outbound/Inbound.
- R2-1 done: added an "about-credits" block to EmptyPaneCarousel.svelte after
  the licenses block - the tagline ("Built on a strong open-source foundation.
  Chan is free and open-source software.") + the verified stack grouped
  browser/server (Svelte, xterm.js, CodeMirror, Mermaid, Cytoscape+d3-force,
  KaTeX, Lucide; axum/Tokio, Tantivy, Candle/BGE, notify, rust-embed,
  portable-pty, yamux/h2; Tauri desktop). Used CANONICAL mermaid.js.org (the
  report's mermaid-cjv.pages.dev is a non-canonical mirror - FLAG). New pinning
  test added; vitest 47/47; vite build compiles; my-file svelte-check clean.
- FLAG to @@LaneA: whole-tree svelte-check is RED from YOUR B5 TeamDialog WIP -
  TeamDialogConfig.mcpEnv is required but teamOrchestrator.test.ts (83/320/352),
  teamBootstrapOrchestrator.test.ts (70), teamLeadRestart.test.ts (55) fixtures
  lack it. Not my files; your lane to fix.
- Pending: @@Alex WKWebView hand-smoke; a visual browser-smoke of the About
  slide (static content, low risk - for the joint frontend pass); optional lima
  E2E tunnel run for deeper verification.
- Files (pathspec sha = 756fa643ccedd385): README.md, web-marketing/src/pages/
  home.html, docs/manual/{index,desktop,gateway}.md, web/src/components/
  EmptyPaneCarousel.svelte + dashboardTabAndCarousel.test.ts.
- Reported via task-LaneD-LaneA-3.md.

### B5 e2e toggle smoke (both surfaces landed; @@LaneA asked to run it)
- Live cs-driven smoke is impractical in this sandbox: `cs` is feature-gated
  into the chan binary (chan-shell is a lib), and the control-socket UDS
  connect fails cross-process in the sandbox (socket binds + HTTP works, but a
  separate-process UDS connect ENOENTs). So I did the rigorous, reproducible
  equivalent: an in-codebase e2e test.
- Added control_socket::tests::spawn_team_mcp_env_toggle_reaches_member_pty_env:
  spawn_team with a probe member (command prints `<<MCP:${CHAN_MCP_SERVER_JSON:+set}>>`)
  on a registry WITH an mcp_socket_path; reads the marker off the member's PTY
  scrollback (term_scrollback). config.mcp_env=true -> "<<MCP:set>>" (CHAN_MCP_
  stamped); config.mcp_env=false -> "<<MCP:>>" (omitted). PASSES (0.23s). This
  exercises the full chain TeamConfig.mcp_env -> spawn_team -> CreateOptions
  .mcp_env -> set_mcp_env -> child PTY env, the new link my B5 added.
- Gate: fmt clean; clippy -p chan-server clean; chan-server tests 399 (was 398,
  +my e2e). Updated B5 pathspec sha (incl the test) = 8dd649548539fac4.
- For a true LIVE smoke @@Alex can run on a real machine: serve a workspace,
  `cs terminal team new/load` a team with mcp_env on/off in config.toml, then in
  a member terminal `env | grep CHAN_MCP_` (present iff on). Documented for him.
- NOTE: @@LaneA's "then proceed with D1+R2-1" crossed my D1+R2-1 completion poke;
  those were already done last turn (task-LaneD-LaneA-3).
- Confirmed (re-run after @@LaneA's mcpEnv fix): B5 e2e PASS + whole-tree
  svelte-check 0 errors on the integrated tree.

### Heads-up absorbed: scoped-gate blind spot (B10 chan-desktop)
- @@LaneA: B10 ServeConfig.verbose missed desktop/src-tauri/src/embedded.rs:104
  (chan-desktop is a separate Cargo workspace); full pre-push caught it, @@LaneA
  fixed (verbose:false), rides in my B10 commit. My grep was crates/-scoped +
  cargo check default-ws only. Logged to memory
  [[feedback_scoped_gate_misses_separate_workspaces]].

### R2-3 transport (task-LaneA-LaneD-4) - DONE
- ~2-line additive: WindowCommand::OpenSurvey gains tab_name: Option<String>,
  pinned to wire camelCase `tabName` (serde rename + skip_if None). handle_survey
  threads the existing selector: --tab-name=X -> Some(X), --tab-group/none ->
  None (SPA keeps window-wide fallback). SurveySpec/reply/survey_id/bus
  UNCHANGED. Did NOT touch the B4 pane-exec region.
- Added open_survey_frame_serializes_tab_name_as_camel_case_tabname (pins
  command==open_survey, tabName when Some, no snake_case tab_name, omitted when
  None) - guards gate-blind wire rename.
- Gate: cargo check -p chan-server green; fmt+clippy clean; chan-server 400.
  control_socket.rs cumulative sha = fd11557ba32459b5.
- Poked @@LaneA so @@LaneB reads the real tabName. My last round-2 item.
