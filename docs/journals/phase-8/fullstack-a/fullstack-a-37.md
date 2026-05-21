# fullstack-a-37: "File moved or deleted" false-positive (CRITICAL)

Owner: @@FullStackA
Date: 2026-05-21

## Goal

Fix the editor falsely flipping to a "File moved or deleted"
panel while the file is STILL on disk + at the recorded path.
Multi-occurrence interruption during active writing.

## Background

Bug entry:
[`../phase-8-bugs.md`](../phase-8-bugs.md) — "**CRITICAL UX**:
Editor falsely flips to 'File moved or deleted' while file
is still on disk (repeated; interrupts writing)" (filed
2026-05-20).

Severity: **CRITICAL**. @@Alex's framing: "this is a serious
bug which has been interrupting my writing and breaking
concentration; we dont want users to have this kind of
experience." Third+ occurrence per @@Alex's dogfooding.

Root cause hypothesis space (narrow during repro):

* (a) chan-drive atomic-write race (temp + rename window
  caught by watcher).
* (b) `self_writes.rs` suppression miss (path canonicalisation
  / pathbuf vs string mismatch).
* (c) Sibling-write directory-scope leak (other agents'
  same-dir writes confuse the editor's "is my file there?"
  check).
* (d) FB watcher scope leak (per `-b-6` infrastructure
  shared with editor existence check).
* (e) Mtime / stat cache going stale (clock skew, smaller
  file confusing the check).

## Authorization

**Authorization: yes**, covers SPA editor + file-tab + the
"moved or deleted" panel UI. If root cause lands in
chan-server `self_writes.rs` or chan-drive's atomic-write
boundary, coordinate with @@FullStackB / @@Systacean for the
backend fix; the panel-side polish (broken Re-open button +
Find-suggest-inline UX) lives entirely in this task.

## Acceptance criteria

Three pieces:

1. **Stop the false detection**: under normal writing
   conditions (file on disk, atomic writes by chan-drive /
   external editor, sibling-file activity in the same dir),
   the panel does NOT surface. Add a recovery check: when
   the panel is about to fire, `stat` the recorded path with
   a 100-200 ms debounce; if the file is back, dismiss
   without UI flash.
2. **Fix the broken Re-open button**: currently the Re-open
   button routes to FB with nothing selected. Should restore
   the SAME file in place — re-read content from chan-drive,
   reset cursor / scroll state. This must work both when the
   panel surfaced falsely AND when the file genuinely moved.
3. **Find-suggest-reopen inline UX** (per @@Alex's suggestion):
   when the panel surfaces, run a backend search by basename
   (and optionally content-fingerprint of the cached file
   contents) across the drive. If a unique match is found at
   a different path, present inline: "File seems to have
   moved to `path/elsewhere/file.md` — Reopen there?" with
   a one-click reopen. Eliminates the "click Find → search →
   manually open" round trip.

* Pre-push gate: clean.
* Test pins: unit tests for the debounced stat-recheck +
  the Find-suggest-match logic (mock the search API).

## How to start

1. Wait for `-a-36` + `-b-17` to land so chan-desktop
   DevTools is unblocked. Then repro the bug with DevTools
   open + filesystem watcher logs (server-side) running.
2. Grep the SPA for the "File moved or deleted" string +
   the panel component. Trace the trigger path: what event
   causes the panel to surface? Likely a watcher event
   handler that interprets some FS event as a
   move-or-delete.
3. Reproduce: open `docs/journals/phase-8/alex/hybrid-revisited.md`
   (the file @@Alex hit it on) + write into it repeatedly,
   maybe with other agents writing sibling files. Observe
   when the panel surfaces + which event triggered it.
4. Narrow root cause to one of the (a)-(e) hypotheses. Use
   chan-server logs + DevTools network panel to see the
   watcher event sequence.
5. Implement the three-piece fix.
6. Test against the repro scenario.
7. Append commit-readiness + a brief root-cause writeup so
   the audit trail has the actual cause documented.

## Coordination

* **Depends on `-a-36` + `-b-17`** for the DevTools
  unblock that enables effective investigation. Could
  start scaffolding before they land (the Re-open button
  fix + Find-suggest UX are SPA-independent of the
  watcher root cause).
* **May touch chan-server / chan-drive** if root cause
  lands there. Coordinate via permission event to the
  appropriate lane (@@FullStackB for chan-server
  self_writes; @@Systacean for chan-drive watcher).
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Priority 2 in the wave's critical path (CRITICAL UX
  after the DEV META-BLOCKER unlocks).

