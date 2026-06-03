# chan-desktop launcher redesign - design

## >>> DECISIONS LOCKED (@@Alex via @@LaneA, 2026-06-02) - BUILD READS THIS FIRST

The build follows these locked calls. Where §4/§5/§6 below frame the WINDOW
as recommended, that is the REJECTED alternative - keep it for context, but
BUILD THE MODAL PATH.

- D2 = MODAL (in-launcher overlay). Build §4.1 "MODAL FALLBACK" + the modal
  interaction/choice spec. Do NOT create new.html / new.js /
  launcher-common.js, and do NOT add open_new_workspace_window or its
  capability/permission. The 3-choice body is showNewWorkspaceDialog() in
  main.js, an overlay like showPreflightDialog (ESC / backdrop / [X]
  dismiss; dismiss NEVER stops a live inbound listener). Files touched:
  desktop/src/{index.html,main.js,styles.css} only.
- D3 = CONNECTION DOT. Remote (URL) rows show a static dot in ON (green when
  d.url present/connected, grey otherwise); drop the url/tunnel text tags;
  the inbound/outbound direction lives on the WHERE-column icon.
- D4 = DROP THE TAGLINE. Remove the italic "what are we working on today?"
  (index.html:17) + its `.brand-tagline` CSS. Header = enso + "Workspaces"
  + [New] + theme toggle.
- D1 = KEEP add-time toggles (Semantic search + Reports) in the Local
  choice. Architect call (@@LaneA) on @@LaneB's rationale: creation-time
  selection is load-bearing (avoids a wasteful re-index); distinct from the
  removed ongoing gear. @@Alex informed; may veto.
- ESC-on-Cmd+P bug: ALREADY FIXED (6100ec84). VERIFY only, no new code, SPA
  not launcher - out of scope for the launcher build.

## <<< end locked decisions

Owner: @@LaneB. Status: FINAL (design-first; awaiting @@Alex sign-off).
Source of truth for the build. Grounded in the current code, not the draft
alone. @@LaneC's inventory and @@LaneD's gap finding are folded in (see
section 7); both resolved the gating questions.

Inputs:
- draft + screenshots: docs/journals/phase-16/desktop-redesign-draft/
  draft.md, image.png (launcher), image-1.png (folder picker),
  image-2.png (Attach: Open-by-URL outbound + Receive-remote inbound).
- new-team-1/launcher-inventory-LaneC.md  (current-code map, file:line).
- new-team-1/spa-settings-gap-LaneD.md     (gear-removal gap: NO GAP).

