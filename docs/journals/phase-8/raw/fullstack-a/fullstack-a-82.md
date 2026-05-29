# fullstack-a-82 — Hang-recovery banner STILL not surfacing empirically (follow-up to -a-74)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: HIGH (data-loss prevention)

## Goal

Re-investigate why the hang-recovery restore banner
still doesn't surface empirically on force-reload,
even after `-a-74`'s `beforeunload` + `pagehide`
flush fix.

## Reference

@@WebtestA's triple proactive walk (`1e44d40`)
verdict: **STILL PARTIAL**.

`-a-74` shipped the synchronous flush via
`beforeunload` + `pagehide` listeners in App.svelte
(+28 lines). 18 vitest pins green. Mechanism-verified.

But empirically: banner still doesn't surface
post-reload. Mechanism passes; the empirical UI flow
fails.

## Hypotheses (audit at pickup)

### H1 — Flush listener fires but write doesn't land

`beforeunload` listener calls
`flushPendingBufferWrites()` synchronously, but:

* `pendingWrites` Map is module-scoped — does it
  actually contain the in-flight write at unload
  time, or is the debounce timer cancelled by some
  earlier teardown path?
* Audit: add `console.warn` (or `tracing` if Tauri-
  side) at the listener entry to confirm it fires
  + log the Map size.

### H2 — Write lands but mount-time read skips

`flushPendingBufferWrites` writes to localStorage.
Mount-time `divergentBufferOrNull` reads from
localStorage. But:

* Is the read happening BEFORE disk content loads,
  comparing against an empty disk content, then
  returning null (because "buffer equals empty disk"
  on the wrong-empty path)?
* Audit: log the divergentBufferOrNull entry +
  what tab.content / tab.saved values are at that
  moment.

### H3 — Banner mount-effect runs but isn't visible

The banner component might be mounting but
positioned off-screen / behind something / with
zero opacity / etc.

* Audit: inspect DOM at the moment the banner
  should appear (via Chrome MCP DOM query); confirm
  `.recovered-banner` element exists + is visible.

### H4 — Force-reload IS triggering Svelte unmount cleanup after all

If `window.location.reload()` DOES trigger
component unmount before page tear-down (contrary
to the `-a-74` audit verdict), the cleanup might
be CANCELLING the pending write before
`beforeunload` fires (since cleanup runs first).

* Audit: log component unmount + listener fire
  ordering with timestamps.

## Diagnostic path

1. Add temporary `tracing::debug` (or `console.warn`)
   at:
   * `beforeunload` listener entry
   * `pagehide` listener entry
   * `flushPendingBufferWrites` entry + per-tab
     write log
   * `divergentBufferOrNull` entry + return value
   * Banner render condition
2. Repro the empirical flow with Chrome MCP +
   inspect console.
3. Verdict → identify which hypothesis fits.
4. Fix based on verdict.

## Acceptance

1. **Force-reload empirically restores**: type
   unsaved → reload → banner surfaces → click
   Restore → content recovered.
2. **Diagnostic logs left in or removed** per
   implementer's call (temporary debugging is OK
   to leave if labeled).

### Tests

If the bug shifts to a new layer (e.g. H4
ordering), add a test pin against the corrected
contract.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* HIGH priority — data-loss prevention.
* Walks @@WebtestA after fix lands.

## Authorization

Yes for editorBuffer + FileEditorTab + App.svelte +
tests + task tail + outbound. Temporary diagnostic
logging encouraged.

## Numbering

This is `-a-82`.

## Out of scope

* Re-architecting the recovery mechanism beyond
  the current `localStorage` + flush pattern.
* Terminal-side recovery.

## 2026-05-22 — ready for review (H1+ root cause: tab-id regeneration)

Four-file change. SPA-only.

### Audit verdict

Root cause is a **5th hypothesis** the task body
didn't list: **tab-id regeneration across
reloads**.

* `web/src/state/tabs.svelte.ts:25`: tab ids are
  generated from a module-level counter
  (`nextId`).
* Module state resets on every page load, so
  `nextId` restarts at 1.
* Pre-reload tabs were assigned `tab-3` /
  `tab-7` / etc.; post-reload they get fresh
  ids (`tab-1`, `tab-2`, ...).
* SerTab payload restores tabs by path + flags
  but does NOT preserve tab.id.

