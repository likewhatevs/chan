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