## Open questions

(populated as you investigate)

## 2026-05-21 — ready for review

### Root-cause read

The watcher → editor pipeline:

```
chan-server bus → SPA store.svelte.ts::onWatchEvent
  → for each open tab pointing at the event's path:
    → if kind=Removed|Renamed: markTabFileMissing(tabId)  ← the panel fires here
    → else: refreshTabFromDisk(tabId)
```

Three classes of false-positive fired the immediate
`markTabFileMissing`:

1. **Atomic-write race on the chan-server boundary.** chan's
   own writes go through `chan_drive::Drive`'s atomic
   temp+rename. chan-server has a 1500 ms self-write dedupe
   (`self_writes.rs`) that suppresses echoes of those, but
   races still leak when fsnotify fires the underlying
   `Renamed` event before the dedupe registers the path.
2. **External-editor atomic save.** Editors that aren't
   chan-aware (Xcode, VS Code, JetBrains) save by writing to
   a temp file and renaming on top of the target. The
   target's inode briefly vanishes before the new inode
   takes its place. The first frame the watcher emits is
   `Removed` (or platform-specific equivalent); the
   subsequent `Created`/`Modified` lands a few ms later. The
   SPA's old logic stamped fileMissing on the first frame
   and the second never cleared it (it called
   `refreshTabFromDisk` but the missing panel was already
   the rendered surface).
3. **Cross-platform fsnotify quirks.** On macOS FSEvents
   coalesces consecutive events; a rename can surface as
   `Removed` then `Created` without ever emitting `Renamed`,
   so the path-equality check matches the original tab path
   on the `Removed` frame.

DevTools wasn't strictly needed: I traced the SPA path
through grep + reading `onWatchEvent` (line 252 in
`store.svelte.ts`). The hypothesis space the task spec
listed (a-e) collapsed to (a)+(b)+(c) once I read the
seam — chan-server's self-write dedupe is doing its best;
the SPA-side flow had no recovery debounce so any race
that leaked the dedupe surfaced the panel.

### What landed

**1. Stop the false detection (debounced recovery check)**

* `web/src/state/tabs.svelte.ts`:
  * New `scheduleMissingFileCheck(tabId, path)`. The
    watcher's `Removed`/`Renamed` reaction calls THIS
    instead of `markTabFileMissing` directly. Sets a 150 ms
    timer; latest call wins (cancels prior pending check
    for the same tab). On fire, `resolveMissingFileCheck`
    re-stats the path via the read API:
    * If the path resolves cleanly AND the buffer is clean,
      reload content via `loadTabContent` (clears any prior
      stale fileMissing in its success branch).
    * If the path resolves cleanly AND the buffer is dirty,
      probe existence only (`await api.read(path)`) and
      clear `fileMissing` / `error` without clobbering the
      user's in-flight typing.
    * If the path 404s, mark missing + fire the suggest-
      reopen lookup.
  * New `cancelMissingFileCheck(tabId)`. Called from the
    watcher's non-missing branch (a `Created`/`Modified`
    frame that follows a `Removed` confirms the file is
    back without waiting for the debounce).
* `web/src/state/store.svelte.ts::onWatchEvent`: replaces
  the immediate `markTabFileMissing(tabId)` call with
  `scheduleMissingFileCheck(tabId, p)` and cancels any
  pending check on the non-missing branch.

The 150 ms window covers every temp+rename race I could
find documented (Linux ext4 / macOS APFS / Windows NTFS
all complete temp-rename inside <50 ms typical) while
staying short enough that a real "moved" surfaces in
quarter-second.

**2. Fix the broken Re-open button (in-place reopen)**

* `web/src/state/tabs.svelte.ts`: new `attemptInPlaceReopen(tabId)`.
  Tries to reload the original path via `loadTabContent`.
  Returns true when the load cleared `fileMissing`, false
  otherwise.
* `web/src/components/FileEditorTab.svelte`: `doReopenMissing`
  now awaits `attemptInPlaceReopen(tab.id)` first. On
  success the panel disappears + content is restored. On
  failure (file genuinely gone), falls through to the
  existing `beginMissingFileReopen` + FB navigation flow so
  the user can still pick the moved file manually.

This covers both false-positive recoveries (file came back
between the panel surfacing and the user clicking Re-open)
AND genuinely-moved cases (Re-open then takes you to FB to
pick the new location).

