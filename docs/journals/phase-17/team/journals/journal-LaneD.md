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

### R2-#4 desktop connecting screen (round-2/desktop-connecting-screen.md) - DONE my slice
- Role = connecting-screen PAGE/UI (@@LaneD); @@LaneB owns src-tauri detection +
  redirection, @@LaneC verifies. Re-bootstrapped post-/clear from $CHAN_TAB_NAME.
- Recon: outbound windows load WebviewUrl::External(remote) via serve.rs
  build_workspace_window; unreachable remote -> WKWebView paints blank white
  (the nil-URL panic guard at serve.rs ~443 documents the exact case). frontendDist
  =../src, withGlobalTauri:true (window.__TAURI__ global like main.js), strict CSP
  (script-src 'self' -> external JS only; default-src 'self' -> page CANNOT fetch
  the remote, so detection MUST be a Rust IPC).
- Built 3 NEW files under desktop/src/ (no overlap with launcher index/main/styles):
  connecting.html, connecting.css, connecting.js. connecting.html <link>s styles.css
  (read-only reuse of theme tokens + base) + connecting.css (spinner, log rows,
  layout). Page: animated ring spinner, "Connecting to {workspace}" + the URL line,
  live elapsed timer (MM:SS / H:MM:SS) + attempt counter, a scrolling log with ONE
  wall-clock-timestamped row per attempt (info/pending/fail/ok colors), footer
  "keeps retrying until you close it". Success -> green check + "Opening
  workspace..." then navigate. Hard-error (no URL) -> red static ring, no misleading
  spinner. Mirrors the launcher theme via shared-origin localStorage.chanDesktopTheme.
  prefers-reduced-motion drops the spin.
