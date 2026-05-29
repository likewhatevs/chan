# @@Frontend task 5: strip unknown keys from the URL hash

Owner: @@Frontend
Status: REVIEW
Source: [webtest-2](./webtest-2.md) follow-up #3.

## Goal

Pre-Phase-5 URLs (`#assistant=open`, `#scopes=2`, …) currently survive
across reloads because the hash router only *reads* known keys and
leaves unknown ones in place. Phase 5 removed those keys, so the
surviving fragments are pure stale state.

@@Architect's call (per the no-clarifying-questions directive): strip
unknown keys on the next hash write. Cleaner URLs, no lingering
references to the removed overlays.

## Acceptance criteria

* When the hash-state writer in `web/src/state/store.svelte.ts` (or
  wherever the consolidated hash-state lives after [frontend-2](./frontend-2.md))
  serialises the next hash, it emits **only** known keys.
* A reload with `#assistant=open&scopes=2&settings=1` ends up with
  `#settings=1` (or the appropriate live state), not the original
  string.
* No regression in the existing hash round-trip behaviour for live
  overlays (`settings=`, `files=`, `graph=`, `terminal` / `s=`, etc.).
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` all green.

## Test expectations

* Add a small unit test in `web/src/state/store.test.ts` asserting
  that a hash containing both known and unknown keys round-trips to
  the known-only subset.

## Out of scope

* Aggressive blocking of unknown keys at read time. The router
  already ignores them; that's fine. We just stop *writing* them
  back.
* Migration to a typed enum of allowed keys (a future cleanup; this
  task is the smallest fix that closes the follow-up).

## Progress

* 2026-05-17 @@Frontend reconciled this task after the update check.
* `persistStateToHash()` already canonicalizes the URL hash to the known
  Chan keys before writing, so stale pre-Phase-5 keys are dropped on the next
  normal hash write.
* Tightened the regression test in `web/src/state/store.test.ts` so a hash
  containing stale keys plus live `settings=1` writes back as `#settings=1`.

## Completion notes

* Verification:
  * `npm --prefix web run check`
  * `npm --prefix web test -- --run`
  * `npm --prefix web run build`
* Build completed with existing Vite chunk-size / ineffective dynamic-import
  warnings, but no errors.
