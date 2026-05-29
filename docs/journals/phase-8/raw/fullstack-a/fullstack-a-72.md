# fullstack-a-72 — Editor/terminal hang recovery via localStorage buffer (data-loss prevention)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: high (data loss prevention)

## Goal

Persist editor + terminal in-progress state to
`localStorage` so a window-reload (forced when the
SPA hangs) doesn't lose unsaved data.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md):

> Some times when the editor or terminal hangs, the
> only way to get back to it on the desktop native
> app is by closing the window and reopening.. the
> problem is that in the editor when you do this you
> end up losing data (just happened to me writing
> this very doc) and your terminals come back with
> that rendering issue; i think we have a task for
> this in the queue, check if we need to dedup but i
> want this resolved - we may need to use buffers in
> localStorage, whatever solves the problem for
> allowing to reload on crash of the js

## Pre-pickup audit

@@Alex flagged "i think we have a task for this in
the queue, check if we need to dedup". Audit step 1
at pickup: grep the bug list + task journal for any
existing "hang recovery" / "localStorage" / "draft
buffer" task. If found, dedup (close this task; bump
the existing one) OR merge scope.

If no existing task: proceed with this body.

## Scope

### Editor buffer persistence

* On every editor mutation (debounced, e.g. 500ms),
  serialize the unsaved content to localStorage
  under a per-tab key (e.g. `chan-editor-buffer-<tab-id>`).
* On editor mount: if a buffer exists for this tab,
  detect divergence vs the on-disk content. If
  divergent, surface a "Restore unsaved changes?"
  toast / banner.

### Terminal scrollback / state persistence

Less critical (terminals rarely carry user-typed
unsaved data in the same way; the rendering issue is
a separate concern). For this task, scope to EDITOR
buffer recovery primarily.