The launcher front-end is desktop/src/* (index.html, main.js, styles.css),
loaded directly by Tauri (tauri.conf.json build.frontendDist = "../src").
It is plain vanilla JS: no bundler, no npm build, no JS test harness.
Editing it needs only a Tauri rebuild, NOT a web/ npm build. The chan SPA
(web/) is a separate surface, embedded via rust-embed and served to the
per-workspace windows; this redesign does not touch it.


## 0. What changes, in one screen

Today                              After
-----                              -----
Header: [Open workspace][Attach]   Header: [New]  (single button)
        [theme]                            [theme]
Rows:   ON | PATH | actions        Rows:   ON | WHERE | actions
        per-row gear (bge/reports)         no gear (toggles live in SPA)
Attach: inline panel toggled in    [New] opens a NEW WINDOW with 3 choices,
        the launcher (outbound      each a different body: Local directory /
        URL form + inbound          Remote outbound / Remote inbound, using
        listen form)                a Team-Work-style segmented switch

The three add-flows are unchanged underneath; only their entry surface
changes. Local picks a folder + preflight + add_workspace (main.rs:234).
Outbound dials a URL via add_outbound_workspace (main.rs:1048). Inbound
binds a loopback listener via tunnel_start (main.rs:946). All three already
work from any main-* window's permission set, so the new window needs no
new capability file (C section 5).


## 1. Terminology (pin this; INBOUND vs OUTBOUND is load-bearing)

The draft requires a clear INBOUND vs OUTBOUND indication for URL
workspaces. Map exactly to the code so the build cannot invert it. The data
already exists: list_workspaces tags each row with `kind` (C section 3).

+-----------+------------------+-------------------+--------------------+
| choice    | code kind        | direction         | command            |
+-----------+------------------+-------------------+--------------------+
| Local     | local (data-path)| n/a (on disk)     | add_workspace      |
|           |                  |                   | (main.rs:234)      |
| Outbound  | outbound         | WE CONNECT OUT to | add_outbound_      |
|           | (main.rs:200)    | a remote URL      | workspace (1048)   |
| Inbound   | tunneled         | WE LISTEN for an  | tunnel_start (946) |
|           | (main.rs:185)    | incoming connect  | + remote dials in  |
+-----------+------------------+-------------------+--------------------+

Outbound = "Open by URL" (image-2 top form). Inbound = "Receive a remote
workspace" (image-2 bottom form). Today outbound rows show a blue "url"
tag and inbound rows a green "tunnel" tag in the ON column; there is NO
directional icon yet (C section 3).


## 2. Header redesign

### Markup

index.html `.actions` today (C: index.html:20-21):

    <button class="btn primary" id="open-workspace">Open workspace</button>
    <button class="btn primary" id="tunnel-btn" ...>Attach</button>
    <button class="btn" id="auth-btn" ... hidden>Sign in</button>
    <button class="btn icon" id="theme-toggle" ...>...sun/moon svg...</button>

After:

    <button class="btn primary" id="new-workspace">New</button>
    <button class="btn" id="auth-btn" ... hidden>Sign in</button>
    <button class="btn icon" id="theme-toggle" ...>...sun/moon svg...</button>

- Drop `#open-workspace` and `#tunnel-btn`; add a single `#new-workspace`
  ("New"). Keep the hidden `#auth-btn` and the `#theme-toggle` as-is.
- Remove `<div id="tunnel-panel-slot"></div>` (index.html:38): the inline
  tunnel panel moves into the new window's Inbound/Outbound choices.
- `#new-workspace` click -> invoke('open_new_workspace_window') (section 4).
- Brand block unchanged. The italic tagline "what are we working on
  today?" stays (see decision D4 if @@Alex wants it dropped per the
  draft's header sketch).

### CSS

No new header CSS. `.actions { display:flex; gap:6px }` already lays out a
single primary button + the icon toggle.


## 3. Row redesign: ON | WHERE

### Columns and header

table header th today: `On | Path | (blank, 150px)` (C: main.js:759-769).
After: `On | Where | (blank, 150px)`. Change the second `<th>` text from
"Path" to "Where". Column widths unchanged.

### The WHERE cell: one renderer, icon by kind

Today three row variants render their second cell differently (renderPath
for local at main.js:748; a `.remote-cell` label for outbound at 727; a
muted label for tunneled at 712). Unify into one `renderWhere(d)` that
emits a leading directional glyph + the path-or-URL:

    local:    [home|computer glyph] /rest-of-path     (= today's renderPath)
    outbound: [outbound glyph] <label or url>  + muted "outbound" caption
    inbound:  [inbound glyph]  <label>         + muted "inbound" caption

Keep the local cell clickable (data-act="reveal" -> reveal_in_finder,
main.rs:1363). Remote cells stay non-interactive text (today's
`.remote-cell`), tooltip = the full URL.

### Icon set (4 glyphs, inline SVG, 13x13, stroke=currentColor)

Match the existing ic-home / ic-computer style (viewBox 0 0 24 24, fill
none, stroke-width 1.8, main.js:130/139). Two exist, two are new:

+----------+--------+------------------------------------------------+
| glyph    | status | meaning                                        |
+----------+--------+------------------------------------------------+
| ic-home  | exists | local workspace under $HOME                     |
| ic-      | exists | local workspace outside $HOME                  |
|  computer|        |                                                |
| ic-      | NEW    | outbound: we connect OUT (arrow leaving a box, |
|  outbound|        | external-link style)                          |
| ic-      | NEW    | inbound: we listen / receive (arrow arriving   |
|  inbound |        | into a tray, download-to-inbox style)         |
+----------+--------+------------------------------------------------+

Suggested paths (the build may refine the art; the semantics out=leaving,
in=arriving MUST hold and the two must be visually distinct):

    ic-outbound: <path d="M14 4h6v6"/><path d="M20 4l-9 9"/>
                 <path d="M19 13v6a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V7a1 1 0
                          0 1 1-1h6"/>
    ic-inbound:  <path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/>
                 <path d="M12 4v11"/><path d="M7 10l5 5 5-5"/>

CSS: add `.where-cell .ic-outbound, .where-cell .ic-inbound` mirroring the
existing `.path-cell .ic-home, .path-cell .ic-computer` rules. The
"inbound"/"outbound" caption is a small `.where-dir` muted span.

### The ON cell for remote rows

Local rows keep the on/off `.switch` (main.js:743-746, set_workspace_on at
main.rs:401). Remote rows have no desktop-side on/off (the remote owns the
lifecycle), so today they put a text tag in the ON cell. Proposal: replace
the text tag with a static connection dot (green when `d.url` is present =
connected/running, grey otherwise); the direction now lives in WHERE. This
keeps the ON column uniform and "more uniform with URLs" per the draft. The
old `.tag-tunnel` / `.tag-outbound` text spans are dropped. (Decision D3:
confirm dot vs keep a badge.)

### Gear removal  [RESOLVED by @@LaneD: no gap, safe to remove]

@@LaneD confirmed NO GAP: the gear toggles exactly bge (semantic search) +
reports, and BOTH are configurable in the chan SPA Dashboard today
(SearchSlotConfig.svelte:287 -> POST /api/index/semantic/{enable,disable};
WorkspaceSlotConfig.svelte:38 -> POST /api/index/reports/{enable,disable};
both mounted at DashboardSlotBack.svelte:66-68). Removing the gear strands
nothing.

Remove the per-row gear:
- render(): drop the `renderFeaturesToggle(...)` call from the local row's
  row-actions (main.js:751) and drop the trailing
  `renderFeaturesPanel(d.path)` sibling row (main.js:756).
- main.js: delete renderFeaturesToggle (780), renderFeaturesPanel (802),
  bindFeaturesToggle (978), loadFeaturesInto (1021),
  collectFeaturesFromPanel (1037), and the bindFeaturesToggle(tr) call in
  bindRowEvents (966).
- styles.css: delete the `.features-toggle`, `.features-panel`,
  `.features-content/-copy/-row/-label/-hint` rules.
- Rust: get_workspace_features (main.rs:438) + set_workspace_features
  (761) become unused (C: gear-only). Delete them + their permissions/
  app.toml entries (allow-get-workspace-features,
  allow-set-workspace-features). add_workspace still takes `features` (the
  add-time toggles, decision D1), so its path is untouched.

NUANCE to note in the build (from @@LaneD): the gear could toggle these
WITHOUT opening the workspace (transient open, main.rs:504); the SPA
toggles act only on the OPEN workspace. So out-of-workspace bulk toggling
goes away. For a local-first single-user app where you open a workspace to
work in it, this is a minor UX shift, not a stranded setting. Optional
polish: a one-line pointer in the launcher / new window ("feature toggles
live in the workspace Dashboard"). Not required.


## 4. The [New] surface: a NEW WINDOW (recommended)

### 4.1 Window vs modal  [recommendation: WINDOW; modal is the fallback]

RECOMMENDATION: a real second Tauri window, NOT an in-launcher modal.

Why this flipped from the v1 draft's modal lean: the v1 blocker was
capability cost. @@LaneC confirmed there is none. A window built in the
`main-*` label space inherits default.json's capability automatically
(dialog:allow-open for the folder picker + every add command); no new
capability file (C section 5, default.json:5). With that gone, the
decision turns on intent and fit:

1. The draft explicitly says the [New] button "opens a new window". The
   machinery already supports it cheaply, so honor the stated intent.
2. The 3-choice body is content-rich (Local carries a folder scan + warns;
   Inbound carries snippets + a Stop control). A window gives it room and
   keeps the workspace list visible behind it rather than dimmed under a
   backdrop.
3. "Resemble the Team Work one" is about the INTERACTION (a segmented
   switch that swaps the layout per choice), which ports identically into a
   window or a modal. A window does not lose it.

Machinery to reuse (C section 5):
- open_new_launcher_window (main.rs:1947) is the template: WebviewWindow-
  Builder::new(app, &label, WebviewUrl::App("new.html")).title("New
  workspace").inner_size(...).build(). Add a sibling command
  open_new_workspace_window that points at a new `new.html` page.
- Label: a SINGLETON `main-new` (matches the `main-*` capability glob, so
  no capability edit; not a numeric `main-N` so it never collides with
  next_launcher_label, main.rs:2012). Dedup: if `main-new` already exists,
  focus it via the show_window path (main.rs:1382) instead of building a
  second one.
- No `?w=` per-window state machinery is needed (that is for workspace
  webviews restoring panes/zoom, serve.rs:340); the New window is
  ephemeral.

MODAL FALLBACK: if @@Alex prefers the lighter surface, the same 3-choice
body becomes an in-launcher overlay (the existing pattern: preflight /
default-workspace dialogs, main.js:199/271/440). That path needs ZERO Rust
and no new HTML (everything lands in main.js + styles.css). It is the
simpler build; its only cost is covering the list with a backdrop and
diverging from the draft's "window" wording. See decision D2.

Everything below assumes the window. The "collapses to modal" deltas are
noted in section 5.

### 4.2 Interaction model (mirror Team Work)

The New window's page (new.js) renders a Team-Work-style segmented switch.
Model it on TeamDialog.svelte's "real estate" toggle (setRealEstate ->
switchRealEstate -> body swap, TeamDialog.svelte:219-221, 519-598).

State: one variable `choice` in {'local','outbound','inbound'}. A segmented
toggle at the top (`.nw-choices`, role=radiogroup) has three buttons;
clicking one sets `choice`, flips the `.on` class, and re-renders the body
(`.nw-body`). Each choice shows a different layout and a different footer
action.

Layout (ASCII):

    +--------------------------------------------------------+
    |  New workspace                                         |   (window
    +--------------------------------------------------------+    title bar)
    | [ Local directory | Remote outbound | Remote inbound ] |  <- .nw-choices
    +--------------------------------------------------------+
    |                                                        |
    |   <body swaps per choice; see 4.3 / 4.4 / 4.5>         |  <- .nw-body
    |                                                        |
    +--------------------------------------------------------+
    |                          <choice-specific footer btns> |  <- .nw-footer
    +--------------------------------------------------------+

Focus the active choice's first input (or the Choose-folder button) on
load. After a successful add, the window closes itself (4.6); the parent
launcher refreshes automatically via the registry-changed / serves-changed
events it already listens for (main.js:1161-1162), so the new row appears
with no cross-window message.

### 4.3 Choice: Local directory

Folds today's pickAndAdd two-step (native picker -> separate preflight
overlay) into one body.

- Initial: intro "A local folder with your markdown files (a git repo or
  any directory)." + a "[ Choose folder... ]" button.
- Choose folder -> open({ directory:true, multiple:false, title:'Select a
  folder containing markdown files' }) (the tauri-plugin-dialog picker,
  works from main-*). On cancel, stay.
- After a folder is chosen, render IN-BODY (reuse the existing helpers):
  - the chosen path (mono).
  - the scan report: compute_workspace_preflight(path) (main.rs:707) ->
    renderPreflightReport (main.js:600): files / markdown / size / media /
    scm + the already-registered + read-only warnings. KEEP this; it is
    the "confirm this is the folder I meant" surface.
  - the two feature toggles (Semantic search, Reports), reusing the
    `.preflight-toggle` markup + copy (main.js:489-518). KEEP per decision
    D1 (creation-time selection, distinct from the removed ongoing gear).
  - footer: [ Back ] (re-show Choose folder) and [ Open ] -> add_workspace
    { path, features:{bge,reports} } (main.rs:234) -> close window.

### 4.4 Choice: Remote outbound ("we connect to a URL")

Reuse renderOutboundAttachForm's fields (main.js:1279), rehomed:
- intro: "Connect to a chan workspace already served at a URL (we dial out
  to it)."
- URL field (placeholder http://127.0.0.1:4000/?t=...) + Name field.
- footer: [ Attach URL ] -> add_outbound_workspace { url, label }
  (main.rs:1048) -> close window. Enter in either field submits. Required-
  URL validation reuses attachOutboundUrl's logic (main.js:1367).

### 4.5 Choice: Remote inbound ("we listen for an incoming connection")

Reuse the inline tunnel panel's two states (renderTunnelPanelHtml,
main.js:1226-1276), rehomed. On entering this choice, read tunnel_status
(main.rs:917).

- NOT listening:
  - intro: "Bind a loopback port to accept an incoming `chan serve
    --tunnel-url` from another machine over an SSH reverse forward (we
    listen)."
  - Port (placeholder auto) + Label + Workspace fields + the helper line.
  - footer: [ Start listening ] -> tunnel_start { preferredPort, label,
    workspace } (main.rs:946) -> transition the body to the listening
    state in place.
- Listening (after start, or if already listening on open):
  - "Listening on 127.0.0.1:<port>"
  - the Local | Tunnel seg toggle (which snippet shows; localStorage
    chanDesktopTunnelMode, main.js:1217), reusing `.seg-toggle`.
  - the snippet block(s) (ssh -R when Tunnel + the chan serve command),
    click-to-copy (main.js:1353).
  - "Connected workspaces appear in the launcher window and open
    automatically."
  - footer: [ Stop ] -> tunnel_stop (main.rs:980) -> back to the form;
    [ Done ] -> close the window (listener keeps running).

Closing the New window NEVER stops a live listener: the listener lives in
Rust AppState, not the window; tunnel_status is the source of truth, so
reopening [New] -> Inbound shows it still listening (matches today, where
hiding the Attach panel leaves the tunnel running). Connected tunneled
rows appear in the launcher via tunneled-workspace-ready / serves-changed
(main.js:1394/1162).

This deletes the inline panel entirely (toggleTunnelPanel,
renderTunnelPanel, renderTunnelPanelHtml, renderOutboundAttachForm,
bindTunnelPanelEvents); the logic moves into new.js.

### 4.6 Window lifecycle: dedup + close

- Dedup: open_new_workspace_window checks for an existing `main-new`
  window and focuses it (show_window pattern, main.rs:1382) instead of
  building a duplicate.
- Self-close on success: new.js closes its own window via
  getCurrentWindow().close() after a successful Local Open / Outbound
  Attach. If core:default does not already grant core:window:allow-close,
  add it to capabilities/default.json (one-line edit to the EXISTING file,
  not a new capability file).

### 4.7 Empty state + first run

- render()'s empty-state "Open workspace" button (#empty-pick,
  main.js:683) -> open_new_workspace_window (lands on the Local choice).
- boot()'s first-run auto-open (currently pickAndAdd when zero workspaces,
  main.js:147-149) -> open_new_workspace_window. Same destination, now the
  unified New window.

### 4.8 CSS + shared code

- Extract the truly-shared atoms into a `launcher-common.js` loaded by BOTH
  index.html and new.html (plain `<script src>`, shared global scope under
  withGlobalTauri): theme apply/toggle (main.js:23-49), escapeHtml/
  escapeAttr (1119-1125), showError (1109), the preflight render helpers
  (600-671), and the tunnel/outbound form renderers + snippet-copy
  (1226-1365). The 3-choice form orchestration lives in new.js.
- styles.css: add a `.nw-*` block. Reuse heavily: clone
  `.team-realestate-toggle` / `.team-realestate-mode` for `.nw-choices`;
  reuse `.preflight-*` (toggles, report rows, path), `.tunnel-row` inputs,
  `.seg-toggle`, `.snippet` inside the bodies. Remove the now-dead
  `.features-*` (gear) rules. Keep styles.css single-owner (one file).


## 5. Implementation plan

### Files changed (window approach)

+----------------------------+---------------------------------------------+
| file                       | change                                      |
+----------------------------+---------------------------------------------+
| desktop/src/index.html     | header: drop Open workspace + Attach, add   |
|                            | [New] (-> open_new_workspace_window); remove|
|                            | tunnel-panel-slot; load launcher-common.js  |
| desktop/src/new.html       | NEW: the New-workspace window page; loads    |
|                            | launcher-common.js + new.js + styles.css    |
| desktop/src/new.js         | NEW: 3-choice form (segmented switch; Local |
|                            | picker+scan+toggles; Outbound; Inbound +    |
|                            | listening); add_* + self-close              |
| desktop/src/launcher-      | NEW: shared atoms (theme, escapes, showError,|
|  common.js                 | preflight render, tunnel/outbound forms)    |
| desktop/src/main.js        | header handler -> invoke command; row redesign|
|                            | ON|WHERE + renderWhere + icons; remove gear |
|                            | (5 fns); remove inline tunnel panel (moved  |
|                            | to new.js/common); thead Path->Where; empty |
|                            | + first-run reroute                         |
| desktop/src/styles.css     | add .nw-*; add ic-outbound/ic-inbound;      |
|                            | remove .features-* gear CSS                  |
| desktop/src-tauri/src/     | NEW command open_new_workspace_window (sibling|
|  main.rs                   | of open_new_launcher_window) + register in  |
|                            | generate_handler (1781); delete get/set_    |
|                            | workspace_features (438/761)                |
| desktop/src-tauri/         | add allow-open-new-workspace-window to the  |
|  permissions/app.toml      | main-window set; remove allow-get/set-      |
|                            | workspace-features                          |
| desktop/src-tauri/         | only if needed: add core:window:allow-close |
|  capabilities/default.json | for the New window self-close (one line)    |
+----------------------------+---------------------------------------------+

Collapses-to-modal delta (if @@Alex picks the fallback, D2): drop new.html
+ new.js + launcher-common.js + the open_new_workspace_window command +
its app.toml permission + the default.json close line. The 3-choice body
becomes showNewWorkspaceDialog() in main.js (overlay like showPreflight-
Dialog). Rust then only loses the two feature commands (gear removal).

### Build + browser-smoke test plan

- Build: `cd desktop && make dev` (or `make build`). Launcher edits are
  under frontendDist="../src", so NO web/ npm build is needed; a Tauri
  rebuild picks them up.
- Static gate (Rust): `cargo fmt --check`, `cargo clippy --all-targets -D
  warnings`, `cargo build -p chan-desktop`. (The window approach adds a
  command + deletes two; the modal approach only deletes two.)
- No JS gate exists: desktop/src is plain vanilla JS, no bundler/eslint/
  test. Launcher correctness is app-smoke only.
- Smoke MUST be the real app, not Chrome MCP: chan-desktop renders in
  WKWebView (macOS), which Chrome automation (Blink) cannot reach. A human
  or a Mac-resident lane drives. Walk:
  1. Header shows a single [New]; theme toggle still flips; the New window
     matches the launcher theme.
  2. [New] opens the window; clicking [New] again focuses it (no second
     window).
  3. Local: Choose folder, scan rows fill, toggles default off, Open
     registers, the window closes, a new ON|WHERE row appears with the
     home/computer icon.
  4. Outbound: enter URL + name, Attach URL, window closes, an outbound
     row appears with the outbound icon + "outbound" caption.
  5. Inbound: Start listening shows port + snippet; copy works; Local|
     Tunnel switches the snippet; Stop returns to the form; Done closes
     the window WITHOUT stopping the listener (reopen -> still listening).
  6. Rows: ON|WHERE header; no per-row gear; reveal-in-Finder works; Open
     split + Open-in-Browser + Forget work.
  7. Empty state + first run open the New window on Local.

### Proposed lane split for the build

- Frontend-Lane (single owner of desktop/src/*): index.html, new.html,
  new.js, launcher-common.js, main.js, styles.css. One owner because the
  files are tightly coupled (shared CSS; the tunnel/outbound logic MOVES
  from main.js into new.js/common) and main.js is one ~1400-line file;
  splitting it across concurrent lanes collides (a half-applied markup or
  shared-CSS change breaks the other lane's smoke).
- Rust-Lane (parallel, src-tauri only, disjoint files): the
  open_new_workspace_window command + register + app.toml permission;
  delete get/set_workspace_features + their perms; the optional
  default.json close line. Joined to the Frontend-Lane by a 3-line naming
  contract: command `open_new_workspace_window`, page `new.html`, label
  `main-new`.
- Verify-Lane (sequenced AFTER both land): macOS chan-desktop app smoke per
  the plan above. Not Chrome-automatable (WKWebView); a Mac lane or @@Alex
  drives.

(Modal fallback collapses this to one Frontend-Lane + a tiny Rust-Lane
that only deletes the two feature commands.)


## 6. Decisions for @@Alex (design review)

D1 (add-time feature toggles) [I decided; C assigned me this]: the gear is
removed, but I recommend KEEPING the two toggles in the Local choice's
add-time flow. Rationale: the gear was ONGOING reconfiguration of existing
workspaces from the launcher (that belongs in the SPA, removed); the
pre-flight pair is CREATION-TIME initial selection, a different concept,
and it is load-bearing - chan-workspace boot reads the choice on first
scan (register_and_boot, main.rs:308-327), so dropping it forces a wasteful
re-index for anyone who wants semantic search on a new workspace. Tradeoff
if you want a strictly "no feature controls anywhere in the launcher"
surface: drop them too; new workspaces then open with graph+BM25 only and
you enable semantic/reports in the SPA after opening (one extra index).

D2 (window vs modal): recommending a real NEW WINDOW (honors the draft's
"open a new window"; the machinery supports it with no new capability
file). The in-launcher MODAL is the lighter fallback (zero Rust, no new
HTML, but covers the list and diverges from the draft wording). Confirm
window, or pick the modal.

D3 (remote ON cell): local rows keep the on/off switch; remote rows have no
desktop-side on/off. Proposal: a static connection dot (green when
connected) + the type signal moves to the WHERE icon, dropping the
"url"/"tunnel" text tags. OK, or keep a text badge in ON?

D4 (tagline): keep the italic "what are we working on today?" tagline in
the header? The draft's header sketch omits it. Minor.

Heads-up (not a decision): the draft's "ESC should cancel the Cmd+P Team
Work dialog" bug appears ALREADY FIXED - a capture-phase Escape handler
landed 2026-06-01 (commit 6100ec84, TeamDialog.svelte:274-289). It is an
SPA concern (web/), not a launcher file (C cross-cutting note). Recommend a
quick verify rather than a new fix; out of scope for this redesign.


## 7. Peer recon folded in

@@LaneC (launcher-inventory-LaneC.md): confirmed the current-code map I
read independently - the three add-flows + their commands (sections 1-2),
row rendering + the kind field (section 3), the gear = exactly bge+reports
via get/set_workspace_features (section 4), and the window machinery: a
real second window is supported, a `main-N`/`main-*` window inherits the
default capability, no new capability file (section 5). C also flagged the
add-time-toggle consistency call -> resolved as D1.

@@LaneD (spa-settings-gap-LaneD.md): NO GAP. Both gear settings live in the
SPA Dashboard today (Search slot + Workspace slot, with their /api/index/
{semantic,reports}/{enable,disable} routes). Gear removal strands nothing;
the only shift is losing out-of-workspace bulk toggling (noted in section
3). No new SPA surface required.

Both findings are integrated above; no open dependency remains. The only
open items are the @@Alex design decisions in section 6.
