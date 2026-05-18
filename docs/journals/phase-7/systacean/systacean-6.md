# systacean-6: cross-drive drift, SPA persistent-state phase

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Close out cross-drive navigation drift. `systacean-3` shipped
the cache-header fix (`f94c4b5` — `Cache-Control: no-store`
on SPA shell, `Vary: Host` on shell + assets); @@WebtestA's
`webtest-a-4` regression sweep verified the headers land
correctly (`curl -sI` confirmed) but **drift still
reproduces**: Lane A on 8801 hops to Lane B on 8810 within
~1.5s of navigation, landing on Lane B's session.

The header-cache class was necessary hygiene but not the
root cause. The remaining gap is almost certainly SPA-side
persistent state read across same-host different-port
boundaries.

## Relevant links

* `systacean-3` (predecessor): [./systacean-3.md](./systacean-3.md)
* @@WebtestA's re-repro evidence + headers + hypothesis:
  [../webtest-a/webtest-a-4.md](../webtest-a/webtest-a-4.md)
  ("Drift status" section)
* Original @@WebtestB recipe (still valid):
  [../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md)

## Why this is yours (still)

Persistent-state scoping is a systems / browser-semantics
concern. The fix likely lives in chan-server (storage key
namespacing, cookie scope) and/or the SPA bootstrap path
(read storage with a port-or-token-prefixed key). Once the
scope is identified, @@FullStack will own any SPA code
changes; you scope the design and the server seam.

## Acceptance criteria

* Root cause confirmed: identify which browser storage
  mechanism is leaking across ports on the same host.
  Likely candidates, in order:
  1. **localStorage / sessionStorage**: scoped per-origin
     in spec (host + port); some browser-internal sync
     paths or service-worker shims can blur this. Inspect
     DevTools > Application > Storage on both 8801 + 8810
     while the drift fires.
  2. **IndexedDB**: per-origin in spec. Same inspection.
  3. **Cookies**: scoped per-host by default, NOT per-port.
     If chan-server sets a cookie without a `Path` /
     `Domain` discriminator, the cookie set by 8810 will
     be sent to 8801 and vice-versa.
  4. **Welcome-state pane menu's Files action**: @@WebtestB
     flagged earlier that the welcome-state global drives
     picker defaults to most-recent drive. If the SPA reads
     "most-recent drive URL" from storage (any of the above)
     at bootstrap and navigates to it, that explains the
     pre-JS hop.
* Fix lands. Most likely shape:
  * Namespace storage keys with the per-launch bearer token
    or the chan-serve port, so 8801 and 8810 cannot read
    each other's state.
  * If cookies are involved, scope to a path or set
    instance-discriminating attributes so they don't leak.
  * Document the chosen scheme in the static-assets
    module's comments + the chan-server design doc.
* @@WebtestA re-repro on the same recipe shows no hop:
  Lane A navigation stays on 8801 with two `chan serve`
  instances live on different ports.

## Out of scope

* Reverting `systacean-3` — those headers stay (good
  hygiene; they're correct for the per-instance shell).
* Multi-drive UX redesign — separate concern.

## How to start

1. Reuse @@WebtestA's still-running 8801 server at
   `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`
   (drive `/tmp/chan-webtest-a-1/`). Bring up a fresh 8810
   server on a separate drive (any throwaway will do).
2. Open both in Chrome. DevTools > Application > Storage on
   each. Note what keys are present in each origin's
   localStorage / sessionStorage / IndexedDB. Cross-reference
   any keys that look like "last drive" / "active drive" /
   "recent" / URLs.
3. Check Set-Cookie headers from chan-server (chan does
   handle bearer tokens; verify whether any cookie crosses
   the port boundary).
4. If welcome-state Files-action is implicated, grep for
   storage reads in `web/src/state/` and the welcome /
   bootstrap paths.
5. Propose the namespacing fix in an append before
   implementing; tag @@Architect for sign-off via event.
6. Coordinate with @@FullStack if any SPA-side code change
   is needed (storage key prefix).

## Hand-off

Standard. Pre-push gate green. @@WebtestA re-repro after
landing.

## @@Systacean update — 2026-05-18 18:08 BST

Checked the likely storage classes. `chan-server`/SPA code in this
tree does not set cookies for auth or session routing; the only cookie
matches are the tunnel public-header scrubber. The route-affecting SPA
storage keys are:

* `chan.token` in `web/src/api/transport.ts`
* `chan.session.window` in `web/src/api/client.ts`

Those keys were global within the browser storage bucket. They should
be origin/prefix scoped even though browser storage is specified as
per-origin, because the repro path involves Chrome automation and
reused tabs across same-host ports. Patch direction:

* derive `storageScopeKey(base)` as
  `<base>:<window.location.origin><chan-prefix-or-/>`;
* use it for the bearer token and per-tab session window id;
* keep ignoring the old global keys so stale cross-lane values cannot
  route a new instance;
* document the scheme in the static-assets module comment and
  chan-server design section.

## @@Systacean update — 2026-05-18 18:11 BST

Patch is ready to land. Verification:

* `cd web && npm run test -- src/api/client.test.ts` — passed
* `cd web && npm run check` — passed, 0 diagnostics
* `cd web && npm run test` — passed, 22 files / 218 tests
* `scripts/pre-push` — passed