The pre-`-a-82` buffer key was
`chan:editor-buffer:<tab.id>`. So:

1. User opens `notes/a.md` → tab.id = `tab-7`.
2. Types unsaved → debounced write queues.
3. Force-reload → `beforeunload` fires →
   `flushPendingBufferWrites` lands the write
   at `chan:editor-buffer:tab-7`.
4. Reload → SerTab restores tab → tab.id =
   `tab-1` (fresh counter).
5. Mount → `divergentBufferOrNull("tab-1", ...)`
   reads `chan:editor-buffer:tab-1` → null.
6. Banner doesn't surface.

Mechanism passed vitest because the unit tests
passed the SAME id to write + read. The
empirical bug surfaced only across reload.

H2 was a secondary contributor: the persistence
effect runs at mount BEFORE `tab.saved` finishes
loading, so it queues an empty `""` write that
could clobber the restored buffer post-debounce.

### Fix

`web/src/state/editorBuffer.ts`: documented the
key-on-path convention. The module's signature
treats the key as opaque, so no code change
needed there — only the contract update +
caller-side migration.

`web/src/components/FileEditorTab.svelte`:
* All buffer-API calls now pass `tab.path`
  instead of `tab.id`:
  * `divergentBufferOrNull(tab.path, tab.path, disk)`
  * `cancelPendingBufferWrite(tab.path)`
  * `queueBufferWrite(tab.path, content, tab.path)`
  * `clearEditorBuffer(tab.path)`
* New early-return in the persistence effect
  when `tab.saved === undefined` — disk
  content hasn't loaded yet; skip the write
  so a `""` initial value doesn't clobber
  the just-restored buffer after the 500ms
  debounce.
* Comment block documents the tab-id
  regeneration root cause + the disk-load
  race.

`web/src/state/editorBuffer.test.ts`: +2 new
pins documenting the key-on-path contract +
the motivation (a tab-id-keyed buffer is
unreadable after the id regenerates).

`web/src/components/hangRecoveryPathKey.test.ts`
(new): 6 raw-source pins covering all four
buffer API call sites in FileEditorTab using
`tab.path`, the undefined-saved guard, and the
rationale comments.

### Acceptance

1. **Force-reload empirically restores**: type
   unsaved → reload → buffer key is
   `chan:editor-buffer:notes/a.md` (stable);
   on remount the new tab also queries by
   `notes/a.md` → buffer found → divergent →
   banner ✓ (mechanism-verified; @@WebtestA
   walk for empirical confirm).
2. **No regression on `-a-72`/`-a-74`
   primitives**: helper signature unchanged
   (opaque-string key); 18 prior pins remain
   green.

### Gate

* vitest **855 / 855** (+9 net from `-a-81`
  slice 1's 846).
* svelte-check 0 errors / 0 warnings across
  4015 files.
* npm build clean.
* Rust gate not re-run.

(3 unrelated test flakes on first vitest run
— known EmptyPaneCarousel / Pane /
TerminalTab load-contention pattern; cleared
on re-run.)

### Decisions

* **Key on path, not tab.id** — paths are
  stable across reloads (SerTab persists
  them); tab ids are module-counter-derived
  + reset on every page load. Path-keyed
  buffer reads survive the reload + the
  tab-id regeneration.
* **Two tabs same path share a buffer** —
  acceptable edge case. Same file → same
  unsaved-content semantic; whoever mounts
  first reads the banner. The SPA's
  open-by-path dedup at `openInActivePane`
  already prevents most duplicate file tabs.
* **`saved === undefined` early return**
  guards against the disk-load race. Without
  this, an empty initial `tab.content`
  + missing `tab.saved` would cause the
  persistence effect to queue a `""` write
  that races the file fetch.
* **Hypothesis labelling**: the task body
  enumerated H1-H4; my audit found the bug
  outside that list (tab-id regeneration).
  Flagging in the impl note so the
  diagnostic-path framing surfaces the
  actual cause for future readers.

### Suggested commit subject

```
Hang-recovery: key buffer on tab.path (not tab.id) so it survives reload (fullstack-a-82)
```

Single commit. Module doc + caller migration
+ disk-load guard + tests tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/state/editorBuffer.ts`
* `web/src/state/editorBuffer.test.ts`
* `web/src/components/FileEditorTab.svelte`
* `web/src/components/hangRecoveryPathKey.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-82.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@WebtestA empirical re-walk.
