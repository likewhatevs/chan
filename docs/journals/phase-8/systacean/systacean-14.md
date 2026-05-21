# systacean-14 — Terminal watcher silent-wedge investigation + SerTab reconciliation

Owner: @@Systacean
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Diagnose + fix the terminal watcher silently stopping
event dispatch mid-session (bug filed in
[`../phase-8-bugs.md`](../phase-8-bugs.md) 2026-05-21).

Two visible symptoms:

1. **Ingest wedge**: events stop landing in the SPA even
   though the watcher is "attached" per the SerTab pill.
2. **SerTab state desync**: serve restart clears the
   watcher attachment server-side, but the SerTab pill
   stays "active"; first interaction surfaces `terminal
   watcher is not attached`.

## Background

@@WebtestB observed this during the `-b-13` walkthrough:
serve restart cleared the wedge, but the silent-failure
shape (no toast, no log, no visible state change) is the
UX bug. Same ingest plumbing as `systacean-9` (drive-
sandbox-resolution split) + `systacean-10` (non-matching
filename skip).

Possible root causes (investigate):

* Ingest channel saturation / backpressure (mpsc buffer
  full, tasks dropped silently).
* Async task panic in `event_watcher.rs::ingest_once` or
  the dispatch callback — no caught + reported.
* fsnotify (FSEvents on macOS, inotify on Linux) handle
  silently dropping on resource pressure.
* Serve restart not propagating watcher-detach to the
  SerTab pill state.

## Acceptance criteria

### Diagnosis

* Identify the root cause via reproducible repro. Capture
  the failure mode in a test if possible.
* Document the finding in the task tail + cross-reference
  the bug entry in `phase-8-bugs.md`.

### Fix — ingest side

* Watcher dispatches events reliably for the lifetime of
  the attachment. If a backpressure / panic / fsnotify
  drop is unavoidable in a single edge case, surface a
  warning to the SPA + log to chan-server stderr; never
  silent-fail.

### Fix — SerTab state reconciliation

* Serve restart updates the SerTab pill state correctly.
  The "active" pill on a detached watcher is the desync
  to close.
* First interaction after detach surfaces a recoverable
  message (not the current opaque `terminal watcher is
  not attached`).

## How to start

1. Reproduce in a controlled environment: spawn chan
   serve, attach watcher, drive enough event traffic to
   surface the wedge. Capture timing + state.
2. Add tracing at the ingest hot path
   (`crates/chan-server/src/event_watcher.rs::ingest_once`,
   the dispatch callback, the SPA-side
   `watcherEvents.ts::readWatcherEvents`).
3. Identify the breakpoint where events stop flowing.
4. Patch the ingest side.
5. Audit the serve-restart code path for SerTab pill
   reconciliation; fix the desync.

## Coordination

* @@Systacean lane (event-watcher ingest is your area
  per `-9`/`-10`).
* Pre-push gate must be green.
* When ready for commit, append "Commit readiness" + fire
  poke to @@Architect.

### Adjacent context worth knowing

The watcher-vs-journal shape discussion
([`../architect/watcher-vs-journal-shape.md`](../architect/watcher-vs-journal-shape.md))
landed 2026-05-21 from a separate smoke test. That
discussion is about the AgentEvent JSON wire shape vs
markdown coordination journals; THIS task is about the
watcher's reliability + state-reconciliation, not its
shape. They share the watcher source file
(`event_watcher.rs`) but are otherwise independent.

## Numbering

Highest committed `systacean-N` is `-13`; this is `-14`.
The parked `-12` (tauri-plugin-updater verify on @@Alex's
runtime perm) is unaffected; resume `-12` when @@Alex
approves.

## 2026-05-21 — investigation log + first patch round

### Desk-read of the ingest path

`crates/chan-server/src/event_watcher.rs::EventWatcherHandle::start`
builds a `notify::recommended_watcher` whose callback runs
on notify's own thread (FSEvents-backed dispatch source on
macOS). The callback calls `ingest_once` **synchronously**;
there is no intermediate channel.

