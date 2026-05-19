# fullstack-36: external link click does nothing on Chan.app desktop

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

@@Alex reports that clicking external links inside the
Chan.app desktop (Tauri) build does nothing. In the
browser test, the same click opens a new tab. The
intent (per `fullstack-2`) is: desktop = open in the
system browser via the Tauri opener plugin; browser =
new tab via `window.open`.

Diagnose why the call is failing silently on desktop
and fix it.

## Relevant links

* @@Alex's chat note 2026-05-19 06:25 BST.
* Code: `web/src/editor/external_links.ts`
  (`openExternalUrl`).
* Caller: `web/src/components/BubbleOverlay.svelte:412`
  (and editor click handler).
* Predecessor: `fullstack-2` (`3ab0aac` — Route
  external links through the desktop shell). Note:
  the original walkthrough was code-audit only
  because Chrome MCP can't drive WKWebView.

## Why this is silent today

```ts
// external_links.ts (current shape)
if (w.__TAURI__?.opener?.openUrl) {
  await w.__TAURI__.opener.openUrl(url);
  return true;
}
const invoke = w.__TAURI_INTERNALS__?.invoke ?? ...;
if (invoke) {
  await invoke("plugin:opener|open_url", { url });  // ← may throw
  return true;
}
```

* If the invoke throws (capability denied, command not
  registered, name mismatch in Tauri 2.x), the promise
  rejects.
* Callers do `void openExternalUrl(url)` so the
  rejection is swallowed.
* Result: visible "nothing happens" with no console
  surface unless DevTools is open.

## Confirmed config (already correct)

* `desktop/src-tauri/Cargo.toml:26`:
  `tauri-plugin-opener = { workspace = true }`.
* `desktop/src-tauri/src/main.rs:731`:
  `.plugin(tauri_plugin_opener::init())`.
* `desktop/src-tauri/capabilities/default.json`:
  `opener:default`, `opener:allow-open-url`,
  windows: `["main"]`.

So the plugin IS registered. The bug is in either:

1. **The JS-side detection** — `w.__TAURI__.opener` is
   likely NOT auto-injected as a global in Tauri 2.x;
   it's imported from `@tauri-apps/plugin-opener` per
   the plugin's docs. The first branch is dead code on
   2.x, falling through to invoke.
2. **The invoke command shape** — `plugin:opener|open_url`
   uses the Tauri 2.x plugin-invoke shape. Could be a
   minor mismatch (e.g. `:` vs `|` separator, or the
   command name). Confirm against the plugin's docs.
3. **A sub-window** — if the link click happens from a
   window that's not "main", the capability doesn't
   apply. Confirm `windows: ["main"]` matches all
   windows we care about.

## Acceptance criteria

* Clicking a markdown link / link bubble / chat-bubble
  link in the desktop build opens the URL in the system
  browser.
* Same behavior in the web build (opens a new browser
  tab, fallback already handled).
* `openExternalUrl` surfaces errors:
  * `console.warn` (at minimum) when the Tauri path
    fails so future regressions are visible.
  * Caller in `BubbleOverlay.svelte` and the editor
    click handler get a small toast / status when the
    open fails (don't crash the prompt).
* **No-default-browser fallback** (@@Alex 2026-05-19
  06:30 BST):
  * If the Tauri opener call fails for ANY reason
    (no default browser, no app handler for the URL
    scheme, opener plugin error, permission denied),
    show a clear inline toast with the URL text + a
    "Copy URL" affordance. The user can paste the URL
    wherever makes sense.
  * Do NOT silently fall back to opening in chan's
    own webview (defeats the "external" purpose,
    pollutes the SPA's session/cookies).
  * The error message should be plain English: e.g.
    "Couldn't open link in browser — copy URL?".
    Avoid leaking the Tauri error string into the
    user toast (log it to console for debugging).
* Unit test that mocks an invoke-throwing scenario and
  asserts the error is captured / logged.
* If the fix involves importing from
  `@tauri-apps/plugin-opener` (the proper Tauri 2.x
  module path), do that — drop the global-detection
  branch.

## Out of scope

* Routing other URL schemes (mailto / tel beyond what
  `OPENABLE_SCHEMES` already lists).
* Settings-window opener capability (separate question
  if it surfaces).

## How to start

1. Open the Chan.app dev build with DevTools (now
   reachable via the pane hamburger per
   `fullstack-6`!). Open a doc with a link, click,
   watch the console.
2. Inspect what's actually on `window.__TAURI__` and
   `window.__TAURI_INTERNALS__` at runtime — likely
   the SPA's first detection branch never matches on
   Tauri 2.x.
3. Try importing from `@tauri-apps/plugin-opener` if
   that gives a clean call shape.
4. Coordinate with @@Systacean only if the Tauri-side
   capability or plugin init needs adjustment.

## Hand-off

Standard. Pre-push gate green. **This needs real
desktop testing** — DevTools console while clicking
links in Chan.app, since Chrome MCP can't reach
WKWebView. Ping via
`alex/event-fullstack-a-architect.md`.
