# fullstack-a-74 — Editor hang-recovery banner doesn't surface on force-reload (PARTIAL from webtest-a proactive walk)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Fix the empirical UI gap surfaced in @@WebtestA's
proactive walk of `-a-72`: typing unsaved content +
force-reload does NOT show the restore banner, even
though the mechanism passes vitest (13 pins green).

## Reference

* `-a-72` task body + commit `cb00db0`.
* @@WebtestA verdict (`9278c3d`): "banner UI not
  surfaced empirically (initial-mount race)".

## Audit hypothesis

Two likely race shapes (audit at pickup):

### H1 — Force-reload skips unmount lifecycle

`window.location.reload()` may NOT trigger Svelte
component unmount before the page tears down. The
500ms debounce flush is wired in the mount-effect's
cleanup return, but cleanup may not fire on
force-reload → the last 500ms of edits never persist
to localStorage → mount-time `divergentBufferOrNull`
returns null → no banner.

Fix shape: persist on `beforeunload` (or
`pagehide`) window event in addition to the
debounce + unmount cleanup. Forces the in-flight
buffer to flush synchronously before the page tears
down.

### H2 — Mount-time read runs before initial content loads

`FileEditorTab.svelte`'s mount effect calls
`divergentBufferOrNull(tabId, tabPath, diskContent)`
— but `diskContent` may be empty / stale at mount
time (the file fetch hasn't completed). Divergence
check compares against an empty disk content,
spuriously returns the buffer → banner SHOULD
appear → so this isn't H1's failure mode... but
maybe the check returns null because the comparison
incorrectly matches?

Audit which hypothesis fits via the divergent
helper's call site + the disk-content load timing.

## Fix shape

Most likely (H1):

```ts
// In editorBuffer.ts or App.svelte
window.addEventListener("beforeunload", () => {
  flushPendingBufferWrites();
});
```

Where `flushPendingBufferWrites` cancels the
debounce timer + synchronously writes the latest
content for all open tabs.

Alternatively (H2): defer the
`divergentBufferOrNull` call until disk content has
loaded.

## Acceptance

1. **Force-reload restores empirically**: type
   unsaved content → Cmd+R (or browser refresh) →
   banner appears on reload with the unsaved
   content as the buffer content.
2. **No regression on vitest**: existing 13 pins
   still pass.
3. **No regression on saved-state path**: clean
   state (content === saved) still suppresses the
   banner.

### Tests

Vitest pin for the new beforeunload / pagehide
listener (DOM event simulation) + the synchronous
flush.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.

## Authorization

Yes for editorBuffer + FileEditorTab + tests + task
tail + outbound.

## Numbering

This is `-a-74`.

## Out of scope

* Re-architecting the hang-recovery mechanism beyond
  the unload-flush fix.
* Terminal-side recovery (deferred from `-a-72`).
* The underlying hang root cause.
