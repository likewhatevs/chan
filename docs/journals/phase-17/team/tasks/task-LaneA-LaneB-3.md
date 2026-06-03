# task-LaneA-LaneB-3: B12 - direct dashboard chord (out of hybrid nav)

From: @@LaneA  To: @@LaneB  Wave: 2 (HOLD - sequence WITH/right-after B1;
same onWindowKey region as B1's cmd+shift+p. Do not start mid-B8.)

## @@Alex's ask (verbatim, 2026-06-02)

"we need a better shortcut for the dashboard tab.. today it's the only one
we're mixing with the hybrid nav... i want something like cmd+shift+d in tauri
if not conflicting, and what'd be the relative web and linux equivalents"

## Current state (grounded)

- Dashboard has NO direct chord. Reachable only via hybrid nav `Mod+. i`
  (App.svelte handlePaneModeKey) + the hamburger. shortcuts.ts
  `app.dashboard.open` (web/native "Mod+. i") is discoverability-only, NOT a
  dispatch source (shortcuts.ts ~298-310).
- Direct chords are hardcoded e.code branches in App.svelte::onWindowKey
  (~654 cmd+shift+m graph, ~662 cmd+shift+p rich prompt, ~687 cmd+s search).
- openDashboardInActivePane() already exists (store.svelte.ts ~984; imported in
  App.svelte ~76, called ~554/924) - NO @@LaneC change needed; just call it.

## Conflict analysis (lead, verify on smoke)

- NATIVE (Tauri): Cmd+Shift+D (mac) / Ctrl+Shift+D (linux). Not bound in-app
  (app uses Cmd+Shift+M/P/T/]/[, no D); no macOS system-global on Cmd+Shift+D;
  Tauri owns the window on linux. FREE -> use it.
- WEB (browser, mac + linux): Cmd/Ctrl+Shift+D = the browser's "bookmark all
  tabs", which page JS can NOT reliably preventDefault. CONFLICTS. Use
  Alt+Shift+D on web instead - exactly the precedent the app already sets for
  tab nav (app.tab.next web "Alt+Shift+]" vs native "Mod+Shift+]").

## Implement

- shortcuts.ts: change app.dashboard.open to native "Mod+Shift+D",
  web "Alt+Shift+D"; make it a real chord (add escapeTerminal: true so it fires
  from a focused terminal); drop the "no direct chord / Hybrid Nav only" comment
  (keep `Mod+. i` + hamburger working as ALTERNATE paths - do not remove them).
- App.svelte onWindowKey: add a dispatch branch next to cmd+shift+m/p:
    native/mac:  e.metaKey && e.shiftKey && !e.altKey && !e.ctrlKey && code KeyD
    web/linux:   the Alt+Shift+D branch (e.altKey && e.shiftKey && code KeyD),
                 mirroring how the other web vs native chords split (see the
                 isMac branch ~737-738 for the pattern). preventDefault. Use
                 e.code === "KeyD" (layout/Option-glyph agnostic, per the
                 existing comments). -> openDashboardInActivePane().
- Confirm the chord does NOT collide with hybrid-nav transaction mode (the whole
  point: the dashboard stops being "mixed with hybrid nav").

## Gate

- make web-check + svelte-check + npm run build.
- Browser-smoke (web): Alt+Shift+D opens the dashboard in the active pane from
  both a focused editor and a focused terminal; Cmd+Shift+D in the BROWSER still
  does its bookmark thing (we deliberately don't fight it on web).
- Desktop-smoke note: Cmd+Shift+D (mac) is @@Alex's WKWebView hand-smoke
  (agents can't drive WKWebView) - flag it for @@LaneA to route to @@Alex.

## Report

Fold into your B1 report or cut a separate task-LaneB-LaneA-N + poke @@LaneA.
