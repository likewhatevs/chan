# Task: two desktop/SPA refinements

Dispatched by @@LaneA. Identify your role from `$CHAN_TAB_NAME`.

## R1 - window title by workspace kind (@@LaneB, desktop)

chan-desktop window titles should encode the workspace kind with a leading
icon, then the locator:

- Local disk, under the user's home: `{home-icon} {path}`
- Local disk, elsewhere: `{computer-icon} {path}`
- Remote OUTBOUND (attach to a remote URL): `{outbound-icon} {URL}`
- Remote INBOUND (tunnel; a remote dials in over a local listener):
  `{inbound-icon} {listen-addr}`

Scope the current title logic in `desktop/src-tauri/src/serve.rs`
(`build_workspace_window` / the `workspace-*` vs `tunnel-*` vs `outbound-*`
flavours @@LaneC mapped earlier) and the local-vs-home detection. Use clear
unicode glyphs for the icons; PROPOSE the exact glyphs in your journal and
poke @@LaneA to confirm before finalizing (the four icons are the one open
choice). Own-gate: `make -C desktop check` + cargo fmt/clippy. Do NOT push.

## R2 - pre-flight bubble: checkmark toggle (@@LaneD, SPA)

The workspace-ready bubble shows optional layers as `Semantic search OFF
[Turn on]` / `Reports ON [Turn off]`. The OFF/ON label + Turn on/Turn off
button pair is confusing. Replace BOTH with a single CHECKMARK toggle per row:
a checked box = the layer is ON, unchecked = OFF; clicking the row/box toggles
it (same enable/disable calls the buttons made). Find the bubble component
(the "<workspace> is ready" notification with the Semantic search / Reports
rows). Keep it keyboard-accessible. Own-gate: `make web-check`. Do NOT push.

## Coordination

Own non-overlapping files (R1 = desktop/src-tauri; R2 = the SPA bubble
component + its state). Report to @@LaneA with a 1-line poke pointing at your
journal. @@LaneA owns the full-tree gate + commit before any tag.

### R2 status (@@LaneD) - DONE, own-gate green + browser-smoked

Component: `web/src/components/PreflightOverlay.svelte` (the "<workspace> is
ready" onboard card). Replaced the per-row `on/off` label + `Turn on/Turn off`
button with ONE checkmark toggle per row:
- Each row is now a `<button role="checkbox" aria-checked aria-label>` (the
  whole row is the click/keyboard target; Space/Enter toggle natively, SR
  announces on/off). A check SVG fills the box only when on; busy shows a
  spinner. Removed the now-dead `.onboard-state`/`.onboard-toggle`/
  `.onboard-layer-top` markup + CSS.
- Same enable/disable calls: Reports -> `toggleReports`; Semantic -> a new
  `toggleSemantic` dispatcher routing to the SAME three calls the old buttons
  made (on->disable; off+needs-model->download+enable; off->enable). The
  model-missing case shows a small "downloads ~63 MB" aside; first toggle tries
  enable, next downloads (consent preserved, no auto-download on a stray click).
- Gate: `make web-check` exit 0 (svelte-check 0 errors, vitest 1670/1670, build
  clean). Browser-smoked on a fresh served workspace: card renders the two
  checkmark rows; Reports toggled off->on round-trips (aria-checked + check SVG
  track the API result); Semantic enable verified; both keyboard-native buttons.
  Server + tab torn down.
