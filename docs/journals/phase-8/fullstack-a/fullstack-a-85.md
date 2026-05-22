# fullstack-a-85 — File move success toast doesn't auto-dismiss (uses persistent ui.status instead of setTransientStatus)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Make the file-move success toast auto-dismiss (3s
default) like every other action confirmation. Today
it sits stuck until something else overwrites
`ui.status`.

## Reference

@@Alex 2026-05-22 screenshot: "Moved 'docs/journals/
phase-8/alex/addendum-a.md' (15 links updated)"
toast stuck on screen for an extended period.

`web/src/state/store.svelte.ts:2424-2427` writes
`ui.status = ...` directly (persistent shape):

```ts
ui.status =
  linkBits.length > 0
    ? `Moved '${target}' (${linkBits.join(", ")})`
    : null;
```

The transient pattern (auto-dismissing at
`TRANSIENT_STATUS_DEFAULT_MS = 3000`) is in
`setTransientStatus()` at `store.svelte.ts:164-185`.
The move success path should use it.

## Fix shape

Swap the success branch to `setTransientStatus(msg)`:

```ts
const moveMsg =
  linkBits.length > 0
    ? `Moved '${target}' (${linkBits.join(", ")})`
    : null;
if (moveMsg) {
  setTransientStatus(moveMsg);
} else {
  // No link updates worth surfacing — clear any prior
  // status so the user isn't left looking at "Moving…".
  ui.status = null;
}
```

Audit similar adjacent surfaces while in this file:

* `store.svelte.ts:2475` create failed (error path — KEEP persistent).
* `store.svelte.ts:2497` create failed (error — KEEP).
* `store.svelte.ts:2517` rename failed (error — KEEP).
* `store.svelte.ts:2613` delete failed (error — KEEP).

Any other SUCCESS-path direct `ui.status =` writes
that should be transient? Audit + flag at task tail.

## Acceptance

1. **Move success toast auto-dismisses** at 3s
   (TRANSIENT_STATUS_DEFAULT_MS default).
2. **Move error path stays persistent** — user
   notices `rename failed: ...` without it
   vanishing.
3. **"Moving…" in-flight pill behavior unchanged**.

### Tests

Vitest pin on the move success path calling
`setTransientStatus`. Audit pin on the error path
still doing direct `ui.status =`.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* Tiny ~5-10 LOC change.

## Authorization

Yes for `web/src/state/store.svelte.ts` + test +
task tail + outbound.

## Numbering

This is `-a-85`.

## Out of scope

* Re-styling the status pill UX.
* Adjusting the 3s default timeout.
* Other adjacent success-path-stuck-as-persistent
  bugs unless surfaced by the same audit.
