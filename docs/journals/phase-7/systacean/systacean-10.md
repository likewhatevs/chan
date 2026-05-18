# systacean-10: verify systacean-6 effectiveness, revert if no-op

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Confirm whether `systacean-6` (per-instance SPA storage
scoping via `storageScopeKey(base)`) is load-bearing for
cross-drive drift, or whether `systacean-3`'s `Vary: Host`
on hashed assets is sufficient on its own. @@WebtestA in
`webtest-a-4` noted drift no longer reproduces under
warm-cache stress, and flagged that systacean-6 may be a
no-op once systacean-3 was correct.

@@Alex preference: clean and no tech debt. If the
storage-scope code isn't doing real work, revert it. If
it IS doing work in some scenario systacean-3 doesn't
cover, document the scenario and keep.

## Acceptance criteria

* A controlled repro with **only `systacean-3` headers
  applied**, no per-instance storage scoping. Run two
  `chan serve` instances on different ports on the same
  host, drive each in a fresh Chrome MCP tab, attempt the
  warm-cache drift recipe (navigate to 8810 first to
  populate any storage, then navigate to 8801).
* If drift reproduces with -3 alone: systacean-6 is
  load-bearing. Document the scenario in code comments
  on `storageScopeKey()` and keep. Add a regression test
  that nails the case.
* If drift does NOT reproduce with -3 alone: systacean-6
  is a no-op for the documented bug. Revert the storage-
  scoping (`web/src/api/transport.ts` +
  `web/src/api/client.ts`) and any "ignore stale globals"
  defensive code. Keep the test that proves -3 alone is
  sufficient.

## Out of scope

* Reverting systacean-3 (those headers stay regardless;
  they're correct hygiene for a per-instance shell).
* Other drift sources outside the cross-port storage
  class.

## How to start

1. Revert only the systacean-6 commit locally (`git revert
   83fbb20 --no-commit`); keep -3 in place.
2. Build, run two instances, repro the warm-cache recipe
   from `webtest-a-4`.
3. Two outcomes:
   * Drift reproduces → discard the revert, document.
   * Drift gone → keep the revert, gate + commit.

## Hand-off

Standard. Pre-push gate green. Coordinate with @@WebtestA
for a re-verification on whichever direction lands. Ping
via `alex/event-systacean-architect.md`.

## 2026-05-18 20:56 BST - verification and decision

Decision: `systacean-6` is not load-bearing for the documented
cross-port drift now that `systacean-3`'s shell/asset cache headers
are in place. Reverted the SPA storage key namespacing and its docs:

* `web/src/api/transport.ts` uses the plain `chan.token` key again.
* `web/src/api/client.ts` uses the plain `chan.session.window` key
  again while preserving FullStack's in-flight event-reply API addition
  in the working tree.
* Removed the origin-scoped storage regression test and documentation
  comments introduced by `83fbb20`.
* Kept the existing server tests that prove the remaining fix:
  SPA shell is `Cache-Control: no-store` and hashed assets vary on
  `Host`.

Controlled no-`systacean-6` smoke:

* Built the SPA after reverting the storage scope layer.
* `8801` / `8810` were already bound by existing `chan` processes, so
  I used isolated same-host ports `18801` and `18810` rather than
  killing another agent's servers.
* Started two `target/debug/chan serve --no-browser` instances against
  `/private/tmp/chan-systacean10-a` and
  `/private/tmp/chan-systacean10-b`.
* Confirmed both SPA shells returned `cache-control: no-store` and
  `vary: Host`.
* Browser warm-cache path with the storage-scope layer removed:
  navigated to `18810` first, then to `18801`; after the SPA settled
  and another observation delay, the tab remained on
  `http://127.0.0.1:18801/#files=1%3A`.

Chrome MCP is not exposed in this Systacean runtime; this local browser
smoke is paired with @@WebtestA's earlier Chrome MCP `systacean-3`
warm-cache PASS in `webtest-a-5`, which used the same failure recipe
before `systacean-6` landed.

Verification:

* `npm run test -- src/api/client.test.ts`
* `npm run check`
* `npm run build`
* `cargo test -p chan-server static_cache_headers --no-default-features`
* `cargo fmt --check`
