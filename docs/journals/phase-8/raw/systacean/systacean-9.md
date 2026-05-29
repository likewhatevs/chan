# systacean-9: outside-drive watcher read fails with "No such file or directory"

Owner: @@Systacean
Date: 2026-05-20

## Goal

Watching an outside-drive directory must work end-to-end:
attach succeeds AND subsequent event reads from that
directory succeed. Today the attach succeeds (post
`systacean-5` + `fullstack-b-3`'s "accept any path") but
the event-read path errors with `watch read failed: io
error: No such file or directory (os error 2)`.

## Background

@@WebtestB found this during a proactive lane-B walk on
2026-05-20:

> Outside-drive watcher events can't be listed because
> the read path uses the drive sandbox. Repro: open rich
> prompt, click Watch directory, type any absolute
> outside-drive path, OK → pill attaches (no event_watcher
> WARN, systacean-5 is working) but red toast
> `watch read failed: io error: No such file or directory
> (os error 2)` surfaces.

Audit trail at the tail of
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the relevant 2026-05-20 heading.

### Why this is end-of-Round-1 polish, not a `-b-3` blocker

`fullstack-b-3` (watcher dialog: accept any path +
create-if-missing + drop overwrite warning) made the
attach work for outside-drive paths. `systacean-5`
(event_watcher: skip directory paths silently) closed
the EISDIR-on-fresh-empty-dir loophole. Both were
correct fixes for what they targeted. **But** the event-
read path elsewhere in chan-server still applies the
drive-sandbox path-resolution rules, so an absolute
outside-drive path passed to the read endpoint fails
the sandbox lookup → ENOENT.

User-visible symptom: the watcher pill shows attached
(green light) but the user sees a red toast every time
chan tries to list events. The feature looks broken even
though the attach itself is fine.

## Authorization

**Authorization: yes**, this task covers edits to
`crates/chan-server/src/event_watcher.rs` (or
`crates/chan-server/src/routes/` for the watcher-events
read route — wherever the read path lives),
`crates/chan-drive/src/drive.rs` (if the sandbox-
enforcing helper sits in chan-drive), and the
TerminalRichPrompt event-listing wiring if the SPA needs
to thread the outside-drive marker. @@Systacean may
proceed without further in-chat confirmation from @@Alex.

## Acceptance criteria

* Watcher attached to an outside-drive absolute path
  (e.g. `/tmp/chan-watch-wave3-outside/`) successfully
  lists events from that path without a "No such file or
  directory" toast.
* In-drive watch targets continue to work unchanged
  (regression check).
* The read path resolves absolute paths outside the drive
  via direct filesystem reads (not via the drive sandbox);
  in-drive relative paths still route through the
  sandbox. The decision point is whether the path is
  inside the drive root or not — same pattern as the
  attach path's resolver.
* No new security surface: the read path is gated on the
  same trust boundary that allowed the attach in the
  first place (settings-guard or equivalent). If
  attaching to an outside-drive path requires settings-
  level credentials, the read does too.
* Pre-push gate: fmt + clippy `-D warnings` + workspace
  test + svelte-check + npm build.
* Unit test pinning the outside-drive-read happy path
  (mirror shape of the existing event_watcher tests).

## How to start

1. Reproduce on lane-B (fixture is up):
   ```
   /tmp/chan-watch-wave3-outside/  (outside-drive dir)
   ```
   Lane-B server on `127.0.0.1:8820`. Attach via
   TerminalRichPrompt Watch directory → outside-drive
   path → confirm the red toast fires.
2. Trace the read path. The attach path was fixed in
   `fullstack-b-3`'s `resolve_watcher_dir`; the read
   path probably lives in a sibling helper that still
   enforces drive-sandbox path resolution. Find it
   (likely a `Drive::read_dir` or similar called from
   `crates/chan-server/src/routes/`).
3. Apply the same in-drive-vs-outside-drive split: if
   absolute outside-drive path, bypass the sandbox + read
   directly; if in-drive, keep the existing sandbox
   resolution.
4. Coordinate with `systacean-5`'s `event_watcher::ingest_once`
   guard — that fix correctly skips directory paths; the
   read-listing change here is about path resolution, not
   the EISDIR symptom.

## Coordination

* @@WebtestB verifies on lane-B drive once landed (same
  fixture they used to find it).
* Lives in chan-server / chan-drive; no SPA work needed.
* Coordinate with `fullstack-b-3`'s `resolve_watcher_dir`
  pattern — the read path mirrors the attach path's
  sandbox decision.

## 2026-05-20 — scope question for @@Architect

Traced the read path. The frontend's `readWatcherEvents`
(`web/src/state/watcherEvents.ts`) is implemented as:

```ts
const entries = await api.list(dir);          // → GET /api/files?dir=<dir>
for (const file of files) {
  const body = await api.read(file.path);     // → GET /api/files/<file.path>
}
```

Server-side, both `/api/files` handlers route through the
drive sandbox (`drive.list` / `drive.read_text`), which
rejects absolute outside-drive paths at
`fs_ops::validate_rel` → ENOENT.

The "no SPA work needed" constraint in this task body
collides with the most natural fix shapes. Two options
on the table; both have trade-offs:

### Option A — dedicated server endpoint

`GET /api/terminal/:session/watcher/events` looks up the
session's `watcher_dir` (already tracked in
`Registry`'s session state), reads the events directly
(bypassing the drive sandbox), and returns an array of
parsed event payloads.

Pros:
* Clean separation of concerns — watcher reads use the
  terminal-session lane (settings-gated by attach), drive
  reads stay sandboxed.
* Server-side complexity is bounded — one new handler.
* No URL-routing weirdness around absolute paths in axum
  wildcards (the `/api/files//tmp/foo` double-slash case).
* Endpoint shape mirrors the existing
  `/api/terminal/:session/event-reply` pattern.

Cons:
* Tiny SPA change — `readWatcherEvents` switches from
  `api.list + api.read` to the new endpoint, accepting the
  session id as an extra arg. ~5-line diff in
  `web/src/state/watcherEvents.ts`. Violates the task's
  "no SPA work needed" constraint, but only minimally.

### Option B — sandbox bypass in /api/files for watcher paths

Modify `api_list_files` + `api_read_file` to recognise an
absolute path that matches an active watcher's
`watcher_dir` and serve it via direct `std::fs` calls
instead of the drive sandbox.

Pros:
* Strictly no SPA change — matches the task constraint
  verbatim.

Cons:
* Cross-cutting coupling: `routes/files.rs` consults
  `terminal_sessions::Registry` watcher state, weakening
  the per-route security boundary.
* URL routing: axum's `*path` wildcard on
  `/api/files//tmp/foo` (note double slash) needs
  verification — needs tested experimentally and may
  require route-pattern tweaks or middleware.
* The "any watcher attached → /api/files serves outside-
  drive paths" semantic is broader than necessary; option
  A keeps it scoped to the explicit watcher endpoint.

### My recommendation: option A

Option A is the smaller, cleaner change. The SPA-side
diff is essentially a one-liner ("call X instead of Y")
that @@FullStackB (TerminalTab.svelte owner) can land in
the same commit batch. The task's "no SPA work needed"
heuristic was written assuming a clean server-only fix
existed; on inspection it doesn't, and option A is the
closest approximation.

If option A approved: I'll add the endpoint + a small
SPA-side patch (1-file `watcherEvents.ts` change to thread
the session id) in one commit, plus a unit test for the
endpoint. Estimated ~150 LoC across both crates.

If option B preferred: I'll need an extra session
investigating axum's double-slash routing behaviour
before committing to the shape; flagging that uncertainty
upfront.

Holding before any code change pending your call. Both
options keep the existing in-drive watcher path working
unchanged.

## 2026-05-20 — @@Architect: scope answer (Option A approved)

Reviewer: @@Architect.

**Option A approved.** Your reasoning is right on every
point:

* Sandbox-as-security-boundary stays clean.
  `routes/files.rs` keeps its single per-route contract;
  watcher reads route through the terminal-session
  lane (settings-gated at attach time).
* No axum-wildcard / double-slash investigation needed.
  The `/api/terminal/:session/watcher/events` shape
  mirrors the existing `/api/terminal/:session/event-reply`
  pattern.
* The "no SPA work needed" line in my task body was
  speculative + wrong. I assumed the read path was a
  single bypass-able call; on inspection it's a
  composed two-call shape (`api.list` + `api.read`)
  that both route through the sandbox. The architect-
  side audit-trail correction lives in this append.

### Cross-lane authorization

You're authorized to land the SPA-side patch
(`web/src/state/watcherEvents.ts` ~5 lines) in the same
commit as the chan-server endpoint addition. Reasoning:

* The SPA diff is trivial (call X instead of Y) — not
  worth coordinating across two agents for a 5-line
  change that's logically part of one feature.
* @@FullStackA is currently on `-23` (FB dock
  separator); pulling them in would block both lanes.
* The endpoint contract + SPA consumption are the same
  logical change. Splitting them risks landing the
  endpoint without the SPA caller updated (or vice
  versa).

This is a one-time cross-lane crossing for a specific
small change, not a precedent for routine cross-lane
SPA work. Future SPA edits stay in @@FullStackA's
lane.

### Updated acceptance criteria

* Drop "No backend / Rust work in this task" — was
  meant the opposite intent (no SPA work); inverted +
  outdated. The task IS chan-server primary + a small
  SPA edit.
* New endpoint shape:
  `GET /api/terminal/:session/watcher/events` →
  `{ "events": [{...event payload...}, ...] }` (or
  array directly — pick whatever matches the existing
  `/api/terminal/:session/event-reply` shape).
* SPA `readWatcherEvents` consumes the new endpoint;
  drops the `api.list` + `api.read` composition.
* Unit test pins the endpoint serves outside-drive +
  in-drive paths correctly (the session's `watcher_dir`
  is the source of truth; the endpoint reads what
  Registry tells it).

### Estimated 150 LoC

Reasonable. The bulk is endpoint handler + struct +
test scaffold; the SPA diff is the small tail. Go
ahead.

Push waits until end of Round 2 (no Round-1 binary cut
per the restructure).

## 2026-05-20 — implementation + commit

### Server endpoint

New `GET /api/terminal/:session/watcher/events` handler in
`crates/chan-server/src/routes/terminal.rs::api_terminal_watcher_events`:

* Looks up `state.terminal_sessions.watcher_dir(&session)`;
  409 + "terminal watcher is not attached" when no
  watcher is set on the session.
* `tunnel_public` gated (same shape as the rest of the
  terminal-write surface).
* Reads the watcher dir directly via `std::fs::read_dir`
  + `std::fs::read_to_string` — bypassing the drive
  sandbox, which was the source of the ENOENT.
* Filters filenames with a server-side equivalent of the
  SPA's prior regex (`^(event|pre-flight)-.+\.(md|json)$`),
  skips hidden files (matches `event_watcher::ingest_once`).
* Sorts deterministically.
* Returns `Vec<{ path: string, content: string }>` — raw
  content so the SPA's existing `parseWatcherEvent` keeps
  validating the JSON shape; server stays out of the
  WatcherEvent schema.

Wired through `routes/mod.rs` (`api_terminal_watcher_events`
re-export) and `lib.rs::router()` (new route alongside
`/api/terminal/:session/watcher`).

### Server tests

Two new unit tests in `routes/terminal.rs::tests`:

* `is_watcher_event_filename_matches_spa_regex` — pins
  the server-side filename filter against the cases the
  SPA's prior regex matched. Catches drift on either side
  of the boundary.
* `list_watcher_events_reads_outside_drive_dir` — pins
  the lane-B repro case. Tempdir simulating
  `/tmp/chan-watch-wave3-outside/`, writes 2 event files
  + 3 non-event files (one hidden, one wrong-name, one
  nested subdir). Asserts the returned list is the 2
  event files only, sorted, with raw content passed
  through.

### SPA changes (cross-lane authorized in the @@Architect append)

* `web/src/api/client.ts` — new `terminalWatcherEvents(sessionId)`
  binding for the new endpoint.
* `web/src/state/watcherEvents.ts` — `readWatcherEvents`
  signature changes from `(dir: string)` to
  `(sessionId: string)`. Body drops `api.list +
  api.read` composition; calls the new endpoint once and
  parses each entry's content via the existing
  `parseWatcherEvent`. Tightens the docstring with a
  pointer to the systacean-9 root cause.
* `web/src/components/TerminalTab.svelte::refreshWatcherEvents`
  — call site updated to pass `tab.terminalSessionId`
  instead of `tab.watcher.path`. Early-returns if the
  session id isn't set yet (mirrors the existing
  `if (!tab.watcher) return` guard shape).
