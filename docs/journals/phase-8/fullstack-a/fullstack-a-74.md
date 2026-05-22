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

## 2026-05-22 — ready for review

Three-file change. SPA-only.

### Audit verdict

Picked **H1 — unload-skip** as the root cause.
Source-level trace:

* `-a-72`'s 500ms debounce lived in
  `FileEditorTab.svelte`'s `$effect` via a local
  `setTimeout`. Cleanup was wired in the mount
  effect's teardown.
* `window.location.reload()` (Cmd+R → `reloadWindow()`)
  does NOT trigger Svelte component unmounts — the
  page tears down at the navigation layer, skipping
  cleanup callbacks.
* Net effect: in-flight timer's pending write
  never reached localStorage → mount-time
  `divergentBufferOrNull` returned null → banner
  didn't surface.

H2 was ruled out at audit: FileEditorTab is only
mounted AFTER tab.content has loaded (Pane.svelte
gates on `active?.kind === "file"` where `active`
comes from `pane.tabs.find((t) => t.id === pane.activeTabId)`
and the file-fetch happens at tab-open time, not
on mount).

### Fix shape: shared queued-write registry

`web/src/state/editorBuffer.ts`:

* New `pendingWrites: Map<tabId, PendingWrite>`
  module-level state.
* `queueBufferWrite(tabId, content, path)` —
  schedules a 500ms debounced write. Replaces
  any prior pending entry for the same tab.
* `cancelPendingBufferWrite(tabId)` — cancels
  the timer + removes the entry. Used on
  graceful unmount / clean transitions.
* `flushPendingBufferWrites()` — drains the
  Map synchronously, calling `writeEditorBuffer`
  for each entry. Used by App.svelte's unload
  listeners.

`web/src/components/FileEditorTab.svelte`:

* Replaced the inline `setTimeout` debounce with
  `queueBufferWrite`. The mount-effect cleanup
  now calls `cancelPendingBufferWrite` instead
  of clearing a local timer.
* Clean-state branch calls
  `cancelPendingBufferWrite` + `clearEditorBuffer`
  (was just clearEditorBuffer; the cancel
  defends against a race where a pending write
  could overwrite the cleared state).

`web/src/App.svelte`:

* Imports `flushPendingBufferWrites`.
* Registers `beforeunload` + `pagehide` listeners
  in onMount that call `flushPendingBufferWrites`
  synchronously. Both events fire reliably before
  page teardown; `pagehide` is mobile-safe,
  `beforeunload` covers desktop reload.
* Cleanup removeEventListener on destroy (for
  HMR / SSR symmetry; the handlers are tiny so
  the leak risk if missed is negligible).

### Tests

`web/src/state/editorBuffer.test.ts`: +5 new
pins covering the debounce timing, latest-call-
wins semantics, cancel behaviour, multi-tab
synchronous flush, and flush idempotence.
Uses `vi.useFakeTimers()` to drive the debounce
deterministically.

### Acceptance

1. **Force-reload restores empirically**:
   `beforeunload` fires before
   `window.location.reload()` tears down the
   page; `flushPendingBufferWrites` runs
   synchronously + persists the latest content.
   Mount on the new page reads it back via
   `divergentBufferOrNull` + surfaces the banner.
   ✓ (mechanism-confirmed via tests; @@WebtestA
   re-walk to confirm empirically).
2. **No regression on vitest**: 13 prior pins
   + 5 new = 18 total, all green ✓.
3. **No regression on clean-state path**:
   FileEditorTab's clean-state branch now also
   cancels any pending write, ensuring the
   cleared buffer doesn't get clobbered by a
   late debounce flush. ✓

### Gate

* vitest **819 / 819** (+5 net from `-a-73`'s
  814).
* svelte-check 0 errors / 0 warnings across
  4010 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Registry in editorBuffer.ts, not
  FileEditorTab.svelte** — the flush callback
  needs to run independently of Svelte's
  component-lifecycle teardown. Module-level
  state is the clean place.
* **`beforeunload` AND `pagehide`** — both
  registered to cover platform quirks. Both
  call the same idempotent flush; double-fire
  is a no-op after the first.
* **Sync flush only** — `beforeunload` can't
  reliably await async work. `flushPendingBufferWrites`
  is purely synchronous (clearTimeout + sync
  localStorage write).
* **Cancel-before-clear in clean transition**
  — defends against the case where the user
  saves while a debounced write is in flight;
  without cancel, the post-save flush would
  re-persist the now-saved content as a "buffer"
  + surface the banner on next reload.

### Suggested commit subject

```
Hang-recovery: flush pending buffer writes on beforeunload / pagehide (fullstack-a-74)
```

Single commit. Module + integration + listener
+ tests tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/state/editorBuffer.ts`
* `web/src/state/editorBuffer.test.ts`
* `web/src/components/FileEditorTab.svelte`
* `web/src/App.svelte`
* `docs/journals/phase-8/fullstack-a/fullstack-a-74.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
