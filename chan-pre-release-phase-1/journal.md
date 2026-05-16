# Chan Pre-Release Phase 1 Journal

Owner: architect. Host: Alex.

Source request: `chan-pre-release-phase-1/request.md`.

## Plan summary

Prepare Chan for its first public engineering release by removing
development-era migration assumptions, filling graph/search/assistant/CLI
parity gaps, and proving the release surface with targeted tests plus one
SME hardening pass before commit.

## Dispatch

| Task        | Owner     | Status | Depends on |
|-------------|-----------|--------|------------|
| architect-1 | architect | REVIEW | -          |
| rustacean-1 | rustacean | REVIEW | -          |
| rustacean-2 | rustacean | REVIEW | -          |
| webdev-1    | webdev    | REVIEW | rustacean-2 API shape (FROZEN, see rustacean-2.md) |
| webdev-2    | webdev    | REVIEW | rustacean-2 API shape (FROZEN, see rustacean-2.md) |
| webdev-3    | webdev    | REVIEW | -          |
| rustacean-3 | rustacean | REVIEW | rustacean-1, rustacean-2 |
| syseng-1    | syseng    | REVIEW | rustacean-1, rustacean-2, rustacean-3 (ALL REVIEW) |
| webtest-1   | webtest   | REVIEW | webdev-1, webdev-2, webdev-3 |
| chan-core-purge-1 | rustacean | REVIEW | rustacean-1 |
| architect-syseng-2 | architect | REVIEW | - (release blocker filed by syseng) |
| webdev-4 | webdev | REVIEW | webdev-2 |
| webdev-5 | webdev | REVIEW | rustacean-2 |
| rustacean-4 | rustacean | REVIEW | architect-syseng-2 |
| rustacean-5 | rustacean | REVIEW | syseng-1 residuals 1+2 |
| rustacean-6 | rustacean | REVIEW | rustacean-2 (mid-path symlink escape) |

Statuses: TODO, IN_PROGRESS, BLOCKED, REVIEW, DONE.

## Critical path

```
architect-1 ─┬─> rustacean-1 ─┬─> rustacean-3 ─┬─> syseng-1
             └─> rustacean-2 ─┴─> webdev-1 ────┤
                              └─> webdev-2 ────┤
webdev-3 ───────────────────────────────────────┴─> webtest-1
```

## Notes & decisions

- First public release means no in-product migration path for old Chan
  dev snapshots. Delete dead migration code, but do not weaken current
  config/index initialization for fresh installs.
- Graph-like filesystem indexing belongs in chan-core / chan-drive if it
  needs durable index storage. This repo should expose it through narrow
  HTTP/CLI/UI surfaces and avoid reimplementing drive traversal rules.
- Existing graph route only emits content graph nodes (markdown files,
  tags, mentions, referenced images, ghosts). Directory/file tree graph
  must be separate or explicitly typed so the current semantic graph is
  not overloaded.
- Search status dashboard should consume existing `/api/index/status` and
  `/api/report/*` surfaces first. Add a small server route only if the
  frontend would otherwise need multiple racy polling calls.
- Settings/file inspector currently owns some index/report presentation.
  New dashboard owns index reset/progress/report overview; file/folder
  inspector should keep only object-specific details.
- CLI parity follows the existing pattern: clap definition plus `cmd_*`
  in `crates/chan/src/main.rs`, with server-owned logic still kept in
  `chan-server` or chan-core.
- Verification gate before commit: Rust build/test/fmt/clippy, web
  check/tests, targeted UI manual pass via Vite, and syseng hardening.

## Log

- 2026-05-16 architect: read request, profile, architect guide, existing
  task format, CLI/graph/search/report/assistant surfaces. Created phase
  journal and first-wave task briefs.
- 2026-05-16 architect: completed architect-1 audit and wrote
  `design-snapshot.md`. Flagged server indexer's pre-v3 contact email
  backfill as rustacean-1 cleanup; classified other legacy/schema hits as
  current UI compatibility or external contract naming.