* `web/src/state/watcherEvents.test.ts` — updated test
  mocks `api.terminalWatcherEvents` instead of
  `api.list` + `api.read`. Single-entry case mirrors
  the outside-drive shape (`/tmp/chan-watch-wave3-outside/`
  path). Asserts the new endpoint was called with the
  session id.

### Security envelope (per @@Architect's reply)

The new endpoint reads from `Registry::watcher_dir`, which
is populated by `api_set_terminal_watcher`. That set
endpoint is `tunnel_public`-gated; an anonymous tunnel
viewer can't attach a watcher. The new read endpoint
shares that gate. Same trust boundary as the existing
`event-reply` endpoint, which writes to the same
`watcher_dir` for survey replies.

### Gate

* `cargo fmt --check` — clean.
* `cargo clippy -p chan-server --all-targets -- -D warnings` —
  clean.
* `cargo test --all` — green; `chan-server` test count
  185 → 187 (+2 new from this task). `chan-drive` count
  unchanged.
* `web/npm run check` (svelte-check) — 0 errors, 0
  warnings.
* `web/npx vitest run state/watcherEvents` — 5 tests pass
  (4 existing + 1 rewritten).
* `web/npm run build` (vite) — successful.

### Status

Committed as `c69e2fc`:

```
Watcher events: dedicated /api/terminal/:session/watcher/events endpoint (systacean-9)
```

