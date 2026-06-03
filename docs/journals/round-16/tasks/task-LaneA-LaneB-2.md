# task-LaneA-LaneB-2: BUILD the launcher redesign (MODAL path)

From: @@LaneA (lead)  To: @@LaneB  Type: build

@@Alex signed off the design. Build it. You designed it, so you own the
frontend build end to end.

## Source of truth

new-team-1/desktop-redesign-design.md - read the ">>> DECISIONS LOCKED"
block at the top FIRST. @@Alex chose the MODAL path (D2). The doc's §4/§5
frame the WINDOW as recommended; that is the REJECTED alternative. BUILD
THE MODAL: §4.1 "MODAL FALLBACK" + the modal interaction/choice spec, §2
header, §3 rows.

## Files - you are the single owner (do NOT touch src-tauri)

desktop/src/index.html, desktop/src/main.js, desktop/src/styles.css.
NO new files (no new.html / new.js / launcher-common.js). NO Rust.

## Build checklist (modal path + locked decisions)

1. Header (index.html): drop #open-workspace + #tunnel-btn, add ONE
   `<button class="btn primary" id="new-workspace">New</button>`. Remove
   `<div id="tunnel-panel-slot">`. DROP the `.brand-tagline` line
   (D4: index.html:17) + its CSS. Keep hidden #auth-btn + #theme-toggle.
2. [New] handler -> showNewWorkspaceDialog('local'): an overlay modal modeled
   on showPreflightDialog (main.js:199/271/440). Top = a 3-button segmented
   switch (local | outbound | inbound) modeled on TeamDialog's real-estate
   toggle; clicking swaps `.nw-body` + footer. ESC / backdrop / [X] dismiss.
   Dismiss NEVER calls tunnel_stop (a live inbound listener keeps running;
   tunnel_status is the source of truth).
   - Local (§4.3): Choose folder -> open({directory:true,...}) -> in-body
     preflight scan (compute_workspace_preflight -> renderPreflightReport)
     + the TWO add-time toggles (KEEP, D1) + [Back]/[Open] ->
     add_workspace{path, features:{bge,reports}} -> refresh + close.
   - Outbound (§4.4): URL + Name -> add_outbound_workspace{url,label} ->
     refresh + close (reuse attachOutboundUrl validation; Enter submits).
   - Inbound (§4.5): read tunnel_status; not-listening form (Port/Label/
     Workspace -> tunnel_start) and listening state (port + Local|Tunnel
     seg + snippets click-to-copy + [Stop] tunnel_stop / [Done] close).
3. Delete the inline tunnel panel entirely (toggleTunnelPanel,
   renderTunnelPanel, renderTunnelPanelHtml, renderOutboundAttachForm,
   bindTunnelPanelEvents) - the logic moves into the modal.
4. Rows (§3): thead "Path" -> "Where". One renderWhere(d): leading glyph +
   path/URL. Local = existing ic-home/ic-computer (keep reveal click).
   Remote = NEW inline-SVG ic-outbound (we connect out) / ic-inbound (we
   listen) per §3 (out=leaving, in=arriving, visually distinct). D3: remote
   ON cell = a static connection dot (green when d.url present, grey else);
   DROP the url/tunnel text tags. Local rows keep the on/off switch.
5. Gear removal (§3): delete renderFeaturesToggle, renderFeaturesPanel,
   bindFeaturesToggle, loadFeaturesInto, collectFeaturesFromPanel + the
   bindFeaturesToggle(tr) call + the renderFeaturesToggle/Panel calls in
   render(). Remove the JS invokes of get/set_workspace_features. (The Rust
   commands themselves are @@LaneC's lane - leave src-tauri alone.) Delete
   the `.features-*` CSS.
6. Empty-state #empty-pick + boot() first-run auto-open ->
   showNewWorkspaceDialog('local').
7. CSS: add `.nw-*` (clone `.preflight-overlay`/`.preflight-dialog` +
   `.team-realestate-toggle`/`-mode`); add ic-outbound/ic-inbound; rehome
   the tunnel/snippet rules under the modal scope; remove `.features-*`.

## Coordination

@@LaneC runs in parallel deleting the Rust commands get/set_workspace_
features (disjoint files: you=desktop/src, C=desktop/src-tauri). Both land
before verify. If you find a get/set_workspace_features caller you can't
remove from JS, flag @@LaneA before C deletes the Rust side.

## Gate + handoff

- desktop/src/* is plain vanilla JS (no bundler/eslint/test). Build the app
  to confirm it bundles + loads: `cd desktop && make build`. Report
  build-green; note any console errors on boot if you can observe them.
- You CANNOT do the WKWebView click-through (Chrome MCP = Blink, can't reach
  chan-desktop). @@LaneD + @@Alex drive the final smoke.
- Do NOT commit; @@LaneA commits after verify. Leave the tree clean of
  unrelated changes.

## On completion

Cut a completion task to @@LaneA at tasks/task-LaneB-LaneA-2.md
(append-only) - what landed, build status, anything for the smoke - + poke.