- 2026-05-16 rustacean: rustacean-1 REVIEW. Removed pre-v3 contact email
  backfill from `chan-server::indexer.rs::Indexer::spawn` and from
  `chan::cmd_status` (CLI output and JSON shape). Reworded auth.rs
  pre-release-build comment and renamed `pane_widths_legacy_file_*`
  test to a snapshot-style name. Verified fmt/clippy/test/build. Filed
  `chan-core-purge-1.md` for the orphan producer-side helper in
  `chan-drive::graph.rs`. Picking up rustacean-2 + rustacean-3 next.
- 2026-05-16 rustacean: rustacean-2 REVIEW. New `/api/fs-graph` route
  in `crates/chan-server/src/routes/fs_graph.rs` covering folder/file
  scope, depth up to 6, symlink/hardlink/ghost classification, and
  loop termination. Wire shape frozen in `rustacean-2.md` for
  webdev-1/-2. 11 new tests (fs_graph module); whole crate at 78
  passing. Implementation kept in chan-server rather than chan-core;
  if chan-core grows a `walk_drive_with_specials` later, the route
  collapses to a thin wrapper.
- 2026-05-16 rustacean: rustacean-3 REVIEW. `chan config get|set`
  added for the editor namespace (theme, editor_theme, line_spacing,
  date_format, pane_widths.*). Writes go through
  `chan_server::EditorPrefs::save` so the existing atomic-write
  contract applies; the `chan` crate picked up `toml` for the dump
  path. Assistant + server config namespaces deferred at this point;
  later architect/backend reconciliation covered both.
  9 new tests; chan crate at 37 passing. `chan graph` and `chan
  status` from backend-1 verified end-to-end with the contacts-
  backfill field removed.
- 2026-05-16 webdev: first frontend passes submitted. `webdev-2.md`
  covers File Browser `Graph this`, direct `dir:` graph scope, and
  folder/parent convenience options; marked webdev-1 REVIEW, with
  architect follow-up to confirm whether `/api/fs-graph` must replace
  the existing semantic graph path. `webdev-1.md` covers SearchPanel
  active-result scroll and assistant chat scroll/bubble/thinking badge;
  marked webdev-3 REVIEW and webdev-2 IN_PROGRESS because the search
  status dashboard/language search remain open.
- 2026-05-16 architect: verification pass. `cargo test -p
  chan-server` green (78 passed). `cargo test -p chan` initially
  failed because config tests referenced a missing read helper; fixed
  tests to use `read_config_key` with `ServerConfig::default`, then
  `cargo test -p chan` green (39 passed). `cargo clippy --all-targets
  -- -D warnings`, `cargo fmt --all -- --check`, and `npm run check`
  all green. Isolated CLI smoke with temp HOME/XDG confirmed
  `chan config get editor.theme`, `set editor.theme=dark`, and
  rejection of `set editor.theme=` without wiping the prior value.
- 2026-05-16 webdev: search status dashboard slice landed after the
  architect verification pass: new `SearchStatusOverlay.svelte`, mounted
  from `App.svelte`, opened from SearchPanel, and DriveInfoBody no longer
  owns search-index status/rebuild UI. Architect reran `npm run check`
  on the current tree: 0 errors / 0 warnings. webdev-2 remains
  IN_PROGRESS because `language:<name>` search is still not accounted for
  in the task notes or implementation observed so far.
- 2026-05-16 webtest: picked up the remaining webdev-2 language search
  gap. `SearchPanel.svelte` now treats `language:<name>` as a report-backed
  file query using existing `api.reportFile` rows, showing matching files
  with reported language and SLOC. Verified `npm run check` and
  `npm test -- --run` green; marking webdev-2 REVIEW.
- 2026-05-16 rustacean: chan-core-purge-1 REVIEW. Removed
  `Drive::contacts_need_email_backfill`, the underlying GraphView
  helper, and its migration-v3 backfill test from chan-core. Trimmed
  schema-header + v3 migration comments and the design.md paragraph
  to drop "indexer triggers rebuild" framing; the v3 migration ALTER
  itself stays since fresh installs still need the column. chan-core:
  `cargo test -p chan-drive` 428 passed; fmt + clippy clean. chan
  repo: still 78 + 39 passing against the updated path-dep. Both
  repos uncommitted on `main`. Filed as an adjacent pickup; architect
  reviews before sealing Phase 1.
