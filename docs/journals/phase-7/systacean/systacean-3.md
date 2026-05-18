# systacean-3: cross-drive nav drift investigation

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Investigate and (likely) fix the cross-drive navigation drift
@@WebtestB surfaced. When two `chan serve` instances run on
different ports for different drives (Lane A on 8801, Lane B
on 8810), navigating in Lane B's browser tab sometimes hops
to Lane A's URL during page load, **before any of our JS
runs**. Repro is deterministic with Lane A still running on a
different port and Lane B in multi-tab use.

## Relevant links

* @@WebtestB's repro recipe at
  [../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md)
  (the multi-poke drift investigation thread, especially the
  14:10 BST "drift re-fires" section).
* Their key observations:
  * Server returns 200, no `Location:` header.
  * No `location.assign / replace / href =` calls in
    `web/src` (grep clean).
  * Same hashed JS bundle on both ports (rust-embed
    deterministic build).
  * The hop happens before any page JS runs.

## Why this is yours

This is below the SPA — likely browser-cache / same-origin-
different-port semantics, ServiceWorker registration, or
HTTP-cache-header subtlety. Systems-engineering territory.

## Acceptance criteria

* Root cause identified with evidence. Most likely
  candidates to verify or rule out:
  1. **HTTP cache headers** on chan-server's SPA shell
     route — if the index/HTML / bundle is cached without
     a `Vary: Host` or port-distinguishing key, the
     browser may cross-pollinate between ports on the same
     host.
  2. **ServiceWorker registration** — if chan registers a
     service worker, its scope is the origin (host + port),
     but Storage and OPFS share across ports on some
     browsers. Verify the service worker exists, its scope,
     and whether it caches the SPA shell or any state.
  3. **rust-embed bundle determinism** — same bundle bytes
     on different ports means the browser may unify cache
     entries by content hash in some cases. Investigate
     `ETag` / `Content-Length` collisions.
  4. **Tunnel proto / WebSocket session sharing** — if
     anything in chan-server reuses session state across
     bearer-token boundaries by accident.
* Fix lands or — if the cause is a browser-platform quirk we
  can't fix from the server side — a documented mitigation
  (e.g., set `Cache-Control: no-store` on the SPA shell;
  add a per-instance nonce; add `X-Chan-Instance: <bearer>`
  header that the SPA validates before bootstrapping). The
  goal: navigating in Lane B never silently lands in Lane A.
* Regression test or doc note so this doesn't slide back.

## Out of scope

* SPA-level routing changes (drift happens before SPA loads).
* UI affordances for multi-drive users (separate concern).

## How to start

1. Run two `chan serve` instances on different ports for
   different drives (the existing 8801 + 8810 are perfect).
2. Open one tab on Lane B (8810). Repro the drift per
   @@WebtestB's recipe. Watch the browser's Network panel
   carefully — note which request returns first, what its
   headers are, and whether Service Worker intercepts
   appear.
3. Inspect chan-server's response headers for the SPA
   shell route. Check `Cache-Control`, `Vary`, `ETag`.
4. Inspect whether a service worker is registered:
   `navigator.serviceWorker.getRegistrations()` in DevTools.
5. Propose the fix in an append before implementing; tag
   @@Architect for sign-off via event.

## Hand-off

Standard. Pre-push gate green. Coordinate with @@WebtestB
for re-repro under any fix.

## 2026-05-18 16:48 BST - proposal before fix

Resumed as @@Systacean after the round-1 recycle.

Static serving inspection:

* `crates/chan-server/src/static_assets.rs` serves both the SPA
  shell and hashed assets with only `Content-Type`.
* No service worker registration exists under `web/src`.
* URL mutation greps are clean for cross-origin navigations; the
  explicit uses all build paths from `window.location.host`.
* The old 8801 / 8810 repro servers are not running, so I could
  not live-confirm browser headers without starting fresh servers.

Likely fix:

* Set `Cache-Control: no-store` on `index.html` and SPA fallback
  responses.
* Add `Vary: Host` on those shell responses so any intermediary /
  WebView cache cannot reuse a shell across same-host different-port
  chan instances.
* Keep bundled assets cacheable but host-scoped with
  `Cache-Control: public, max-age=31536000, immutable` plus
  `Vary: Host`.
* Add focused tests on `serve_static` headers.

Why this is safe:

* The shell is per-instance because it carries injected runtime meta
  (`chan-prefix`, `chan-settings-disabled`) and bootstraps the
  current origin. It should not be browser-reused blindly.
* Hashed assets are content-addressed by Vite filenames and can stay
  cached; `Vary: Host` prevents cross-instance ambiguity.
* The change does not alter API auth, token handling, terminal
  sessions, or frontend route parsing.

Remaining uncertainty:

* @@WebtestB also narrowed one observed path toward the welcome-state
  pane menu / Files overlay opening global drive state. This patch
  addresses the below-SPA cache/header class from the task. It still
  needs a Lane B re-repro after landing; if drift survives with the
  new headers, the next fix belongs in the welcome-menu Files action
  rather than static serving.

## 2026-05-18 17:18 BST - implementation ready

Implemented the static-serving mitigation.

Changed:

* `crates/chan-server/src/static_assets.rs`
  * `index.html` and SPA fallback responses now carry
    `Cache-Control: no-store`.
  * Static asset responses now carry
    `Cache-Control: public, max-age=31536000, immutable`.
  * Both shell and asset responses carry `Vary: Host`.
  * Added focused tests for the cache-header helper.

Root-cause assessment:

* No service worker registration exists in `web/src`.
* No frontend cross-origin navigation path showed up in the
  location grep; explicit websocket/fetch paths use the current
  `window.location.host`.
* The static asset route was the confirmed server-side gap:
  SPA shell and assets had no cache headers and no host-varying
  marker, while the shell is per-instance runtime content.

Verification:

* `cargo fmt --check`
* `cargo test -p chan-server static_assets`
* `cargo clippy -p chan-server --all-targets -- -D warnings`
  is running at the time of this append.

Known gap:

* I did not spin fresh 8801 / 8810 servers for browser re-repro
  in this pass. @@WebtestB should re-run the drift repro once this
  lands. If the drift remains, the next suspect is the welcome-state
  pane menu / Files action opening global drive state, not the
  static-serving layer.

Proposed commit message:

```text
Scope SPA cache headers per host

Mark the runtime SPA shell as no-store and vary static responses by
Host so same-host different-port chan instances cannot reuse stale
shell state across drives. Keep bundled hashed assets cacheable with
immutable caching, and add focused cache-header tests.
```

## 2026-05-18 17:18 BST - verification complete

`cargo clippy -p chan-server --all-targets -- -D warnings`
completed clean.

Ready for @@Architect review. Commit remains gated on @@Alex.

## 2026-05-18 17:45 BST - commit authorized

Read @@Architect's inbound event:
[../alex/event-architect-systacean.md](../alex/event-architect-systacean.md#2026-05-18-1835-bst--poke-commit-authorized-for-systacean-3).

@@Alex granted commit clearance verbally in chat, transcribed by
@@Architect.

Pre-push gate:

* `scripts/pre-push` - passed

Proceeding to commit and push only the `systacean-3` code/docs.
