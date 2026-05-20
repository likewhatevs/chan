# systacean-5: chan-server event_watcher emits "Is a directory" error on freshly-created watch root

Owner: @@Systacean
Date: 2026-05-20

## Goal

`chan_server::event_watcher` should not attempt to read the
watch root as if it were an event-file journal. Today it
does, and emits
`failed to read event file <path>: Is a directory (os
error 21)` whenever a freshly-created (empty) watch root is
attached. @@WebtestB caught this as a red toast top-right
in the lane-B walkthrough of `fullstack-b-3`.

## Background

@@WebtestB's wave-1 verification of `fullstack-b-3`
flagged this as a side observation (not blocking -3, but
worth a separate task):

> server-side `chan_server::event_watcher` emits
> `failed to read event file <path>: Is a directory (os
> error 21)` when the watch root is a freshly-created
> empty directory. Surfaces as a red toast top-right on
> first attach to a brand-new outside-drive dir (case a);
> quieter for in-drive new dir (case b — toast did not
> surface); absent for an existing dir with files
> (case c — `docs`). Likely the watcher polls the watch
> root as if it were an event-file journal.

User-visible symptom: red error toast on a clean
"attach watcher to new directory" operation, which is the
exact UX the `fullstack-b-3` fix was designed to enable.
Functional impact: the watcher still attaches and works,
but the toast is alarming and incorrect.

## Acceptance criteria

* Attaching the watcher to a freshly-created (empty)
  directory does not emit the "Is a directory" error.
* Attaching the watcher to a directory with files
  continues to work (no regression on case c).
* Attaching the watcher to a directory inside the drive
  vs outside the drive behaves the same (case a vs b
  consistency).
* If there are legitimate "read event file" calls that
  should target real event-file paths inside the watch
  root, those still fire correctly — the fix is about
  not treating the watch root itself as an event file.
* No new tests required beyond a unit test pinning
  "attach to empty dir produces no error stream
  notifications" if the event_watcher module has a
  testable seam.

## How to start

1. Find `event_watcher` in `crates/chan-server/src/`. The
   module is the watcher-event stream the rich prompt
   reads from. Likely in
   `crates/chan-server/src/event_watcher.rs` or a similar
   path.
2. Trace the path from "watcher attached to <root>" to
   the failing `read_to_string` (or similar) call. The
   error string verbatim is "failed to read event file
   <path>: Is a directory" — that lets you grep cleanly.
3. Likely root causes:
   * The poll loop enumerates the watch root and tries to
     read every entry, including the root itself if it
     somehow ends up in the entry list.
   * A path-join bug that produces the root path verbatim
     instead of root + child filename, then attempts a
     file read on the directory path.
4. Fix the enumeration / path-join. If the watch root is
   itself in the entry list erroneously, filter it out.
5. @@WebtestB has a reproducer set up on lane-B
   (`/tmp/chan-watch-wb-outside/` exists outside the
   drive, `newdir-wb-missing/` exists at the lane-B drive
   root). Either drive reproduces with a fresh empty dir.

## Coordination

* @@WebtestB verifies on lane-B drive once landed.
* Coordinate with `systacean-4` (directory-typed-as-file
  in indexer) on sequencing if both share an indexer
  walker abstraction — unlikely (`event_watcher` is the
  rich-prompt watcher, `systacean-4` is the graph
  indexer), but flag if they touch the same module.

## 2026-05-20 — root cause + fix

### Root cause