- FIRST built to a self-proposed query-param contract; @@LaneB then posted the real
  contract. REBUILT connecting.js to it exactly:
  * inputs via injected `window.__CHAN_CONNECTING__ = { url, target }` (init script,
    pre-script) NOT query string; url = display+probe, target = navigate (carries
    ?w=<label>+#fragment). Page treats target as opaque, replaces verbatim on
    success (all w=/token/zoom construction stays Rust-side).
  * detection = `invoke('probe_url', { url }) -> { reachable, status, detail }`;
    reachable=true for ANY HTTP response (even 401/404), false only on transport
    failure (the blank-white case to retry past). Rejected IPC = failed attempt.
  * page owns the loop (2s cadence, no overlapping probes), timer, rows, success
    nav; probe_url stays stateless. No events (avoids the late-listener race
    @@LaneB flagged).
  Kept a query-param + simulated-probe fallback so the page renders standalone with
  no Tauri (?demo=ok/fail) for dev smoke.
- Smoke (Chrome, served desktop/src over a loopback http server on a PRIVATE port
  after a collision with another lane's chan-lane on :8799; moved to :8913, bound
  127.0.0.1, torn down after): verified fail-path (7 timestamped rows accruing +
  live timer + "keeps retrying"), success-path (green check + connected (HTTP 200)
  + Opening workspace), no-URL hard error, dark + light theme token inheritance.
  JS-eval confirmed the new {reachable,status,detail} shape surfaces `detail` in the
  fail row. node --check connecting.js clean. Tore down server (:8913 free) + tab.
- Gate: pure static frontend assets; `make -C desktop check` is Rust-only and
  unaffected (no Rust touched by me). My files: desktop/src/connecting.{html,css,js}.
- BLOCKED-on for live end-to-end: @@LaneB's probe_url + outbound redirection +
  allow-probe-url permission. My page is contract-complete; checked the LaneD
  Status box in the task file. Reporting to @@LaneA.
- @@LaneA coordination: poked @@LaneC to run STAGE-1 (page-in-Chrome, stubbed
  inputs) ahead of the integrated build; wrote the recipe into the task file.
- STAGE-1 RESULT (@@LaneC, 2026-06-03): PASS, ZERO visual defects -> no page
  changes needed. Confirmed: immediate paint (never blank white), spinner +
  "Connecting to workspace" + {url}, live MM:SS timer (1s tick), one red
  wall-clock-timestamped row per attempt accruing to attempt 10 with no
  give-up + auto-scroll, demo=ok green check + "connected (HTTP 200)" +
  "Opening workspace...", dark+light both clean, ZERO console/CSP errors.
  @@LaneC independently confirmed the :8799 chan-lane collision (used :8913).
  Correctly deferred to stage-2 (need Tauri): the real location.replace(target)
  + the no-url hard-error state. Page is visually + behaviorally signed off
  standalone; only stage-2 (LaneB integrated build) remains.

### Survey Part C (round-2/survey-system.md) - pairing with @@LaneB - web slice DONE
- @@LaneA freed me from #4 to pair on Part C: every cs terminal survey overlay
  must show options PLUS an [F] follow-up AND a Dismiss; dismiss returns a
  distinct "dismissed" reply so the asking agent can tell. @@LaneB owns Part A
  (window_id) + the survey route/spec; I take the overlay.
- Recon: overlay = BubbleOverlay.svelte; state = survey.svelte.ts; reply type =
  client.ts SurveyReplyRequest (option|followup); Rust SurveyReply enum in
  chan-shell/wire.rs (Option|Followup), route in chan-server/routes/survey.rs.
  Today [F] is gated on allowFollowup+context; there is NO dismiss (overlay was
  intentionally non-dismissable b/c CLI blocks on the reply).
- COORDINATION: proposed a clean non-overlapping split - web/ = me (BubbleOverlay
  + survey.svelte.ts + SurveyReplyRequest type + the 2 tests), crates/ = @@LaneB
  (SurveyReply::Dismissed + route + cli + always-populate-followup-context),
  single shared contract = the reply JSON. Wrote it into the task file "Part C
  contract + ownership" + poked @@LaneB to confirm/counter. Built immediately
  (web side compiles+tests independently of the Rust; safe under either LaneB
  decision).
- Built (web slice): client.ts SurveyReplyRequest += {kind:"dismissed"} + followup
  context nullable; survey.svelte.ts ungated requestFollowup (sends followup ??
  null) + new dismissSurvey (posts {kind:"dismissed"}, clears slot, busy-guard);
  BubbleOverlay always renders F + Dismiss in a .survey-actions row, keys
  1..N/f/Escape (Escape = real dismiss reply now, stopPropagation so it does not
  bubble to other overlays). Left SurveySpec.allowFollowup alone (LaneB's field;
  UI just stops reading it) - flagged the lockstep when LaneB drops it.
- Tests: survey.svelte.test.ts (followup-without-context posts null; dismiss
  posts+clears only that slot; failed-dismiss keeps survey+clears busy);
  BubbleOverlay.test.ts (F + Dismiss render on the bare default spec). Replaced
  the 2 old allowFollowup-gated [F] tests.
- Gate (own): svelte-check 0 err (1 pre-existing RichPrompt warning, not mine);
  full vitest 1670/1670 incl the real-Svelte jsdom overlay mount; vite build
  clean. Web side green independent of crates/.
- DEFERRED: the live integration smoke (raise a real survey -> click/Escape
  Dismiss -> agent gets "dismissed") needs @@LaneB's route to handle
  kind:"dismissed"; that is the natural joint step once crates/ lands. The
  reactivity is covered by the runtime jsdom mount (no $state-in-$derived added;
  dismiss mutates state only in event handlers, like the existing pickOption).
- Files: web/src/api/client.ts, web/src/state/survey.svelte.ts,
  web/src/components/BubbleOverlay.svelte + the 2 test files. Reported to
  @@LaneA + @@LaneB.
- INTEGRATION (both slices in the shared tree): @@LaneB landed crates/ green;
  reply JSON matches my contract exactly (verified by reading routes/survey.rs
  SurveyReplyRequest: option/followup(nullable)/dismissed, camelCase). @@LaneB
  counter: F-without-context = FALLBACK (route accepts followup:null = bare
  defer, no file; CLI opts into a file via --followup-dir), NOT always-populate.
  My SPA already sends followup ?? null, so NO web change needed - aligned.
- Smoked the integration (sandbox-allowed level): cargo build -p chan GREEN
  (web bundle + LaneB crates link). WIRE smoke (renamed binary, private port
  4717, bearer POST /api/survey/reply, torn down clean): all 3 reply kinds
  deserialize -> 404 "no survey parked" (parsed past the shape into the bus
  lookup); control {kind:"bogus"} -> 422 "unknown variant, expected one of
  option/followup/dismissed" = the route accepts EXACTLY my 3 kinds. followup:
  null path confirmed (LaneB's bare-defer). Build+wire empirically green.
- DEFERRED: live human-loop visual (cs-raised survey -> overlay -> Dismiss ->
  agent CLI prints "survey dismissed") + Part A live (overlay reaches team-
  dialog terminal). cs cross-process UDS blocked in this sandbox (B5 finding) ->
  real machine / webtest lane. Documented in task file "Part C integration
  smoke". Part C is implementation-complete + integration-verified short of the
  human loop.
- allow_followup DROP (@@LaneA-ratified, @@LaneB did the Rust side): my
  synchronized half done - removed allowFollowup from client.ts SurveySpec + the
  2 test fixtures + 3 stale comments (grepped both casings, zero left in
  web/src). Re-gated GREEN: svelte-check 0 err, vitest 1670/1670, build clean;
  web/dist refreshed. Lockstep complete; field is gone end to end (Rust + TS).

### R2 pre-flight bubble checkmark toggle (round-2/desktop-refinements.md) - DONE
- Task: the workspace-ready onboard bubble showed "Semantic search OFF [Turn on]"
  / "Reports ON [Turn off]" (confusing label+button pair); replace BOTH with ONE
  checkmark toggle per row, keyboard-accessible, same enable/disable calls.
- Found it: PreflightOverlay.svelte onboard-card ("<workspace> is ready"), two
  layer rows (Semantic, Reports). Semantic had a 3-state button (Turn off /
  Download & enable / Turn on) because enabling needs the BGE model.
- Built: each row is now `<button role="checkbox" aria-checked aria-label>`
  (whole row = click+keyboard target; Space/Enter native; SR announces). Check
  SVG fills the box only when on; busy = spinner (reduced-motion guarded).
  Removed dead .onboard-state/.onboard-toggle/.onboard-layer-top markup+CSS.
  New `toggleSemantic` dispatcher routes to the SAME 3 calls (on->disable;
  off+needsModel->downloadAndEnable; off->enable) so download consent is
  preserved (no auto-download on a stray click); model-missing shows a small
  "downloads ~63 MB" aside. Reports -> toggleReports unchanged.
- Gate: make web-check exit 0 (svelte-check 0 err, vitest 1670/1670, build
  clean; no new a11y/unused-css warnings).
- BROWSER-SMOKED (fresh served workspace, private port 4731, renamed binary,
  torn down): onboard card renders the 2 checkmark rows; Reports round-trips
  off->on (aria-checked + check SVG track the API result, confirming runtime
  reactivity); Semantic enable verified (model present); JS-confirmed both are
  BUTTON role=checkbox + aria-checked + aria-label (keyboard-native). Server +
  tab torn down clean.
- Files: web/src/components/PreflightOverlay.svelte (single file). Reported to
  @@LaneA.