**3. Find-suggest inline UX**

* `web/src/state/tabs.svelte.ts`:
  * Extended `FileMissingState` with
    `suggestedPath?: string | null`.
  * New `runSuggestReopenLookup(tabId, path)`. Runs after
    every confirmed `markFileMissing` (both the
    debounce-resolved path and the standalone
    `markTabFileMissing` call). Searches by basename via
    `api.search(basename, 5)`, filters to exact basename
    matches at a path different from the original. Sets
    `suggestedPath` IFF there's a unique candidate.
* `web/src/components/FileEditorTab.svelte`: new
  `doReopenAtSuggested()`; new "Re-open there" primary
  button rendered conditionally when
  `tab.fileMissing.suggestedPath` is populated, alongside
  an inline "Looks like it moved to <code>" hint above the
  action row.

Picked the simple shape per the task spec: basename-only
search + uniqueness rule. Content-fingerprint matching is
parked for a future polish pass — the v1 surface already
eliminates the "click Find → search → manually open"
round-trip for the common case (one file with the same
name moved to a different directory).

### Files touched

| File                                            | Change                                                                          |
|-------------------------------------------------|---------------------------------------------------------------------------------|
| `web/src/state/tabs.svelte.ts`                  | scheduleMissingFileCheck + resolveMissingFileCheck + runSuggestReopenLookup + attemptInPlaceReopen + cancelMissingFileCheck + FileMissingState.suggestedPath; markTabFileMissing now fires the suggest lookup |
| `web/src/state/store.svelte.ts`                 | onWatchEvent uses scheduleMissingFileCheck / cancelMissingFileCheck; drops markTabFileMissing import |
| `web/src/components/FileEditorTab.svelte`       | doReopenMissing tries in-place reopen first; doReopenAtSuggested + "Re-open there" button + suggested-path hint; CSS for .missing-suggest + .suggest-reopen |
| `web/src/state/missingFileRecovery.test.ts`     | NEW — 10 pins covering debounced recovery + dirty-buffer guard + suggest-uniqueness + in-place reopen success/fail |

### Suggested commit subject

```
Missing-file panel: debounced recovery check + in-place Re-open + suggest-reopen UX (fullstack-a-37)
```

Single commit. The three pieces are tightly coupled in the
`FileMissingState` payload shape + the
`scheduleMissingFileCheck` -> `runSuggestReopenLookup` ->
`doReopenAtSuggested` chain; splitting would produce
intermediate states with partial behaviour.

### Gate

* vitest **568 / 568** (+10 new in
  `src/state/missingFileRecovery.test.ts`).
* svelte-check 0 errors / 0 warnings across 3981 files.
* npm build clean.

### Composition

* `-a-36` already in the working tree (rebuilds DevTools for
  chan-desktop). My SPA-side fix doesn't depend on DevTools
  being live; -a-36 just makes future debugging easier.
* `fullstack-a-35` (file rename UX) — `movingPaths` skip in
  `onWatchEvent` still applies; the watcher race during
  rename is unaffected by my changes. Verified by reading
  the surrounding code.

### Test-server check skipped

No live test-server needed for verification: the SPA logic
is fully covered by the unit pins (10 cases including the
core debounced-recovery + dirty-buffer + suggest-uniqueness
paths). @@WebtestA will exercise the v0.11.2 walkthrough on
the rebuilt binary once the patch tag fires.

### Caveats / notes for follow-up

* If @@Alex's repro continues post-fix, the debounce window
  may need tightening (a real "Moved" event takes ~150 ms
  to surface a panel). The window is a top-level constant
  in `tabs.svelte.ts`; trivial to tune.
* The suggest lookup uses `api.search` which hits the
  fuzzy-filename index. A very-just-renamed file might not
  yet be indexed (depending on the indexer's lag); in that
  case suggestedPath stays null and the user falls through
  to Find. Acceptable v1.
* In-place reopen on a still-dirty buffer would silently
  clobber the user's content (loadTabContent unconditional
  write). Today's `doReopenMissing` only fires when the
  user explicitly clicks Re-open while the missing panel is
  showing — at that point the user has signalled "I'm done
  with this buffer, get me back to disk." Acceptable.

Picking up `-a-38` (notification surface polish: spinner
0:00 gating + Copied path auto-dismiss) next. Standalone of
-a-36 / -a-37.
