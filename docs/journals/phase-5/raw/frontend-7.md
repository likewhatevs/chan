# @@Frontend task 7: OBS-WT5-D — per-tab session key for plain-browser users

Owner: @@Frontend
Status: REVIEW
Severity: MEDIUM — data loss across plain-browser tabs that share
the same drive on the same origin. chan-desktop is unaffected
(each window already gets a `w=<window-label>`).
Source: [webtest-1](./webtest-1.md) round-5 smoke, OBS-WT5-D.

## Symptom

Two browser tabs pointed at the same `chan serve` origin both
fall back to `w=default` for their session-blob key (the chan-
desktop fix from [backend-2](./backend-2.md) only set `w=<label>`
when chan-desktop wraps the URL). Result: the later-writing tab
overwrites the earlier tab's session blob. Webtest A observed a
4-tab layout in tab A collapse to a 1-tab layout when tab B
navigated to a bare hash on the same origin.

## Why this isn't covered by backend-2

[`backend-2`](./backend-2.md) deliberately landed
"browser fallback to `default`" because the original ask was
about chan-desktop multi-window. The browser-multi-tab story
falls through to that fallback, which is "last writer wins" by
design. The cost only becomes visible when more than one plain-
browser tab targets the same drive at the same time.

## Goal

Give each plain-browser tab its own session-blob key so tabs do
not stomp on each other.

## Proposed fix

* On first load (no `w=` in URL, no key in `sessionStorage`),
  generate a short random `w` id (e.g. 8 hex chars) and persist
  it in `sessionStorage` under a known key
  (`chan.session.window` or similar).
* Reuse the stored id on every subsequent load within the same
  browser tab (sessionStorage scope is per-tab, which is exactly
  what we want — one persistent id per tab session).
* The id is **never** added to the URL (sharing the URL must
  still hand the recipient a clean drive root, not the sender's
  session).
* `web/src/api/client.ts` already derives the session key from
  the URL today via the chan-desktop `w=` path; extend it to
  prefer the URL key when present (chan-desktop), then the
  sessionStorage key (browser), then `default` (legacy fallback).

## Acceptance criteria

* Two plain-browser tabs on the same `http://127.0.0.1:8787/`
  origin each have a unique session-blob key. Layouts in either
  tab survive a reload of the other.
* chan-desktop unchanged: `w=<window-label>` from
  [backend-2](./backend-2.md) still wins over the
  sessionStorage path.
* Tunneled drives unchanged.
* Existing tests still pass; add one new test for
  "browser-tab fallback prefers sessionStorage over default."
* `npm --prefix web run check` + `npm --prefix web test -- --run`
  + `npm --prefix web run build` all green.

## Hardening expectations

* On private/incognito tabs the sessionStorage entry still
  scopes per-tab, so the behaviour is the same; no quota issues
  (we're storing a tiny string).
* If sessionStorage is unavailable (rare; some embedded webviews
  in privacy-locked-down modes), fall through to `default` so
  the app does not break. Log a one-time console warning.

## Coordination

* @@Webtest B's "two browser windows on the same service" baseline
  observation in [webtest-2](./webtest-2.md) implicitly assumes
  the current shared-`default` behaviour; this fix flips that
  expectation. Update the baseline note.

## Out of scope

* Reconciling two tabs that already shared `default` before this
  fix landed. Old shared blobs stay where they were; new tabs
  get fresh per-tab keys.
* Surfacing the per-tab id in the URL (deliberately rejected
  above; shared URLs must not carry session state).

## Progress

* 2026-05-17 @@Frontend started after the round-5 webtest poke.
* Updated `web/src/api/client.ts` session key selection:
  1. chan-desktop URL `w=<window-label>` wins;
  2. plain browser tabs use a generated 8-hex sessionStorage key under
     `chan.session.window`;
  3. if sessionStorage is unavailable, fall back to legacy `default` and log
     a one-time warning.
* Updated `web/src/api/client.test.ts` to cover the sessionStorage fallback
  and generated-id reuse. Existing URL-label tests still cover chan-desktop
  priority and encoding.

## Completion notes

* Verification:
  * `npm --prefix web run check`
  * `npm --prefix web test -- --run`
  * `npm --prefix web run build`
* Build completed with existing Vite chunk-size / ineffective dynamic-import
  warnings, but no errors.
* Webtest B's old plain-browser `w=default` baseline should be considered
  superseded: no-`w` browser tabs now use per-tab session keys.
