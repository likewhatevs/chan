# task-LaneB-LaneA-3: B12 DONE - direct dashboard chord (out of hybrid nav)

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-3 (Wave-2 B12)

## Result: dashboard now has a direct chord; empirically verified in Chrome.

Web: Alt+Shift+D. Native: Mod+Shift+D (Cmd+Shift+D mac / Ctrl+Shift+D linux).
Hybrid Nav `Mod+. i` + the hamburger remain as alternate paths. Dashboard is
no longer mixed with hybrid nav.

## Implement (your conflict analysis confirmed)

- shortcuts.ts `app.dashboard.open`: web "Alt+Shift+D", native "Mod+Shift+D",
  escapeTerminal:true (fires from a focused terminal), note "or Mod+. i".
  Web uses Alt+Shift+D because Cmd/Ctrl+Shift+D = browser "bookmark all tabs"
  (page JS can't reliably preventDefault) - the same web-vs-native split the
  app already uses for tab/pane nav.
- App.svelte onWindowKey: a `KeyD` branch beside cmd+shift+p. Native guarded by
  isTauriDesktop() + currentOS() (Cmd mac / Ctrl linux), web = Alt+Shift+D ->
  openDashboardInActivePane(). The `chan:command app.dashboard.open` bridge
  case ALREADY existed (~926), so if KEY_BRIDGE_JS later intercepts Cmd+Shift+D
  natively it routes through the bridge (stopImmediatePropagation, no
  double-fire); otherwise this onWindowKey branch handles native directly.
- shortcuts.test.ts: dashboard chord assertion updated. The bridge-case test +
  EmptyPaneWelcome test use chordLabel(chordId) dynamically - unchanged.

## Files changed (App.svelte + shortcuts.ts are my lane; shortcuts.ts edit
   authorized by this task)

  web/src/state/shortcuts.ts        blob 3e7ed08b898b84599dda4cdd9992737c53a1e392
  web/src/App.svelte                blob 629cd60a7f5f61687594dafedaac765f463c0eaa
  web/src/state/shortcuts.test.ts   blob e2dbac4f445ab9329d4cf14427960654a6ac6b22

App.svelte blob also carries B1 (the rich-prompt work); B12 added only the
KeyD dispatch branch.

## Own-gate (scoped) - GREEN

  npx vitest shortcuts.test + dashboardTabAndCarousel.test   PASS (59)
  npm test (full vitest)                                     PASS (1647)
  npm run check (svelte-check)                               0 ERRORS
  npm run build                                              OK
(The lone svelte-check warning is the pre-existing B1 RichPrompt root-div one;
exit 0.)

## Empirical smoke (Chrome / Blink, fresh binary on :8792)

- Alt+Shift+D from a focused TERMINAL -> opens the Dashboard in the pane
  (escapeTerminal verified: it fired from inside xterm).
- Alt+Shift+D from a focused EDITOR (notes.md) -> opens the Dashboard.
- Chord hint "Alt+Shift+D" renders in the launcher Dashboard tile AND the pane
  hamburger menu (registry change propagated to the UI).
- I deliberately did NOT press Cmd+Shift+D in Chrome (it opens the browser's
  bookmark-all dialog, which would block automation). Our web branch only binds
  Alt+Shift+D; isTauriDesktop() is false in Chrome so Cmd+Shift+D never matches
  our handler - i.e. we correctly do not fight the browser on web.
Torn down: closed my Chrome tab, killed the server by PID, chan remove, rm
temp. @@LaneD's b11test server (:8810) untouched (no broad pkill).

## For you (desktop hand-smoke + optional KEY_BRIDGE)

- Native Cmd+Shift+D (mac WKWebView) needs @@Alex's hand-smoke (agents can't
  drive WKWebView). Please route to @@Alex.
- If WKWebView swallows Cmd+Shift+D before it reaches JS, add Cmd+Shift+D to
  chan-desktop KEY_BRIDGE_JS (desktop/src/*, your lane). The bridge already
  has `case "app.dashboard.open"`, so no further App.svelte change is needed.

## Status

Wave-2 done on my side: B1 + B12 landed + verified. B4 still HELD per your poke
(B4 + @@LaneD's B5 serialize the chan-server crate). Holding for your B4
dispatch once @@LaneD lands B5.
