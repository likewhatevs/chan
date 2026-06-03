# task-LaneB-LaneA-2: BUILD launcher redesign (MODAL) - COMPLETE

From: @@LaneB  To: @@LaneA (lead)  Type: build completion

## What landed (desktop/src/* only; no Rust, no new files)

All locked decisions built (D1 keep add-time toggles, D2 modal, D3
connection dot, D4 drop tagline):

- index.html: header = enso + "Workspaces" + single [New] + theme toggle.
  Removed #open-workspace, #tunnel-btn, #tunnel-panel-slot, .brand-tagline.
- main.js: showNewWorkspaceDialog() - an overlay (modeled on
  showPreflightDialog) with a 3-button segmented switch (local | outbound |
  inbound) modeled on TeamDialog's real-estate toggle; clicking swaps the
  body + footer. ESC / backdrop / [X] dismiss; dismiss NEVER calls
  tunnel_stop (a live inbound listener keeps running).
  - Local: Choose folder -> open() picker -> in-body preflight scan
    (compute_workspace_preflight + renderPreflightReport) + the two
    add-time toggles -> add_workspace -> refresh + close.
  - Outbound: URL + Name -> add_outbound_workspace -> refresh + close.
  - Inbound: tunnel_status -> port form (tunnel_start) and listening state
    (Local|Tunnel seg + snippets click-to-copy + Stop/Done).
  Deleted the inline tunnel panel (toggleTunnelPanel, renderTunnelPanel,
  renderTunnelPanelHtml, renderOutboundAttachForm, bindTunnelPanelEvents)
  and the gear (renderFeaturesToggle/Panel, bindFeaturesToggle,
  loadFeaturesInto, collectFeaturesFromPanel) + cssEscape. Removed the JS
  invokes of get/set_workspace_features.
  Rows -> ON | WHERE: one renderWhere(d) + new inline-SVG ic-outbound /
  ic-inbound glyphs; remote ON cell = a static connection dot (green when
  d.url present), url/tunnel text tags dropped; thead "Path" -> "Where".
  Empty-state #empty-pick + boot() first-run -> showNewWorkspaceDialog('local').
- styles.css: added .nw-* (clones .preflight-overlay/-dialog +
  .team-realestate-toggle/-mode), .conn-dot, .where-cell/.where-dir;
  rehomed .seg-toggle/.snippet under the modal; removed .brand-tagline,
  .tag*, .tunnel-panel*, .features-*.

## Build status: GREEN

- `cd desktop && make build` -> exit 0; built Chan.app + Chan_0.24.0_
  aarch64.dmg. Re-built green on final source.
- node --check main.js: syntax OK. styles.css braces balanced (113/113).
  Grep audit: no leftover refs to removed symbols.
- desktop/src is loaded directly (frontendDist="../src"), not compiled, so
  no JS gate exists; correctness past syntax needs the app smoke.

## For the smoke (@@LaneD / @@Alex - WKWebView, not Chrome-automatable)

Walk: header shows one [New]; New -> Local (Choose folder, scan fills,
toggles, Open registers, row appears with home/computer icon); New ->
Outbound (URL+Name, Attach, outbound row w/ outbound icon + "outbound"
caption); New -> Inbound (Start listening shows port+snippet, copy works,
Local|Tunnel switches snippet, Stop returns to form, Done closes w/o
stopping listener); rows ON|WHERE, no gear, reveal-in-Finder + Open split +
Forget work; remote ON dot green when connected; ESC/backdrop/[X] dismiss;
empty + first-run open the modal on Local.

## CROSS-LANE FLAG for @@LaneC (do not let CI red)

My build surfaced a dead_code warning: `method live_workspace is never
used` (desktop/src-tauri/src/embedded.rs:77). It is now orphaned - the only
caller was resolve_workspace_for_features, which @@LaneC is deleting along
with set_workspace_features. `-D warnings` (pre-push / CI) WILL fail on it,
so @@LaneC must also delete live_workspace from embedded.rs (and check
host.live_workspace upstream if it too goes unused). I did NOT touch
src-tauri (C's lane); flagging per the task's coordination note.

## Housekeeping

Did not commit (per task; you commit after verify). Tree has only the
6 expected modified files: desktop/src/{index.html,main.js,styles.css}
(mine) + desktop/src-tauri/{permissions/app.toml,src/main.rs,src/serve.rs}
(@@LaneC's). Journal: new-team-1/journals/journal-LaneB.md.

---

## [LaneA] task-LaneB-LaneA-2 ACCEPTED (frontend done, make build green)

All locked decisions built; make build green. Accepted.

Your live_workspace cross-lane flag is ALREADY RESOLVED: @@LaneC deleted
EmbeddedServer::live_workspace during its dead-code cascade. Verified by
@@LaneA against the live tree: embedded.rs is modified (7 files total, not
6 - your snapshot predated C's embedded.rs edit) and grep finds no
live_workspace remaining. clippy -D warnings is clean. No action for you.

You are done for this round. HOLD; @@LaneA may re-poke if @@Alex's smoke
turns up a frontend fix. Do NOT commit (lead commits after verify).
