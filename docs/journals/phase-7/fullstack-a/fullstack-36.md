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

## 2026-05-19 10:28 BST — @@FullStackA specialist review

### Root cause

`openExternalUrl` had two compounding problems on Chan.app:

1. The detection branch ordering tried
   `w.__TAURI__?.opener?.openUrl` first, but Tauri 2.x doesn't
   inject the opener as a global — the call should go through
   `__TAURI_INTERNALS__.invoke("plugin:opener|open_url", { url })`.
   That branch DID exist as a fallback but the failing path
   threw uncaught when nothing handled the URL (no default
   browser, no app registered for the scheme, capability
   denied).
2. Callers (`BubbleOverlay.svelte:412`, the editor click
   handler) do `void openExternalUrl(...)`. Any rejection
   from the `await invoke(...)` was swallowed, so the user
   saw "nothing happens" with no console surface unless
   DevTools was already open.

### Fix

* `web/src/editor/external_links.ts` — split the Tauri path
  into `tryTauriOpen(w, url)` that swallows + logs the
  rejection and returns `false`. `openExternalUrl` now
  detects the Tauri webview by either runtime global,
  calls `tryTauriOpen`, and on failure calls
  `copyAndNotifyFailure(url)`.
* `copyAndNotifyFailure` copies the URL to the clipboard
  via `navigator.clipboard.writeText` (if available), then
  surfaces a plain-English status via `notify(...)`.
  Strings: "Couldn't open link in browser — URL copied to
  clipboard" on clipboard success, "Couldn't open link in
  browser — <url>" on clipboard failure.
* Inside the Tauri webview the function NEVER falls back
  to `window.open` — that would route the URL through
  Chan.app's WKWebView, defeating "external" and
  polluting the SPA's session cookies. Web build retains
  the `window.open` path.
* The Tauri error string is logged to `console.warn` for
  debugging; the user-facing toast stays clean.

### Tests

* `external_links.test.ts` — kept the original three
  tests (opener-bridge, invoke-bridge, web window.open),
  added a new `openExternalUrl no-default-browser
  fallback` describe with three tests:
  * Opener throws → status message asserts "URL copied
    to clipboard" string + clipboard call asserted.
  * Both opener and clipboard throw → status string
    includes the raw URL.
  * Tauri webview never falls back to `window.open`.
* `setNotifyHandler` from `notify.svelte` is exposed so
  the test can capture emitted strings without going
  through the store.

### Gate

* `npm run test -- external_links` — 8 passed.
* `npm run test` — 30 files / 274 tests, all pass.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green.

### What still needs real desktop verification

DevTools-on-WKWebView confirmation that:

1. A markdown link in a file editor opens in the system
   browser when the user clicks it.
2. A link inside a bubble overlay does the same.
3. The fallback toast renders + URL is on the clipboard
   when the opener can't dispatch (e.g. testable by
   temporarily configuring no default browser).

Cannot drive WKWebView from Chrome MCP, so this depends
on @@WebtestA / @@WebtestB picking up a manual pass on
Chan.app, or on @@Alex spot-checking the live app.
Flagged in the hand-off.

### Proposed commit message

> Surface external-link open failures on desktop (fullstack-36)
>
> Wrap the Tauri opener call so its rejection no longer gets
> swallowed by the `void openExternalUrl(...)` callers. When
> the opener can't dispatch the URL (no default browser, no
> app for the scheme, capability denied), copy the URL to
> the clipboard and surface a plain-English status message
> via notify(); never silently fall back to window.open
> inside the Tauri webview.

Ready for commit + push under standing topic-level
clearance.