`crates/chan-server/src/event_watcher.rs::ingest_once` reads
every notify-delivered path as if it were an event-file
payload via `std::fs::read_to_string(&path)`. On macOS,
FSEvents (backing notify's `RecommendedWatcher`) emits a
synthetic `Create` event whose `path` is the watch root
itself on first attach to a freshly-created dir. Reading a
directory with `read_to_string` errors with EISDIR ("Is a
directory (os error 21)").

The error branch logs a `tracing::warn!`, increments
`dropped_events`, and the
`terminal_event_watcher.dropped_events` counter the SPA
reads off `/api/health` ticks up — which surfaces as a red
toast top-right in the rich-prompt UI.

@@WebtestB observed case-a (outside-drive fresh dir) firing
the toast, case-b (in-drive fresh dir) quieter (event
ordering / race window may have masked it), case-c (existing
dir with files) clean (no synthetic root emit because the
dir wasn't freshly-created).

The lane-boundary heuristic from `systacean-4.md` is wrong
for this task: there's no shared abstraction between
`event_watcher` and the graph indexer. `event_watcher` is
notify-driven on the rich-prompt event directory;
`systacean-4`'s fix lived in `api_graph`'s ghost path.
Independent fixes.

### Fix

Single-file change to
`crates/chan-server/src/event_watcher.rs::ingest_once`:
early-return when the delivered path is a directory.

```rust
if std::fs::metadata(&path).is_ok_and(|m| m.is_dir()) {
    return;
}
```

* No log line: a directory event isn't an error condition,
  it's just notify being chatty on macOS. Quieter than a
  warn at debug-level (which would still pile up in busy
  logs); the unit test pins that no event is dispatched.
* No `dropped_events` bump: skipping a synthetic
  root-of-watch event isn't a dropped event; it's a non-
  event. The counter stays meaningful (real
  read_to_string/parse failures still increment).
* `metadata` errors are not treated as directory — they
  fall through to `read_to_string`, which produces a more
  accurate error (path missing, permission, etc.) that is
  worth logging.

Acceptance criteria walk:

* Attaching to a freshly-created empty dir: no EISDIR error
  → ✅ guard fires before `read_to_string`.
* Attaching to a dir with files: existing behaviour
  preserved → ✅ files emit notify events whose paths are
  the file paths, not the dir; guard doesn't trip.
* Inside-drive vs outside-drive consistency: both go
  through the same `ingest_once` → ✅ same behaviour.
* Real event-file paths still fire: the guard only catches
  `is_dir() == true` → ✅ `watcher_dispatches_atomic_rename_once`
  test still passes (event-1.json is a regular file).

### Test

`ingest_once_skips_directory_paths_silently`: direct-calls
`ingest_once` with the watch root and with a subdirectory.
Asserts `dropped_events == 0` and no event dispatched.
Doesn't depend on FSEvents / notify's OS-specific behaviour;
exercises the guard via the testable seam.

### Gate

* `cargo fmt --check` — clean.
* `cargo clippy -p chan-server --all-targets -- -D warnings`
  — clean.
* `cargo test --all` — green; `event_watcher::` module
  4 → 5 tests.

End-to-end verification on lane-B's repro setup is on
@@WebtestB once the rebuilt binary is in place. The mechanistic
guard is pinned by unit test; the rich-prompt toast should
stop firing.

### Status

Committed as `80a34ee`:

```
event_watcher: skip directory paths instead of treating them as failed event-file reads (systacean-5)
```

Single-file commit
(`crates/chan-server/src/event_watcher.rs`, +58 / -0). Push
held for Round-1 close per the standing systacean-3 plan.

## 2026-05-20 — @@Architect: approved + cleared (already committed)

Reviewer: @@Architect.

Tight root-cause. macOS FSEvents emitting a synthetic
`Create` for the watch root itself on first attach is
exactly the kind of OS-shim behaviour that's invisible
from a Linux dev box and only surfaces under @@WebtestB's
case-a (outside-drive fresh dir) repro. Reading the path
with `read_to_string` then erroring on EISDIR is the
visible symptom; correctly identifying the synthetic root
emit as the source is the load-bearing piece.

Fix is right-sized:
* Early-return on `is_dir() == true` is targeted.
* No log line — a directory event isn't an error, it's
  notify being chatty. Silent is the right level.
* No `dropped_events` bump — skipping a non-event isn't a
  drop; the counter stays meaningful for real failures.
* `metadata` errors fall through to `read_to_string` for
  a more accurate diagnostic if a real path issue exists.

`ingest_once_skips_directory_paths_silently` pins the
contract without OS-dependence. The
`watcher_dispatches_atomic_rename_once` regression check
proves the guard doesn't catch legitimate file events.

Confirmed: `event_watcher` and the graph indexer share no
abstraction; the systacean-4 lane-boundary heuristic was
wrong but the impact was zero (both are independent fixes,
no merge conflict possible).

**Cleared (already committed)**: `80a34ee`. Push waits for
Round-1 close per the standing rule.