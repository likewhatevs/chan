# launcher-inventory-LaneC: chan-desktop launcher, CURRENT code

From: @@LaneC  To: @@LaneA (for @@LaneB design grounding)
Type: recon (read-only). No code changed.

Scope read: desktop/src/index.html, desktop/src/main.js (1399 lines),
desktop/src/styles.css (660 lines), desktop/src-tauri/src/main.rs (2206
lines), serve.rs (1309), tunnel/*, config.rs, plus tauri.conf.json +
capabilities/. Line refs are file:line at the HEAD read on 2026-06-02.

The launcher shell is index.html: a `header.bar` with brand + actions
(buttons #open-workspace, #tunnel-btn, hidden #auth-btn, #theme-toggle),
an empty `#tunnel-panel-slot` (index.html:38), and `<main id="main">`
(index.html:39) where the table renders. All IPC is `invoke(...)` from
main.js; every command resolves to a `#[tauri::command]` registered in
the `generate_handler!` block at main.rs:1781-1821.


## 1. [Open workspace] flow (pickAndAdd)

Button: index.html:20 `<button id="open-workspace">Open workspace</button>`.
Bound at main.js:1127 `openBtn.addEventListener('click', pickAndAdd)`.
Empty-state duplicate button `#empty-pick` -> pickAndAdd (main.js:683-685).
`boot()` also auto-calls pickAndAdd when zero workspaces exist (main.js:147-149).

pickAndAdd (main.js:402-427):
  1. Native folder picker: `open({directory:true, multiple:false,
     title:'Select a folder containing markdown files'})` (main.js:403-407).
     This is the tauri-plugin-dialog `open` imported at main.js:2
     (`window.__TAURI__.dialog`), NOT a custom Rust command. Permission
     `dialog:allow-open` in capabilities/default.json:10. Returns the
     chosen dir as a string, or non-string on cancel (early return 408).
  2. Pre-flight modal: `showPreflightDialog(selected)` (main.js:415, def
     440-594). In-launcher overlay (`.preflight-overlay`), NOT a window.
     Shows path, a live report, baseline/optional explanatory copy, two
     toggles, Cancel/Open. Escape + backdrop cancel; Open has focus.
     - Report rows filled by `invoke('compute_workspace_preflight',{path})`
       (main.js:586) -> renderPreflightReport (600). Rust at main.rs:707;
       PreflightReport struct main.rs:538-579 (file_count, markdown_count,
       size_bytes, image/audio/video, scm, already_registered, writable,
       truncated). Walk capped 100k files / 5s (main.rs:581-582).
     - Two checkboxes default OFF: bge "Semantic search" (main.js:492-504),
       reports "Reports" (506-518). Returned as `{accepted, features:
       {bge, reports}}`.
  3. Register: `invoke('add_workspace',{path, features})` (main.js:418).
  4. `refresh()` (main.js:426) re-reads list_workspaces and re-renders.

Rust add_workspace (main.rs:234-284):
  - canonical_key(path) (241); default features if absent (242).
  - Routes through the SINGLE embedded chan-server Library (251); runs
    register_and_boot via spawn_blocking (258), bracketed by emit_chan_busy
    true/false (254/260) -> the "Adding workspace..." banner.
  - register_and_boot (main.rs:295-329): creates dir if missing (302),
    `library.register_workspace` (306); if bge|reports, open +
    set_semantic_enabled / set_reports_enabled + boot (308-327).
  - Mirrors chosen features into desktop store cache (270-275).
  - Auto-start: `serve::start` (282) mounts + spawns the first window.

No native folder picker lives in Rust; it is the dialog plugin.


## 2. [Attach] flow (toggleTunnelPanel)

Button: index.html:21 `<button id="tunnel-btn">Attach</button>`. Bound at
main.js:1388 -> toggleTunnelPanel (1184). Toggles `tunnelPanelOpen`,
then renderTunnelPanel (1189): invokes `tunnel_status` (1199, Rust
main.rs:917), writes renderTunnelPanelHtml into `#tunnel-panel-slot`
(1207), binds events (1208). Button text flips Attach <-> Hide (1206).

The panel ALWAYS renders TWO forms; the second has two states:

A. OUTBOUND "Open by URL" (renderOutboundAttachForm, main.js:1279-1294).
   Always at the top of the panel (1238, 1256). Fields: URL input
   (#outbound-url-input), Name input (#outbound-label-input), button
   "Attach URL" (data-act=outbound-add).
   - attachOutboundUrl (main.js:1367-1386): `invoke('add_outbound_workspace',
     {url, label})` (1378) -> clears inputs -> refresh. Enter in either
     field triggers the button (1327-1338).
   - Rust add_outbound_workspace (main.rs:1048-1090): normalize_outbound_url
     (strips a `w` query param, http/https + host required, 1137-1152),
     normalize_outbound_label (<=120 chars, 1170), pushes an
     OutboundWorkspace{uuid id, url, label, added_at} into store.outbound,
     then `serve::spawn_outbound_workspace_window` opens a webview NOW
     (1087) and emits SERVES_CHANGED (1088).

B. INBOUND "Receive a remote workspace" (state-dependent).
   - NOT listening (renderTunnelPanelHtml else branch, main.js:1255-1276):
     "Receive a remote workspace", explanatory copy, Port input
     (#tunnel-port-input, placeholder "auto"), Label (#tunnel-label-input),
     Workspace (#tunnel-workspace-input), button "Start listening"
     (data-act=tunnel-start). Handler main.js:1307-1323:
     `invoke('tunnel_start',{preferredPort, label, workspace})` (1316).
     Rust tunnel_start (main.rs:946-978): validates label
     (is_valid_username) + workspace (is_valid_workspace_name), persists
     preferred_* to store, `tunnel::start_listening` binds a loopback port.
   - Listening (main.js:1226-1254): header "Listening on 127.0.0.1:{port}",
     a Local|Tunnel segmented toggle (`.seg-toggle`, persisted in
     localStorage `chanDesktopTunnelMode`, pure UI, main.js:1217-1224,1300),
     ssh + chan_serve snippets (click-to-copy, 1353-1364), and a Stop
     button (data-act=tunnel-stop) -> `invoke('tunnel_stop')` (1342, Rust
     main.rs:980). Snippets come pre-formatted from tunnel_status.

How each becomes a workspace row (all via list_workspaces, main.rs:147):
  - INBOUND tunneled: `state.tunnel.snapshot()` -> kind:"tunneled" rows
    (main.rs:185-198). Event `tunneled-workspace-ready` (main.js:1394) and
    `tunnel-state-changed` (1396) refresh the table / panel.
  - OUTBOUND: `store.outbound` -> kind:"outbound" rows (main.rs:200-217).
    `serves-changed` (main.js:1162) refreshes.


## 3. Row rendering (render, main.js:673-772)

Empty state (main.js:677-687): `.empty` block + `#empty-pick` button.

Table (main.js:759-769): class `workspaces`, THREE columns:
  +--------+----------------------------+--------------------+
  | On     | Path                       | (actions, 150px)   |
  | 60px   |                            | blank <th>         |
  +--------+----------------------------+--------------------+

Rows are built by mapping workspaces (main.js:689-757); `d.kind` selects
one of three shapes:

  kind="tunneled" (main.js:692-718):
    - col1: `<span class="tag tag-tunnel" title="...">tunnel</span>`
      (main.js:711). title = peer addr / public / connected_at (698-702).
    - col2: `.path-cell muted` = the bearer label (712). No real path.
    - col3: renderOpenSplit{includeForget:false} (715). No On toggle,
      no reveal, no Forget (remote owns lifecycle).

  kind="outbound" (main.js:720-738):
    - col1: `<span class="tag tag-outbound" title="Attached URL">url</span>`
      (main.js:727).
    - col2: `.path-cell remote-cell` = label || url || "Remote workspace"
      (721, 727).
    - col3: renderOpenSplit{includeForget:true, forgetLabel:"Forget URL"}
      (730-736).

  kind="local" (default, main.js:740-756):
    - col1: ON/OFF toggle = `<label class="switch">` + checkbox
      `data-act="toggle-on"` + `.slider` (743-746). Change handler
      main.js:922 -> `invoke('set_workspace_on',{path,on})` (Rust
      main.rs:401 -> serve::start when on, serve::stop when off).
    - col2: `.path-cell data-act="reveal"` = renderPath(d.path) (748);
      click -> `invoke('reveal_in_finder',{path})` (main.js:943, Rust
      main.rs:1363, runs open/xdg-open/explorer).
    - col3 `.row-actions`: renderFeaturesToggle (the gear, see sec 4)
      THEN renderOpenSplit{includeForget:true} (751-752).
    - PLUS a sibling `renderFeaturesPanel(d.path)` row appended (756).

ICON logic (renderPath, main.js:125-141): if path is $HOME or under it,
render an inline house glyph `svg.ic-home` and trim $HOME (130-132);
otherwise an inline computer glyph `svg.ic-computer` + full path (139-140).
$HOME comes from `invoke('home_dir')` (main.js:108, Rust main.rs:1349).
NOTE for redesign: there is NO network/globe icon today; tunneled +
outbound rows use the text tags below, not an icon column.

URL / outbound tags: `tag-tunnel` (main.js:711) and `tag-outbound`
(main.js:727) are the only row tags; LOCAL rows carry no tag. Both are
plain `<span class="tag ...">` with a title tooltip; styled in styles.css.

Open split (renderOpenSplit, main.js:845-864): `.split-btn` = primary
"Open" (data-act=launch, disabled unless `hasUrl`) + caret
(data-act=menu-toggle) + `.split-menu` with "Open in Browser"
(data-act=open-browser, uses row `data-url` via opener plugin, main.js:1078)
and optional Forget item (data-act=remove). Local launch -> open_local_
workspace (main.rs:1203); tunneled -> open_tunneled_workspace (1311);
outbound -> open_outbound_workspace (1093).


## 4. Per-row gear (Settings / features)  [draft REMOVES this]

The gear toggles EXACTLY TWO features and nothing else: Semantic search
(bge) and Reports (reports). No rename, no theme, no per-row settings
beyond these two.

Markup:
  - renderFeaturesToggle (main.js:780-794): the `<button class="btn
    features-toggle" data-act="toggle-features">` with a gear SVG,
    title="Per-workspace feature toggles", aria-controls the panel.
    Rendered ONLY on local rows (main.js:751).
  - renderFeaturesPanel (main.js:802-837): sibling `<tr class=
    "features-panel" ... hidden>` colspan=3. Contains explanatory copy
    (`.features-copy`) + two checkboxes:
      * data-feat="bge" "Semantic search" (main.js:815) -- BGE-small
      * data-feat="reports" "Reports" (main.js:824) -- chan-report
    Both rendered with the `disabled` attribute; loadFeaturesInto clears
    `disabled` after the first IPC read (main.js:1027).

Behavior (bindFeaturesToggle, main.js:978-1015):
  - Gear click flips the panel's `hidden` attr (985-999). First open
    lazy-loads via `invoke('get_workspace_features',{path})` (main.js:1023,
    Rust main.rs:438; reads chan-workspace's authoritative state in-process,
    falls back to the desktop store cache).
  - Each checkbox change calls `invoke('set_workspace_features',{path,
    features:{bge,reports}})` (main.js:1007, Rust main.rs:761) with an
    optimistic update + revert-on-error (1009-1011).

There is ALSO a second surface for the same two toggles: the Open-workspace
pre-flight modal (sec 1, main.js:492-518). If the gear is removed but
add-time selection stays, that pre-flight pair is the remaining launcher
entry point; the draft says feature config moves into chan's SPA, so both
the gear (sec 4) and arguably the pre-flight toggles are the in-scope
removals. @@LaneB should decide whether add-time toggles also leave.


## 5. Tauri window machinery -- real second window IS supported

YES. The launcher already creates new top-level windows at runtime via
`WebviewWindowBuilder` (imported main.rs:24). An in-launcher modal is NOT
the only option; both patterns already exist in-tree.

New launcher (picker) window:
  - open_new_launcher_window (main.rs:1947-1963): `WebviewWindowBuilder::
    new(app,&label, WebviewUrl::App("index.html".into())).title("Chan
    Desktop").inner_size(960,600).min_inner_size(720,400).resizable(true)
    .build()`. Loads the SAME index.html as the singleton main, so a fresh
    `boot()` runs (a clean picker, no inherited state).
  - Label from next_launcher_label (main.rs:2012-2034): next free `main-N`
    (N>=2); singleton keeps the bare `main` label.
  - open_new_window_for_focused_workspace (main.rs:1980-2005): File > New
    Window opens another window of the focused LOCAL workspace, else falls
    back to open_new_launcher_window.

Workspace webviews (serve.rs):
  - build_workspace_window (serve.rs:328-420) is the shared builder. It
    APPENDS the `w=<window_label>` query param at serve.rs:340
    (`parsed.query_pairs_mut().append_pair("w", window_label)`), sets title,
    captures config on close (capture_window_config_on_close, 434).
  - Label conventions: `workspace-<hash16>-<seq>` (serve.rs:134,144),
    `tunnel-<hash16>-<seq>` (159,168), `outbound-<hash16>-<seq>` (177,184).
    is_workspace_webview_label matches workspace-/tunnel-/outbound- (189).
  - The `?w=<label>` machinery: config.rs WindowConfig.window_label
    (config.rs:146), a per-window session.json keyed by `w`; the desktop
    pops/pushes a config stack to restore url_hash + panes + zoom across
    close/reopen (serve.rs:195-220).

tauri.conf.json + capabilities:
  - Only the singleton `main` window is declared statically
    (tauri.conf.json:15-24: label "main", 960x600, url index.html). Every
    other window is runtime-built.
  - capabilities/default.json:5 scopes the default capability to
    `["main","main-*"]` and grants dialog:allow-open, opener, process,
    updater, etc. So a [New] window built as a `main-N` index.html window
    INHERITS the folder-picker + opener permissions automatically; no new
    capability file is needed if it reuses the `main-*` label space.
  - capabilities/workspace.json covers the workspace-* / tunnel-* /
    outbound-* webviews (zoom etc.).

Implication for the [New] 3-choice window (draft section 19-23):
  - It can be a REAL second window. Cheapest path: a `main-N` window
    (reusing the default capability), loading either index.html with a
    mode flag or a new dedicated HTML entry. The Team-Work-like
    tab/split-pane layout the draft wants is pure frontend inside that
    window.
  - The folder picker (dialog:allow-open) and add_workspace / tunnel_start
    / add_outbound_workspace commands already work from any `main-*`
    window, so the 3 choices map onto existing IPC: Local dir ->
    pickAndAdd/add_workspace; Outbound -> add_outbound_workspace; Inbound
    -> tunnel_start.
  - An in-launcher modal is equally feasible (the preflight + default-
    workspace dialogs at main.js:199,271,440 are the existing modal
    pattern). Choice between window vs modal is a design call, not a
    capability blocker.


## Cross-cutting notes for the redesign

- Draft "remove SETTINGS button": that is the per-row gear (sec 4),
  features-toggle + features-panel + bindFeaturesToggle, plus the
  get/set_workspace_features IPC. The pre-flight modal's bge/reports
  toggles (sec 1) are a SECOND instance of the same controls.
- Draft "INBOUND vs OUTBOUND indication": today only text tags exist
  (tag-tunnel = inbound listener, tag-outbound = "url" outbound). No
  directional icon. list_workspaces already distinguishes kind
  "tunneled" (inbound) vs "outbound" (main.rs:185 vs 200), so the data
  is present for an icon.
- Draft "merge [Open workspace]+[Attach] into one [New]": both header
  buttons (index.html:20-21) and their handlers (pickAndAdd,
  toggleTunnelPanel) collapse into one entry point that fans out to the
  three existing IPC paths above.
- ESC-on-Team-Work bug (draft tail) is an SPA concern (web/), not in the
  desktop launcher files; out of this inventory's scope.