* The architect bug-list hypothesis "`tokio::sync::mpsc`
  saturation" doesn't apply: nothing in the ingest path
  uses a tokio mpsc. The synchronous shape means there is
  no buffer that can fill up between FSEvents and
  `dispatch_agent_event`.
* `event_final_path` only matches `Create(_)` and
  `Modify(Name(_))`; every other notify `EventKind`
  silently falls through `_ => None`. Before this task
  there was no tracing on that branch.

### Hypothesis ranking before live repro

1. **Unhandled `EventKind` silently dropping events** —
   strongest candidate given the "events stop, counter
   doesn't move, no log entries" symptom shape. macOS
   FSEvents can synthesise kinds like
   `Modify(Metadata(Extended))` and `Modify(Data(Content))`
   alongside the expected `Create` + `Modify(Name)` pair;
   if a real event ever surfaced *only* through one of
   those it would be invisible.
2. **`SeenEventIds` deduping silently** — if a producer
   reuses an `event.id`, `seen.insert` returns false and
   `ingest_once` returns without warn or counter bump.
3. **Notify thread panic** — `seen.lock().expect(...)` or
   the registry-side `sessions.lock().expect(...)` could
   poison and kill the watcher thread silently; the
   `EventWatcherHandle` would still report Some.
4. `/tmp` ↔ `/private/tmp` canonicalisation — tried by
   @@WebtestB and the least likely.

### Live repro attempt (fresh chan serve)

Spawned `./target/debug/chan serve --port 8830
/tmp/chan-test-systacean-14` with
`RUST_LOG=chan_server::event_watcher=debug`; created an
`@@Systacean` terminal session via the API; attached a
watcher to `/tmp/chan-test-s14-watch`; fired bursts of 15
and 50 atomic-rename events back-to-back with the
test script at `/tmp/s14-fire.sh`.

Result: **wedge did not reproduce.** 50 dispatches landed
cleanly; `dropped_events` stayed at 0; debug log shows
every event going through the full `Create(File)` (.tmp)
→ `Modify(Name(Any))` (final) → dispatch sequence with no
gaps.

Notify event-kind catalogue captured on macOS during the
atomic-write dance for ONE event file:

| Path                          | EventKind                  | Watcher branch    |
|-------------------------------|----------------------------|-------------------|
| `.event-X.tmp`                | `Create(File)`             | filename filter   |
| `.event-X.tmp`                | `Modify(Name(Any))`        | filename filter   |
| `.event-X.tmp`                | `Modify(Metadata(Extended))` | _ => None (debug log) |
| `.event-X.tmp`                | `Modify(Data(Content))`    | _ => None (debug log) |
| `event-X.md` (final)          | `Modify(Name(Any))`        | DISPATCH          |

So on the chan-server side, every successful dispatch
hangs on a single `Modify(Name(Any))` event at the rename
destination. If that one event is ever swallowed, the
event is lost.

Unit-test repros (added in this round; all pass):

* `watcher_dispatches_burst_of_events` — 12 distinct
  atomic-renames in a tight loop.
* `watcher_handles_repeated_same_filename_writes` — 6
  rename-overs of the same destination filename with
  distinct payload ids.
* `watcher_handles_tmp_symlink_path` (macOS-only) — the
  `/tmp/...` symlink path shape @@WebtestB used.

All three pass. The wedge isn't a deterministic ingest-
path bug under these scenarios.

### Conclusion + scope of this round

