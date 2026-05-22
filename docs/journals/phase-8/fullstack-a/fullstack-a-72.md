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
