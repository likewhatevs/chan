# fullstack-a-83 — Hang-recovery banner STILL not surfacing — effect-ordering race (3rd-round follow-up to -a-82)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: HIGH (data-loss prevention closure)

## Goal

Resolve the 3rd-round `-a-82` STILL PARTIAL surfaced
by @@WebtestA's `206c010` walk: path-keying fix works
(buffer key confirmed `chan:editor-buffer:CLAUDE.md`)
but banner STILL doesn't surface empirically on
divergent reload.

@@WebtestA's framing: "effect-ordering race".

## Reference

* `-a-82` task body + commit `78d3ed4`.
* @@WebtestA's walk `206c010` — `-a-82` 3rd-round
  PARTIAL with effect-ordering rationale.

## Hypothesis (per @@WebtestA's flag)

The two `$effect` blocks in FileEditorTab.svelte:

1. **Mount effect** — sets `recoveredBuffer =
   divergentBufferOrNull(tab.path, tab.path, disk)`.
2. **Persistence effect** — debounced write on
   `tab.content` mutation; clears buffer when
   `tab.content === tab.saved`.

Race shape: when the editor mounts, `tab.content`
may initially load to match `tab.saved` (clean
state) BEFORE the mount effect reads
`divergentBufferOrNull`. The persistence effect's
clean-state branch then CLEARS the buffer just
before the mount effect could surface the banner.

Or vice versa: the mount effect reads the buffer +
sets `recoveredBuffer`, but then the persistence
effect's clean-state branch fires + clears the
buffer + ALSO removes the recovered marker.

## Diagnostic path

1. Add console.warn (or browser-debug breakpoints)
   at:
   * Mount effect entry + computed
     `recoveredBuffer` value.
   * Persistence effect entry + the clean-state
     branch's `clearEditorBuffer` call.
   * Order of fire on mount-after-reload.
2. Confirm which effect fires first + whether one
   clobbers the other.
3. Fix shape (predicted):
   * Make the mount-time `recoveredBuffer` read
     SYNCHRONOUS / snapshot-before the persistence
     effect runs.
   * OR gate the persistence effect's clean-state
     branch on `recoveredBuffer === null` so a
     surfaced banner isn't cleared mid-render.

## Acceptance

1. **Force-reload empirically restores banner**:
   type unsaved → reload → banner surfaces. The
   3rd-round empirical gap closed.
2. **No regression on `-a-82`'s path-keying**:
   buffer key still `chan:editor-buffer:<path>`.
3. **No regression on saved-state suppression**:
   clean state still suppresses banner.
4. **All 18+ prior pins still pass**.

### Tests

Vitest pin for the effect-ordering contract (the
mount-time recoveredBuffer snapshot + the
persistence effect's gating).

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* HIGH priority — closing the data-loss empirical
  gap.
* @@WebtestA empirically re-walks after fix lands.

## Authorization

Yes for FileEditorTab.svelte + editorBuffer.ts +
tests + task tail + outbound.

## Numbering

This is `-a-83`.

## Out of scope

* Re-architecting the recovery mechanism beyond
  the effect-ordering fix.
* Terminal-side recovery.
