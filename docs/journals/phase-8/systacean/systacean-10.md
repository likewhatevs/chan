# systacean-10: Event watcher convention tightening — regex filter + module doc + process.md note

Owner: @@Systacean
Date: 2026-05-20

## Goal

Mirror the SPA / read-endpoint regex filter in chan-server's
fsnotify watcher path so non-event files in the watcher dir
are skipped silently (no read, no parse, no warn, no
`dropped_events` bump). Today the SPA filters on
`^(event|pre-flight)-.+\.(md|json)$` but the fsnotify
ingestion path doesn't — any non-event file produces a
parse-failure warn + counter bump + red toast in the
rich-prompt UI.

Plus a doc tightening: explicitly document the watcher
event-file naming convention in `event_watcher.rs`'s module
doc + a corresponding note in `phase-8/process.md` (or
wherever the watcher protocol lives).

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md)
"Watcher fsnotify path parses every non-hidden file;
convention is not enforced or documented".

Today's `crates/chan-server/src/event_watcher.rs::ingest_once`
(lines 121-183, as of `c69e2fc`) skips only:
* Directory paths (`is_dir()` early-return — from
  `systacean-5`).
* Hidden files (filename starts with `.`).

Everything else gets `read_to_string` + `serde_json::from_str`.
Parse failures bump `dropped_events` + emit
`tracing::warn!` + surface as red toasts.

The SPA-side filter (in `web/src/state/watcherEvents.ts`'s
`eventFilename`) + the systacean-9 server endpoint (in
`crates/chan-server/src/routes/terminal.rs::api_terminal_watcher_events`'s
`is_watcher_event_filename`) both apply the regex
`^(event|pre-flight)-.+\.(md|json)$`. The fsnotify watcher
path is the asymmetric outlier.

Existing event files all use `.md` despite the JSON content
(`event-survey-bug20-v2.md`, `event-reply-<id>.md`). The
convention is `.md`-only; the regex's `.json` tolerance is
defensive but understates the convention. Doc tightening
should name `.md` as the recommended extension; `.json`
remains accepted by the filter.

## Acceptance criteria

* `ingest_once` skips files whose filename doesn't match
  `^(event|pre-flight)-.+\.(md|json)$` silently — no
  `tracing::warn!`, no `dropped_events.fetch_add`, no
  dispatch attempt. Same shape as the directory + hidden-
  file guards already there.
* Existing per-error parse failures (for files that DO
  match the filename pattern but have bad JSON / unknown
  type) keep their existing warn + counter-bump behaviour
  — only NON-matching filenames are silenced.
* New tests pin: (a) matching filename with valid JSON
  dispatches as today; (b) matching filename with invalid
  JSON warns + bumps counter as today; (c) non-matching
  filename in the watcher dir is silently skipped (no
  warn, no counter bump, no dispatch).
* Module doc at top of `event_watcher.rs` gains a "Watcher
  event-file naming convention" section: filename must
  match `(event|pre-flight)-<id>.md` (or `.json`); content
  is JSON conforming to `AgentEvent`; anything else in the
  watcher dir is silently ignored.
* `phase-8/process.md` (or `phase-N/process.md` if a
  newer doc exists) gains a parallel note in the watcher
  protocol section, cross-referenced to the
  `event_watcher.rs` module doc.
* Full pre-push gate green (fmt + clippy + workspace test
  + no-default-features build + svelte-check + npm
  build + vitest).

## How to start

1. Read the existing systacean-5 + systacean-9 commits for
   the filter pattern + test shape (`is_watcher_event_filename_matches_spa_regex`
   in `routes/terminal.rs` is the template).
2. Add the regex check to `ingest_once` (filename-only,
   no content read until the check passes).
3. Mirror the test pattern from systacean-5 for the
   silent-skip behaviour.
4. Update the module doc + process.md.

## Coordination

* Parallel to the @@FullStackA + @@FullStackB
  rich-prompt-mini-wave tasks; no file overlap.
* @@WebtestB re-verifies on lane-B once landed —
  fixture is "drop a non-event file in the watcher dir
  and confirm no red toast / no counter bump."
* Part of the patch-release wave; lands before the
  release tag fires (commit-grouping plan TBD by
  @@Architect once the wave is complete).
