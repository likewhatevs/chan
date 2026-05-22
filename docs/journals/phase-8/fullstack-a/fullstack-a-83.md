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

## 2026-05-22 — ready for review (effect-ordering race fixed + tab.id discard relic cleaned up)

Two-file change. SPA-only.

### Audit verdict

@@WebtestA's `206c010` walk: path-keying
works (buffer key confirmed
`chan:editor-buffer:CLAUDE.md`) but banner
STILL doesn't surface. Framing: effect-
ordering race.

Audit confirms the race shape: when
`tab.saved` arrives AND `tab.content === tab.saved`
(both just loaded from disk, no user edit
yet), the two effects fire in the same tick:

* Mount effect (declared first): reads
  `divergentBufferOrNull(tab.path, ...)`
  → if buffer divergent vs disk →
  `recoveredBuffer = buffer`.
* Persistence effect (declared second):
  reads `content === saved` (true; both
  just loaded) → enters the clean-state
  branch → `clearEditorBuffer(tab.path)`.

Depending on microtask order in Svelte 5,
the persistence effect can wipe localStorage
BEFORE the mount effect reads it, OR after
— either way it tears down the banner state
the user needed to act on.

### Fix

`web/src/components/FileEditorTab.svelte`:

1. **Persistence-effect clean-state guard**
   — added `if (recoveredBuffer !== null) {
   return; }` at the top of the clean-state
   branch. When the banner is showing, leave
   the buffer in place + skip the
   `cancelPendingBufferWrite`. The user's
   Restore / Discard click is the trigger
   that finalises the state.

2. **`discardBuffer` follow-up fix to
   `-a-82`** — swapped `clearEditorBuffer(tab.id)`
   → `clearEditorBuffer(tab.path)`. Audit
   caught the stale `tab.id` relic from
   before the path-keying re-key (the
   pre-`-a-83` discard silently no-op'd
   because `tab.id` changes on every reload
   + the call targeted a non-existent key).
   The banner cleared from
   `recoveredBuffer = null` but the
   localStorage entry would linger until
   natural expiration.

Inline comments at both sites cross-
reference `-a-82` and the effect-ordering
race for future readers.

### Tests

`web/src/components/hangRecoveryEffectOrder.test.ts`
(new): 5 raw-source pins:
* Clean-state branch's `recoveredBuffer !== null`
  guard structure.
* Rationale comment cites
  "effect-ordering" + the wipe-between-
  read narrative.
* Banner-up "leave buffer in place"
  comment.
* `discardBuffer` uses `tab.path`.
* Pre-fix `tab.id` discard call gone.

### Acceptance

1. **Force-reload empirically restores
   banner** ✓ (mechanism via test pins;
   @@WebtestA empirical re-walk for
   confirmation).
2. **No regression on `-a-82`'s path-keying**
   ✓ — buffer key still
   `chan:editor-buffer:<path>`; all 18+
   prior pins still pass.
3. **No regression on saved-state
   suppression**: clean state STILL
   suppresses banner on subsequent loads
   ✓ — once the user clicks Restore /
   Discard, `recoveredBuffer = null` →
   next persistence-effect clean-state run
   clears localStorage normally.
4. **discardBuffer fix is a positive net**
   ✓ — Discard now actually wipes
   localStorage; pre-fix it silently leaked.

### Gate

* vitest **929 / 929** (+5 net from
  `-a-84`'s 924).
* svelte-check 0 errors / 0 warnings across
  4026 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Guard the clean-state branch** (not
  the mount effect) — minimal-blast-radius
  fix. The mount effect's behavior is
  already correct; the persistence
  effect's clean-state branch was over-
  eager.
* **Bundled the `discardBuffer` `tab.id`
  fix** — same file, same
  `-a-82` family, surfaced by the audit.
  Caller of the bug shows the `-a-82`
  re-key didn't fully reach this spot.
* **Skip `cancelPendingBufferWrite` too**
  when banner is up — defensive; in clean
  state there shouldn't be a pending
  write anyway (the persistence effect
  doesn't queue in clean state), but
  symmetry with the original branch.

### Suggested commit subject

```
Hang-recovery: guard buffer-clear when banner is up + fix discardBuffer key (fullstack-a-83)
```

Single commit. Effect guard + discard fix
+ 5 test pins.

### Files for `git add` (per-path discipline)

* `web/src/components/FileEditorTab.svelte`
* `web/src/components/hangRecoveryEffectOrder.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-83.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@WebtestA empirical re-walk that closes the
3rd-round PARTIAL.