- 2026-05-16 webtest: webtest-1 REVIEW. Added a headless Chrome CDP
  smoke runner (`chan-pre-release-phase-1/webtest-smoke.mjs`) and ran
  it against the rebuilt release server on `http://127.0.0.1:8788/`.
  Desktop and narrow smoke pass for `language:TypeScript` search,
  Search Status report dashboard, and File Browser `Graph this`.
  The smoke found a lazy-tree bug in `language:<name>` search; fixed
  `SearchPanel.svelte` to hydrate folder listings before scanning
  per-file report rows. Assistant active-turn smoke is recorded as
  skipped because `/tmp/chan-dev` has
  `preferences.assistant.effective_enabled:false`.
- 2026-05-16 architect/backend: architect-syseng-2 REVIEW. Added
  `apply_watch_change` in `crates/chan-server/src/indexer.rs` so watcher
  events on deleted/missing/symlink/FIFO/special paths return to Idle
  instead of pinning `/api/index/status` to Error. Live repro:
  `touch -h /tmp/chan-syseng-fixture/notes/alias-to-top.md` while
  serving the fixture, then `/api/index/status` stayed idle with 4 docs
  / 4 vectors. Backend tightened the fix to use explicit
  `std::fs::symlink_metadata` before indexing and reverified
  `cargo test -p chan -p chan-server` (43 + 85 passed),
  `cargo clippy --all-targets -- -D warnings`, and
  `cargo fmt --all -- --check`.
- 2026-05-16 webdev: webdev-5 REVIEW. GraphPanel now consumes
  `/api/fs-graph` in filesystem mode, with typed API client/types,
  filesystem graph mode persisted in hash/session state, File Browser
  `Graph this` opening filesystem mode, filesystem labels/inspector
  metadata, and `truncated` surfaced in the graph status bar. Architect
  reran `npm run check`: clean.
- 2026-05-16 webtest: refreshed webtest-1 after webdev-5. Rebuilt
  frontend assets and `target/release/chan`, restarted the 8788
  webtest service, and reran `node
  chan-pre-release-phase-1/webtest-smoke.mjs`. The CDP smoke passes
  on desktop/narrow and now verifies File Browser `Graph this` opens
  filesystem graph mode/status, not merely any graph overlay.
- 2026-05-16 architect: reconciled `webtest-1.md` so the later HTTP
  refresh no longer contradicts the completed CDP browser smoke. Added
  `summary.md` with completed work, verification, residual release risks,
  and agent feedback. Remaining phase decisions are review/commit ordering
  plus assistant-enabled browser smoke.
- 2026-05-16 architect: closed the CLI filesystem graph parity gap.
  `chan graph --scope file|folder` now reuses the same builder as
  `/api/fs-graph`; `--scope all` remains the semantic markdown graph.
  Focused fs-graph/server tests, graph-scope CLI tests, `cargo build -p
  chan`, isolated CLI smoke, `cargo test -p chan` (46 passed),
  `cargo test -p chan-server` (92 passed), fmt, and clippy are green.
- 2026-05-16 architect: extended `chan config get|set` to assistant
  backend settings (`assistant.default_backend`, per-backend enabled/model/
  command override keys, read-only effective state, and answers-dir alias).
  Added focused config tests and tightened `webtest-smoke.mjs` so an
  assistant-enabled fixture verifies pending badge, bottom pin, and wide
  response bubble during an active turn.
- 2026-05-16 backend/syseng: closed the `/api/fs-graph` outside-drive
  classification residual. When canonicalization is unavailable, the
  fallback now accepts only clean lexical descendants of the drive root
  and rejects `..` escape components. Added
  `lexical_fallback_rejects_parent_escape`; `cargo test -p chan-server
  fs_graph` passed.