If audit reveals the terminal rendering issue is in
scope (per @@Alex's note "terminals come back with
that rendering issue"), surface as a side-finding +
file as a separate task.

### Cleanup

* On successful save → clear the localStorage buffer
  for that tab.
* On tab close (user-initiated) → clear buffer.
* On graceful unmount → clear buffer.

### Eviction policy

* Cap on total localStorage usage (e.g. 10MB across
  all editor buffers). Drop oldest on cap exceed.
* TTL: buffers older than X days (e.g. 7) get
  evicted on next page load.

## Acceptance

1. **Edit content; force reload**: open editor;
   type unsaved changes; reload window; content is
   recovered (with a banner / toast indicating
   "restored").
2. **Saved content + reload**: edit + save +
   reload; no banner (clean save state).
3. **Buffer eviction**: stale buffers (older than
   TTL) cleaned on next load.
4. **Cap respected**: storage cap prevents
   localStorage overflow.

### Tests

Vitest pins for buffer-write debounce, restore-on-
mount, divergence detection, save-cleanup, eviction.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.

## Authorization

Yes for editor / state SPA files + tests + task tail
+ outbound.

## Numbering

This is `-a-72`.

## Out of scope

* Terminal rendering issue on window-reload (separate
  task if surfaced during audit).
* The underlying hang root cause — separate
  investigation. This task is a SAFETY NET for
  unsaved data while the hang root cause is
  diagnosed in parallel.

## 2026-05-22 — ready for review

Five-file change. SPA-only; no Rust touched.

### Pre-pickup audit

@@Alex's "i think we have a task for this in the
queue" — searched `phase-8-bugs.md` + task
journals for existing hang-recovery /
localStorage / draft-buffer tasks. **No existing
task** matches; proceeded with this body. (Other
hang-adjacent entries are about specific
rendering / focus issues, not a localStorage
recovery buffer.)

### What landed

`web/src/state/editorBuffer.ts` (new): the
buffer module.

* `writeEditorBuffer(tabId, content, path)` —
  persists `{ content, updatedAt, path }` to
  `chan:editor-buffer:<tabId>`. Self-prunes
  on `QuotaExceededError` (one retry).
* `readEditorBuffer(tabId)` — returns
  `EditorBuffer | null`. Clears + returns null
  on malformed entries.
* `clearEditorBuffer(tabId)`.
* `pruneEditorBuffers()` — two-pass eviction:
  TTL drop (`MAX_BUFFER_AGE_MS = 7 days`) +
  total-size cap drop oldest-first
  (`MAX_BUFFER_BYTES = 10MB`).
* `divergentBufferOrNull(tabId, tabPath,
  diskContent)` — helper for the editor mount
  path: returns the buffer only when its
  content actually differs from what's on
  disk (clean state ⇒ null; path mismatch ⇒
  clear + null).
* SSR-safe: every entry point gates on
  `typeof localStorage !== "undefined"` so
  bootstrap / unit-test environments without
  localStorage no-op cleanly.

`web/src/components/FileEditorTab.svelte`:

* New `recoveredBuffer` state + `$effect` mount
  hook calling `divergentBufferOrNull`.
* New `$effect` debounced (500ms) write on
  every `tab.content` mutation. Skips the
  write + clears any stale buffer when the
  content matches `tab.saved` (clean state).
* Cleanup teardown returned from the mount
  effect flushes the pending timer on
  unmount so Cmd+W close doesn't drop the
  last 500ms of edits.
* `restoreFromBuffer()` — sets
  `tab.content = recoveredBuffer.content`,
  dismisses the banner. The debounced effect
  re-persists the restored content on next
  tick.
* `discardBuffer()` — clears the storage entry
  + dismisses the banner.
* Banner template + CSS at the top of the
  editor-tab body. Uses `--warn-text` for the
  Restore button (attention-needed affordance).

`web/src/App.svelte`:

* Imports + calls `pruneEditorBuffers()` on
  app mount. Keeps localStorage tidy for
  long-lived sessions.

`web/src/state/editorBuffer.test.ts` (new):
**13 raw-source pins** covering write/read
roundtrip, missing-key null, clear, key
prefix, malformed-entry recovery (bad JSON +
wrong shape), TTL eviction (7+ days drop, in-TTL
kept, non-buffer keys not touched), and the
`divergentBufferOrNull` helper (4 branches).

Tests include an inline minimal `Storage`
polyfill in `beforeAll` because vitest's jsdom
setup in this repo (vitest 4 + jsdom 29) ships
a `window` without `localStorage`. The polyfill
matches the browser Storage shape exactly so
the module's real branches all run.

### Acceptance

1. **Edit + force reload restores**: type
   unsaved → reload → mount-time
   divergentBufferOrNull surfaces the banner →
   user clicks Restore → editor content =
   buffer content. ✓ (mechanism-confirmed via
   tests; UI walk by @@WebtestA to confirm
   empirically.)
2. **Saved content + reload: no banner**:
   clean state (content === saved) clears the
   buffer + skips persistence, so on next
   reload the mount-time check finds nothing
   and the banner doesn't surface. ✓
3. **Buffer eviction (TTL)**: 7-day cutoff;
   prune sweeps on app mount. ✓
4. **Cap respected**: 10MB total; size-cap
   pass drops oldest-first until under cap.
   ✓

### Gate

* vitest **809 / 809** (+13 net from `-a-67`
  slice 1b's 796).
* svelte-check 0 errors / 0 warnings across
  4008 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **localStorage, not sessionStorage** — the
  bug body specifies "buffers in
  localStorage". Persisting across window
  close + reopen is the desired semantic.
* **Per-tab key `chan:editor-buffer:<tabId>`**
  — tab ids persist across reloads via
  SerTab, so the key is stable. Path
  validation in `divergentBufferOrNull`
  defends against tab-id collisions across
  drives (defensive; unlikely in practice).
* **500ms debounce** — common pattern for
  background persistence; balances "don't
  miss recent edits" against "don't spam
  localStorage on every keystroke."
* **Clear on clean state** — when
  content === saved, no recovery is needed,
  so dropping the buffer keeps storage
  bounded + avoids surfacing a banner on a
  cleanly-saved file.
* **Banner over modal** — non-blocking; the
  user can ignore + keep working with the
  disk content. Restore button is clearly
  styled.
* **Terminal scrollback deferred** — task
  body recommends primary focus on editor +
  filing terminal as a separate task if it
  surfaces in audit. Audit didn't surface
  it; terminal restoration is genuinely
  different machinery (xterm.js scrollback
  buffer is not text-mutable state). Leaving
  for a follow-up if @@Alex flags
  empirically.

### Suggested commit subject

```
Editor hang-recovery: persist unsaved content to localStorage with restore banner (fullstack-a-72)
```

Single commit. Module + integration + banner
UI + tests tightly coupled around the same
recovery contract.

### Files for `git add` (per-path discipline)

* `web/src/state/editorBuffer.ts` (new)
* `web/src/state/editorBuffer.test.ts` (new)
* `web/src/components/FileEditorTab.svelte`
* `web/src/App.svelte`
* `docs/journals/phase-8/fullstack-a/fullstack-a-72.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
