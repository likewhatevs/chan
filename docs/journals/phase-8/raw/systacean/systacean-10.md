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

## 2026-05-20 — Implementation + commit readiness

Two-file change, narrow scope, matches the task spec.

### `crates/chan-server/src/event_watcher.rs`

* Module doc gains a "Watcher event-file naming
  convention" section: filename must match
  `^(event|pre-flight)-<id>\.(md|json)$`, `.md`
  recommended (`.json` for compatibility), content is
  JSON conforming to `AgentEvent`. Anything else
  silently ignored. Cross-references the three filter
  sites (SPA, server read endpoint, ingest path).
* New `is_watcher_event_filename` helper duplicates the
  manual parser from
  `routes/terminal.rs::is_watcher_event_filename`
  (systacean-9). Decision rationale: no `regex` crate
  dep already in the workspace; the helper is 20 lines
  + self-contained. A third consumer would be the
  trigger to extract to a shared `util.rs` module.
* `ingest_once`'s old standalone hidden-file guard is
  subsumed by the regex check (the helper rejects
  leading `.` already). The directory guard from
  systacean-5 stays first (different semantic: macOS
  FSEvents synthetic Create on watch root).
* Three new tests:
  * `is_watcher_event_filename_matches_spa_regex` —
    mirror of the routes/terminal.rs test; pins the
    three-site filter regex in lockstep.
  * `ingest_once_silently_skips_nonmatching_filename` —
    stray `notes.md` in the watched dir; no counter
    bump, no warn, no dispatch.
  * `ingest_once_warns_and_bumps_dropped_for_invalid_json_with_matching_name` —
    matching name + bad JSON IS a dropped event;
    counter + warn fire per the existing branch.
* Existing tests still pass:
  `ingest_once_skips_directory_paths_silently` (the
  directory guard runs before the regex check; both
  cases also fail the regex check incidentally),
  `watcher_dispatches_atomic_rename_once` (uses
  `event-1.json` which matches the regex).

### `docs/journals/phase-8/process.md`

New "Watcher event-file naming convention" section at
the tail of phase-8 process. Documents the regex,
recommended extension (`.md`), cross-references the
three filter sites (SPA + server read + ingest),
explicit invariant: parse failures on matching
filenames KEEP their counter-bump (a producer wrote
bad JSON; that IS a dropped event). Only non-matching
filenames are silenced.

### Gate

Full pre-push gate clean for systacean-10's work:

| Check                                    | Result          |
|------------------------------------------|-----------------|
| `cargo fmt --check`                      | clean           |
| `cargo clippy --all-targets -D warnings` | clean           |
| `cargo test --workspace`                 | passing         |
| `cargo test -p chan-server event_watcher`| 8/8 passing     |
| `cargo build --no-default-features`      | builds          |
| `cd web && npm run check` (svelte-check) | 0e 0w           |
| `cd web && npm test` (vitest)            | 506/506 passing |
| `cd web && npm run build`                | built           |

### Pre-existing pre-push gate finding (not my work)

`RUSTFLAGS=-D warnings cargo build --no-default-features`
fails with a `dead_code` error on
`crates/chan/src/main.rs:1540` —
`fn not_a_chan_drive_hint(root: &std::path::Path) -> String`
from `systacean-8` (`693b161`). Both callers
(`cmd_index_set_semantic`, `cmd_index_status`) carry
`#[cfg(feature = "embeddings")]`, but the function
definition does not. Without the `embeddings` feature
the function is unused → dead code → fails `-D warnings`.

NOT introduced by systacean-10 — my work touches only
`event_watcher.rs` + `process.md`. Surfaced
incidentally while running my own no-default-features
gate. Flagging to @@Architect because the patch-release
push will block on this without a one-line
`#[cfg(feature = "embeddings")]` gate on the function
definition.

### Files modified by me

* `crates/chan-server/src/event_watcher.rs` (+158 / -5)
* `docs/journals/phase-8/process.md` (+28 / -0)

Other modified files in the working tree
(`docs/journals/phase-8/alex/event-fullstack-b-alex.md`,
`docs/journals/phase-8/alex/event-fullstack-b-architect.md`,
`docs/journals/phase-8/fullstack-b/fullstack-b-13.md`)
belong to @@FullStackB; will not stage them. Per the
systacean-4 lesson, pre-commit `git diff --staged
--stat` audit will confirm exactly two files staged.

### Suggested commit subject

```
event_watcher: silently skip non-matching filenames; document naming convention (systacean-10)
```

### Commit readiness

Awaiting @@Architect commit clearance. Holding push per
the patch-release coordination — once the @@FullStackA
+ @@FullStackB rich-prompt tasks land and -10 is
cleared, @@Architect publishes the patch-release
commit-grouping plan and I cut the tag (systacean-3
re-activated).