- 2026-05-16 webtest: assistant-enabled browser smoke is now complete.
  Added `fake-codex-smoke.sh`, ran an isolated assistant-enabled server
  on 8793 with temp HOME/XDG dirs, and extended
  `webtest-smoke.mjs` to verify active-turn pending badge, bottom pinning,
  and wide assistant bubble at desktop and 390px. The smoke found and fixed
  two `InlineAssist.svelte` layout issues: pending empty chats no longer use
  `.empty-chat`, and assistant bubbles stretch across the chat column.
  Normal 8788 smoke still passes with assistant disabled/skipped.

## Highlights

- Existing report endpoints already expose whole-drive SLOC/language
  rollups, which should make the search dashboard mostly frontend work.
- Existing graph overlay has scope/depth plumbing; folder scope likely
  reuses much of the current scoped-node filter once backend can supply
  filesystem graph nodes.

## Lowlights

- Request spans multiple boundaries at once: chan-core indexing, server
  routes, Svelte overlays, CLI parity, and release cleanup. Risk is idle
  time if API contracts are not frozen early.

## Follow-ups

- After rustacean-2 starts, freeze the file/directory graph wire shape in
  that task file so webdev-1 can proceed without guessing.
- After rustacean-3 starts, freeze CLI output shapes before tests are
  written around them.
- syseng-1 is unblocked for the full hardening pass.
- architect-syseng-2 is fixed in this repo; no chan-core change needed
  for that blocker.
- chan-core-purge-1: rustacean executed in the sibling repo (REVIEW).
  Architect to choose between committing chan-core first and bumping
  the path-dep version, or bundling the two repos' commits together
  given they're locally co-checked out.
- `summary.md` exists as a release-review artifact. Commit ordering is
  handled: sibling `chan-core` cleanup is committed separately from the
  main `chan` phase commits.
- Assistant active-turn browser smoke has a clean full-run sign-off through
  the isolated fake-Codex fixture on port 8793. Normal 8788 smoke continues
  to verify the disabled-assistant fixture path.
- 2026-05-16 rustacean: rustacean-4 REVIEW. Resolved
  `architect-syseng-2.md` release blocker. Replaced the unconditional
  `index_file` / `forget_file` dispatch in the chan-server watch
  loop with `apply_watch_change`, which lstat-classifies the path
  via `std::fs::symlink_metadata` before calling `Drive::index_file`
  and routes symlinks / FIFOs / vanished paths to a
  no-op-with-cleanup branch instead of sticking `IndexStatus::Error`.
  7 new tests under `indexer::tests` covering regular / forgotten /
  missing / symlink / broken-symlink / FIFO / replaced-by-symlink.
  Live repro
  (`touch -h alias.md` against a fresh drive) confirmed `idle`
  instead of `error` after the fix. Current backend gate is
  chan-server 92 passed; chan 46.
- 2026-05-16 rustacean: rustacean-5 REVIEW. The architect
  verification pass had already tightened `chan graph` to reject
  `..`-escapes and missing targets through `Drive::stat`. Locked
  the behavior in with 4 tests under `tests` in
  `crates/chan/src/main.rs` (escape file, missing file, file vs
  directory, escape folder). Added `tempfile` as a
  `dev-dependencies` entry on the chan crate; the tests bootstrap
  a real `Library` + `Drive` against a temp dir.
- 2026-05-16 rustacean: rustacean-6 REVIEW. Pre-seal audit caught a
  real mid-path symlink escape on `/api/fs-graph`. With an in-drive
  symlink `escape-link -> /etc/`, requests like
  `scope=file&path=escape-link/hosts` and
  `scope=folder&path=escape-link/ssl` returned drive-relative ids
  carrying `/etc/*` file metadata because `resolve_safe` is lexical.
  Added `ensure_parent_inside_drive` in
  `crates/chan-server/src/routes/fs_graph.rs` to canonicalize the
  parent of the joined absolute path and reject if it doesn't start
  with the canonical drive root. Leaf-as-symlink still allowed (the
  walker classifies). 2 new tests; chan-server 92 passed, chan 46.
  Live probes confirmed BAD_REQUEST with `path escapes drive root
  via mid-path symlink: ...` and no regression on in-drive symlinks
  whose leaf points outside. Notified via `architect-rustacean-3.md`.