7 files (`crates/chan-server/src/lib.rs`,
`crates/chan-server/src/routes/mod.rs`,
`crates/chan-server/src/routes/terminal.rs`,
`web/src/api/client.ts`,
`web/src/state/watcherEvents.ts`,
`web/src/state/watcherEvents.test.ts`,
`web/src/components/TerminalTab.svelte`),
+232 / -44. Pre-commit `git diff --staged --stat` audit
clean.

@@WebtestB to re-verify on lane-B against
`/tmp/chan-watch-wave3-outside/` once the rebuilt binary
+ rebuilt web bundle are in place (frontend bundle is
embedded at build time, so chan-server needs a full
`cargo build` after `npm run build` to pick up the new
SPA caller).

## 2026-05-20 — @@Architect: approved + cleared (already committed)

Reviewer: @@Architect.

Clean Option A implementation. The endpoint shape
mirrors `/api/terminal/:session/event-reply` per the
existing pattern; `tunnel_public`-gating shares the
same trust boundary as the watcher-set endpoint; server-
side filename filter pinned by `is_watcher_event_filename_matches_spa_regex`
so any drift between SPA + server filenames trips a
test instead of a runtime ghost.

The `Vec<{path, content}>` shape returning raw content
keeps the WatcherEvent schema parsing on the SPA side
(no schema drift across the boundary). Good.

SPA cross-lane edit landed in the same commit per my
authorization. Five files of SPA + Rust changes total,
+232 / -44. Pre-commit audit clean.

The "rebuilt binary needs `npm run build && cargo
build`" reminder for @@WebtestB is correct hygiene —
rust-embed bakes the SPA bundle at compile time, so
verifying SPA-side changes requires the full chain.

**Cleared (already committed)**: `c69e2fc`. Push waits
until end of Round 2.

Round-1 close gate status: this was the last
@@Systacean Round-1 task. You're queue-empty for the
remainder of Round 1. Standby until Round-2 fan-out.