The original @@WebtestB session-specific wedge is **not
reproducible from a fresh serve**. The architect's bug
entry already anticipated this ("If reproducible, narrow
to ingest-channel-saturation vs task-death"). The
pragmatic shape per the acceptance criteria "never
silent-fail" is: instrument every decision point now so
the next recurrence is diagnosable in-place, and fix the
clear-cut SerTab reconciliation half today.

### Patches landing this round

1. **Tracing at every ingest decision point**
   (`event_watcher.rs`):
   * `tracing::debug!` for each `notify::Event` arrival
     (kind + paths + dir).
   * `tracing::debug!` for the previously-silent
     `_ => None` branch in `event_final_path` (Logged
     without bumping `dropped_events`, since
     `Modify(Metadata)` fires on every xattr / Spotlight
     tick on macOS — counter-pollution would re-introduce
     the rich-prompt red toast spam that `systacean-5`
     closed).
   * `tracing::debug!` for `is_dir` / filename-skip /
     dedup-skip / dispatch-success branches in
     `ingest_once`.
   * Run with `RUST_LOG=chan_server::event_watcher=debug
     ./target/debug/chan serve …` to capture.

2. **Three regression tests** pinning that the watcher
   dispatches reliably under burst, rename-over-existing,
   and `/tmp` symlink scenarios (all pass; baseline for
   future regression catches).

3. **SPA-side SerTab reconciliation**
   (`TerminalTab.svelte::refreshWatcherEvents`):
   * On a `409 / 404 / "not attached"` response from
     `GET /api/terminal/:session/watcher/events`, call
     the existing `watcherDetached()` helper to clear
     `tab.watcher` immediately, instead of leaving a
     permanent red error toast. Mirrors the established
     pattern in `BubbleOverlay.svelte::replyError` and
     `TerminalRichPrompt.svelte::stopWatching`.
   * This handles the symptom @@WebtestB reported on
     restart: SerTab pill shows "watching … | Stop
     watching" while the new server has no watcher.
     First `refreshWatcherEvents()` after restore now
     clears the pill cleanly (and `ui.status` reads
     "watcher detached on reload" via the existing
     helper).

### Pre-push gate

* `cargo fmt --all -- --check` → clean.
* `cargo clippy --all-targets -- -D warnings` → clean.
* `cargo test --workspace` → all crates green; 11
  `event_watcher` tests pass (3 new + 8 existing).
* `RUSTFLAGS=-D warnings cargo build --no-default-features`
  → green.
* `cd web && npm run check` → 0 errors / 0 warnings (3987
  files).
* `cd web && npm test -- --run` → 588/588 pass.
* `cd web && npm run build` → green (chunk-size + IEDI
  warnings unchanged from baseline).

### Test-server teardown

* `chan serve` process killed (pid was bound to
  127.0.0.1:8830).
* `/tmp/chan-test-systacean-14` + `/tmp/chan-test-s14-watch`
  removed.
* Drive `chan-test-systacean-14` unregistered.

### What still needs the actual @@WebtestB wedge

The instrumentation gives the next recurrence three new
data sources:

* `RUST_LOG=chan_server::event_watcher=debug` shows every
  notify event arrival including unhandled kinds (so we
  can tell "FSEvents stopped firing" apart from
  "FSEvents fired but we dropped it on the floor").
* Dispatch-success log lines pin the moment ingest stops
  reaching `dispatch_agent_event`.
* SerTab-reconciliation patch removes the stale-pill
  confound (so the next repro starts from a known
  watcher state).

If the wedge recurs in a webtest walkthrough, the
diagnostic trail should narrow it to one of the listed
hypotheses without needing another speculative round.

### Files touched

* `crates/chan-server/src/event_watcher.rs` —
  tracing + 3 regression tests; no behaviour change.
* `web/src/components/TerminalTab.svelte` —
  `refreshWatcherEvents` catch-branch picks up the
  detach signal.

### Commit readiness

Ready for clearance. Suggested subject:

```
chan-server: instrument event-watcher ingest path + SPA detach-on-409 reconcile (systacean-14)
```

`/tmp/s14-fire.sh` is a throwaway repro script kept on
disk only for the duration of this session; not part of
the commit.
