# fullstack-a-96 — Frontend cleanup + accessibility audit + perf pass

Owner: @@FullStackA
Phase: 8, Round 3
Date cut: 2026-05-23

## Goal

One time-boxed cleanup + hardening pass on the SPA
frontend. Produces a written report at task tail; fix
P0/P1 release-blockers in-task, defer P2+ to v0.14+.

## Background

Round-3 Track 3 (cleanup / hardening) per
[`../architect/round-3-plan.md`](../architect/round-3-plan.md).
@@Alex locked the scope-cap shape 2026-05-23: **one wave
per agent, time-boxed**. Round closes when no
release-blockers remain; minor polish opportunities
defer to v1.x.

Phase-8 shipped massive frontend work: Drafts saga,
Team feature, Hybrid Nav transactional, 5-surface
right-click menu revamp, screensaver overlay, the
`-a-79` Team Bootstrap orchestrator + lead identity
prompt + split-pane real estate, the cursor-saga
fix. The result is a large + active surface that
benefits from a sweep before v0.13.0 ships.

## Scope (three sub-passes, in this order)

### 1. Dead-code + deprecated-pattern sweep

* Run a TypeScript dead-export sweep across `web/src/`.
  Tools available: `npx ts-prune`, `npx
  knip` (either; pick whichever fits the repo style
  best). Per-finding: confirm it's actually dead
  (test imports, dynamic refs, vite ?raw imports
  don't show up cleanly) before deleting.
* Grep for `// TODO` / `// FIXME` / `// XXX` in
  `web/src/`; triage each (still relevant / stale /
  P0+).
* Look for `@deprecated` JSDoc tags and inactive
  Svelte 4 patterns left over from migrations
  (`$:` reactive statements where runes would fit;
  `export let` props that should be `$props()`).
  Do NOT do a wholesale migration; just flag
  obvious leftovers.

### 2. Accessibility audit

Surfaces (priority order; cover all four if time):

1. **Editor** — keyboard nav (Tab / Shift-Tab through
   the toolbar; focus visibility; the Cmd+B / Cmd+I
   etc. shortcuts surfacing in a discoverable place);
   screen-reader labels on toolbar buttons;
   `aria-*` attributes on the contenteditable
   surface.
2. **Hybrid Nav** — keyboard activation of T/O/P/G/E
   shortcuts; tab-stops; focus traps in modals
   (settings, preferences, screensaver PIN).
3. **File Browser** — keyboard nav (arrow keys
   through tree; Enter to open; Cmd+N for new draft);
   ARIA roles on the tree.
4. **Graph / Carousel** — keyboard alternatives to
   mouse-driven interactions where possible;
   `prefers-reduced-motion` honored for the
   pane-flip + carousel animations.

Tool: Chrome devtools Accessibility panel + axe-core
DevTools extension. Manual keyboard-only walkthroughs
on at least the editor + hybrid nav. Report findings
per surface.

### 3. Performance pass

Targets (in order):

1. **Editor** — long-document scrolling smoothness
   (paste a Linux-kernel CHANGELOG, scroll, observe
   FPS via devtools Performance panel). Identify any
   layout-thrash hotspots.
2. **Graph overlay** — large-drive open (the Linux
   kernel checkout if convenient; otherwise a real
   chan-drive with 1000+ files). Initial render time
   + interaction latency.
3. **Carousel** — slide-change frame timings;
   image-load lazy-loading correctness.
4. **General SPA** — first-load bundle size
   (`web/dist/assets/index-*.js`); any obvious
   tree-shake opportunities. Don't tune output of
   `vite build` aggressively — that's @@Systacean +
   future bundle-analysis work.

For each: capture devtools Performance trace summary
(slowest task, longest layout, longest paint); flag
P0/P1 (visible jank with reasonable input) vs P2+.

## Acceptance criteria

1. **Dead-code sweep**: tool ran + report
   (X exports flagged, Y deleted, Z preserved
   with rationale).
2. **Accessibility audit**: surfaces walked +
   per-surface report; P0/P1 fixed in-task or
   filed as follow-up task with explicit
   "release-blocker" tag.
3. **Performance pass**: trace summary per
   target surface + P0/P1 findings either
   fixed or filed.
4. **Final report at task tail**:
   * What was found (counts + categorisation).
   * What was fixed in-task (commit references).
   * What was deferred (severity + follow-up
     task link if filed).
5. **All gate checks pass**: cargo fmt + clippy
   + cargo test + npm test + svelte-check +
   npm build.

## How to start

1. `npx ts-prune` (install if not present) →
   triage output.
2. Devtools axe-core panel on a running chan-desktop
   (use a throwaway drive per the
   test-server-workflow). NOT @@Alex's running
   chan.app session.
3. Devtools Performance panel for the perf pass.
4. Fix in-task as you go (each fix is a separate
   atomic commit); end-of-task append produces the
   report.

## Coordination

* Time-boxed: ONE pass. Round closes when no
  release-blockers remain.
* P0 (data-loss / crash on common path) → fix
  in-task, flag for @@WebtestA walk.
* P1 (broken accessibility on core surfaces /
  visible jank) → fix in-task if cheap, file as
  blocker if not.
* P2 (polish opportunities) → defer to v1.x;
  log in report.
* DO NOT touch @@Alex's running chan.app session.
  Use throwaway drives + dev builds only.

If a fix touches @@Systacean or @@CI surface
(unlikely for pure frontend work), poke first
before editing.

## 2026-05-23 — scope amendment by @@Architect: sub-pass 4 added (chan-server file-read perf)

@@Alex reported (2026-05-23) a 10s timeout on
`GET /api/files/docs/journals/phase-8/alex/event-desktect-alex.md`
(1826-byte file). Root cause traced to chan-server's
`api_read_file` (`crates/chan-server/src/routes/files.rs:248`):
the handler calls `state.drive().read_text()` /
`state.drive().read()` directly in the async context,
**without** `tokio::task::spawn_blocking`. By contrast,
`api_write_file` at `:311` wraps the write at `:318`.

Symptom: blocking FS IO on the async worker thread.
Under contention (concurrent reads, fresh indexer
activity from another session's writes) workers
starve and small-file reads can blow past the SPA's
10s client timeout.

@@Alex framing: "this is not the first time I notice
this kind of issue while loading a .md file from disk
into the editor.. and it shocks me that we cannot
instantly open a 21k bytes file and it times out with
10s". Recurring; promoted to fix-in-Round-3.

### Sub-pass 4 (added)

**chan-server file-read perf — `spawn_blocking` wrap +
GET-handler audit.**

1. Wrap `api_read_file`'s editable-text branch
   (`files.rs:258`) and binary branch (`:273`) in
   `tokio::task::spawn_blocking`, mirroring
   `api_write_file:318`.
2. Test pin for the wrap shape (Rust-side; or a
   smoke verifying read-handler doesn't block the
   runtime under concurrent load — judgment call on
   what's tractable to test).
3. Audit `crates/chan-server/src/routes/*.rs` for
   other GET handlers that call sync chan-drive
   methods directly without `spawn_blocking`. Likely
   candidates: `search/files`, `search/content`,
   `link-targets`, anything reading drive state in
   an axum `get(...)`. Fix obvious ones in-task; if
   the audit turns up >3 affected handlers, fix the
   two most user-facing + file the rest as a
   follow-up task.

### Acceptance criteria (additions to original)

6. **chan-server reads don't block the async
   runtime.** Concurrent file-read load doesn't stall
   other endpoints.
7. **Audit report at task tail**: which other GET
   handlers had the same shape + which got fixed
   in-task + which deferred.

### Residual concern (flag, not in this sub-pass)

Even with the `spawn_blocking` wrap, a 10s timeout on a
1826-byte file points to a deeper symptom (worker
genuinely wedged on something else, not just blocked).
The wrap is necessary but may not be sufficient. If the
recurrence persists post-fix, a chan-server diagnostics
pass is warranted (instrumented logging on request
arrival / departure; lock contention probes). NOT in
scope for `-96`'s time-boxed cap.

### Coordination

* Release-class fix; bundles cleanly into `-96`'s
  perf-pass sub-pass per @@Alex's call (vs cutting a
  separate task).
* Per the time-boxed hardening cap, the audit doesn't
  recurse: one pass, file findings, fix obvious ones.
