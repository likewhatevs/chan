# frontend-15: pin window-scoped broadcast invariant

Owner: @@Frontend
Status: REVIEW

## Goal

Make the per-window scope of terminal broadcast explicit and
defended by a test. Broadcast input from a terminal in window A
must never reach a terminal in window B, even if both windows
share the same chan-server (the multi-window chan-desktop case
that phase 5 backend-2 + frontend-7 wired up).

## Why now

[frontend-2](./frontend-2.md) (REVIEW) symmetrized the
broadcast group via `broadcastTargetIds`. Broadcast is
delivered **client-side** (`broadcastTerminalInput` in
`web/src/state/tabs.svelte.ts` writes to other tabs' sockets
from within the same JS context), so it is naturally
window-scoped today. Alex flagged the invariant before close-
out so we lock it in.

Two risks to defend:

1. A future refactor moves broadcast delivery server-side or
   shares state between windows, accidentally leaking input
   across `w=<window-label>` boundaries.
2. A tab id collision between windows lets a broadcast
   target resolve to the wrong window's tab. Tab ids are
   randomly generated and live in per-window state, so this
   is improbable, but the broadcast write path should not
   trust the id alone.

## Scope

* Audit `broadcastTerminalInput` (and any peer that writes
  broadcast bytes) to confirm it iterates only the **current
  window's tab registry** when resolving target ids. Targets
  not present in the current window's tab list must be
  skipped silently (do not error, do not log noisily — that
  surface is hot).
* Add a Vitest covering the invariant: construct two
  in-memory tab registries representing two windows, share
  one member id between them, fire broadcast from window A
  with a target id that exists in B's registry, and assert
  window B's tab never receives the input.
* If `broadcastTargetIds` is persisted to a shared store
  (e.g., chan-drive blob), confirm the persistence is keyed
  by window-label (matches the phase-5 per-window session
  blob design). If not, document the rule in the
  `web/src/state/tabs.svelte.ts` doc-comment near the
  broadcast helpers so future contributors don't lift it
  out.
* No backend changes; this is a frontend-side audit + test.

## Out of scope

* Server-side broadcast bus. Out of scope this phase; the
  client-side fan-out is the contract.
* Cross-window UI hints ("there are N other broadcast groups
  in other windows"). Not asked for.

## Acceptance criteria

* `broadcastTerminalInput` (and any peer) write path
  guarantees a target is resolved against the current
  window's tab registry. Tabs not in the current registry
  are skipped without raising.
* Vitest test pins the cross-window-isolation invariant.
* Comment in `web/src/state/tabs.svelte.ts` near the
  broadcast helpers states the per-window-scope rule
  explicitly.

## Tests

* New Vitest test for cross-window isolation.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.

## Review and hardening

* @@Frontend self-review.
* @@WebtestA live: open chan-desktop with two windows on the
  same drive, start a broadcast group in window A, type into
  the source terminal, confirm window B's terminals receive
  no input. Spot-check the reverse direction.

## Relevant links

* Per-window session blob: phase-5 backend-2 + frontend-7.
* Broadcast helpers: `web/src/state/tabs.svelte.ts`
  (`broadcastTerminalInput`, `broadcastTargetIds`).
* Frontend implementation: [frontend-2](./frontend-2.md).

## Progress notes

* 2026-05-18: Audited `broadcastTerminalInput`; fan-out resolves
  target ids only through `allTerminalTabs()`, which walks this
  window's Svelte layout registry. A registered input sink whose id is
  not present in the current layout is skipped silently.
* 2026-05-18: Added an explicit doc-comment near the broadcast helper
  pinning the per-window invariant and warning against sink-id-only or
  server-bus fan-out.
* 2026-05-18: Added Vitest coverage simulating a cross-window sink id:
  source tab targets `term-b`, a sink for `term-b` exists, but no
  `term-b` tab is in the current layout, so no input is delivered.

## Completion notes

Ready for review. Validation:

* `npm --prefix web run check`
* `npm --prefix web test -- --run` (19 files, 185 tests)
* `npm --prefix web run build` (passes with existing Vite chunk-size,
  ineffective dynamic import, and plugin timing warnings)
