# Rich Prompt Core Lane

Date: 2026-05-24
Owner: @@CoreArchitect
Status: Core slice and FD pressure follow-up implemented, awaiting Web browser
verification

## Scope

This follows `architect/next-architect-handover.md` for the Core lane. It does
not use `docs/agents/bootstrap.md`.

Audited:

- `crates/chan-drive/src/drafts.rs`
- `crates/chan-drive/src/drive.rs`
- `crates/chan-drive/src/watch.rs`
- `crates/chan-server/src/routes/drafts.rs`
- `crates/chan-server/src/routes/terminal.rs`
- `crates/chan-server/src/terminal_sessions.rs`
- `crates/chan-server/src/event_watcher.rs`
- `crates/chan-server/src/routes/teams.rs`
- `crates/chan-server/src/state.rs`
- `crates/chan-llm/src/mcp.rs`
- `crates/chan-llm/src/tools.rs`
- `docs/journals/phase-9/architect/rich-prompt-web-lane.md`

## Current Behavior

Rich Prompt is not yet a Core-owned draft workspace lifecycle.

- `chan-drive` already owns draft primitives: create, inspect, discard to
  metadata trash, preflight warning, and no-clobber promote.
- `POST /api/drafts/rich-prompt` is history-only. It creates a fresh
  `Drafts/rich-prompt-N/prompt.md` for each submit and does not create an
  active `draft.md` or `spool/` tree.
- The terminal event watcher is attached through
  `PUT /api/terminal/:session/watcher` and lives in
  `terminal_sessions::Session`.
- Closing a terminal drops its watcher handle and kills the shell, but it does
  not discard any draft workspace.
- Watcher status is mostly implicit. `watcher_dir()` returns `None` when no
  watcher is attached, and dropped events are only exposed as a process-wide
  health counter.
- `GET /api/terminal/:session/watcher/events` reads event files from the
  current watcher directory with size and filename filters.
- `resolve_terminal_cwd` already maps `Drafts/...` to metadata directories, so
  terminal cwd can point at a draft workspace.
- MCP descriptions already say `Drafts/...` resolves to uncommitted metadata
  outside the drive root.
- Drive boot already reports broken draft workspaces through `/api/drive`
  warnings, but it does not distinguish active Rich Prompt workspaces from
  ordinary drafts or team workspaces.

## Target Ownership

Core owns:

- Atomic Rich Prompt workspace creation under the draft metadata root.
- Safe workspace naming and path normalization.
- Creation of `draft.md` and `spool/` subdirectories.
- Watcher attach, detach, status, and error semantics.
- Submit-side archive of `draft.md` into `prompt-N.md` plus blank reset.
- Terminal-owned close semantics: stop watcher, close shell, discard workspace.
- Boot warnings for broken active Rich Prompt workspaces.
- MCP contract wording when Rich Prompt workspaces become intentional agent
  workspaces.

Web owns:

- Entry point behavior for Cmd+P and Cmd+. P.
- Visible state machine and user feedback.
- Prompt editor, plus menu, event counter, agent picker, and Spawn agents UI.
- Browser and iab verification.

Boundary rule: Web should not infer filesystem safety from local paths. Core
must return the phase, paths, watcher state, and broken reason.

## Recommended Core Contract

Add a small `rich_prompts` route/module in `chan-server` rather than stretching
the generic `/api/drafts/*` routes. Keep filesystem policy in `chan-drive` and
terminal lifecycle in `terminal_sessions`.

Preferred HTTP shape:

- `POST /api/rich-prompts`
  - Input: terminal session id, optional requested label.
  - Effect: create one draft workspace atomically enough that reload never sees
    a half-built Rich Prompt as active.
  - Output: session id, name, draft path, workspace path, events path,
    process path, watcher state, and phase.

- `GET /api/rich-prompts/:name/status?session=<id>`
  - Effect: inspect workspace and session-scoped watcher state.
  - Output: `active` or `broken`, watcher state, paths, submission sequence,
    and reason when broken.

- `POST /api/rich-prompts/:name/submit`
  - Input: editor `content`, expected submission sequence, and optional
    `expected_mtime_ns`.
  - Effect: archive the provided editor buffer as `prompt-N.md`, write a fresh
    blank `draft.md`, and return the next sequence.

- `POST /api/rich-prompts/:name/close`
  - Input: terminal session id.
  - Effect: stop watcher, close terminal session when still present, move the
    workspace to metadata trash, and return either `discarded` or `broken`.

The Web lane suggested the same basic contract. Core recommendation is to use
workspace name in the URL and pass terminal session id in bodies where terminal
ownership matters. That keeps the persistent draft identity separate from the
ephemeral PTY identity.

Locked details after Web review:

- Status is session-aware from slice 1. The route takes `session` as a query
  parameter so Core can report `terminal session missing` instead of guessing
  from workspace state alone.
- Submit archives the exact editor buffer posted by Web. Core does not rely on
  a prior save of `draft.md` before archiving.
- Active workspaces carry a marker in slice 1. Boot/preflight uses the marker
  to distinguish active Rich Prompt residue from old history-only
  `Drafts/rich-prompt-N/prompt.md` directories and ordinary drafts.

## chan-drive Additions

Add Rich Prompt workspace primitives beside `drafts.rs`, either in that module
or a narrow sibling module if the code starts to crowd the generic draft
helpers.

Needed primitives:

- Create workspace:
  - Validate name with the same single-component rule as drafts.
  - Create an active Rich Prompt marker.
  - Create `draft.md`.
  - Create `spool/process.md`.
  - Create `spool/events/`, `spool/journals/`, and `spool/tasks/`.
  - Refuse collisions.
  - Return unified paths and physical metadata paths.

- Inspect workspace:
  - Validate required entries.
  - Reject symlinks, FIFOs, sockets, devices, unreadable directories, and
    missing `draft.md`.
  - Treat missing or unsafe `spool/events/` as broken.
  - Return a structured reason, not just an I/O string.

- Submit workspace:
  - Pick the next `prompt-N.md` in the same workspace.
  - Refuse to overwrite an existing prompt archive.
  - Write the submitted editor content to `prompt-N.md`.
  - Create a new blank `draft.md` atomically.

- Discard workspace:
  - Reuse the existing metadata-trash move path.
  - Report cleanup failures so Web can keep the terminal visible when manual
    recovery is still possible.

Do not route Rich Prompt setup through a sequence of generic `write_text` and
`create_dir` calls from Web. That would create reload-visible partial state.

## terminal_sessions Additions

Current watcher ownership is close, but status is too implicit for Rich Prompt.

Needed changes:

- Add a per-session watcher status snapshot:
  - `detached`
  - `attached { dir }`
  - `failed { message }`

- Record watcher start failure and provider callback errors in session state.
  The current process-wide `watcher_dropped_events` counter is still useful,
  but it cannot drive a session-specific broken state.

- Add an explicit method that attaches a watcher to a known `spool/events`
  directory without creating arbitrary paths. The current public watcher route
  accepts arbitrary absolute paths for manual workflows and should stay
  separate from Rich Prompt internals.

- Keep watcher identity tied to the terminal session id, not tab label. Rename
  must not affect watcher routing.

- Restart should preserve Rich Prompt metadata identity only if the same
  terminal tab/session is explicitly restarted by Web. A raw session missing on
  restore should surface as broken instead of silently creating a new owner.

## Failure Policy

Default policy from the handover should stand.

## Failure Mode Audit

Current behavior versus target behavior:

- Missing workspace or required files:
  - Current: manual watcher attach creates missing directories silently. The
    existing Rich Prompt history route creates `prompt.md` only, so
    `rich-prompt-N` directories are broken under the new draft contract because
    they lack `draft.md`.
  - Target: Rich Prompt create builds `draft.md` and the full `spool/` tree in
    one Core operation. Missing required pieces produce an explicit broken
    session status.

- Chmod-broken workspace, spool, events, or trash:
  - Current: draft scan reports read errors when the path is inspectable.
    Terminal watcher poll and reply paths return errors later, while the
    watcher handle can remain live without session health.
  - Target: permission loss stops the affected watcher, keeps the terminal open
    when possible, records a persistent reason, and boot preflight warns until
    fixed.

- Deleted active workspace or event directory:
  - Current: a deleted watcher directory usually shows up as read errors later.
    There is no immediate per-session watcher health state.
  - Target: deleted draft, `spool/`, or `spool/events/` marks the Rich Prompt
    broken, stops the watcher, keeps the terminal recoverable, and reports
    cleanup failures on close.

- Stale terminal session:
  - Current: restore can create or attach a new PTY by id or window/tab
    identity. Restart creates a new `Session` without preserving watcher state.
  - Target: workspace identity survives terminal rename. Restart or restore
    must reconcile explicitly, and a missing PTY is a broken but recoverable
    Rich Prompt state.

- Browser reload and server restart:
  - Current: browser reload against the same server usually preserves the PTY
    and watcher. Server restart loses watcher state. Cmd+P still toggles an
    existing active terminal prompt.
  - Target: Cmd+P always creates a new terminal plus backend workspace. Reload
    reconciles workspace, terminal session, and watcher through the status
    endpoint.

- Unsafe event files:
  - Current: watcher ingestion filters filenames but then uses normal metadata
    and `read_to_string`; matching symlinks, FIFOs, or huge event files are not
    gated as tightly as the polling endpoint. The polling endpoint already
    skips non-files and files larger than 1 MiB.
  - Target: event ingestion uses lstat semantics, regular-file-only checks,
    size caps, and no symlink following. Unsafe event entries break only the
    affected watcher session.

- Watcher provider error:
  - Current: drive watchers surface `ProviderError`; terminal event watchers
    only increment a global dropped-event counter and log.
  - Target: provider errors are per-session. Stop that watcher, store the
    reason, expose it through status, and keep the terminal open.

- Missing workspace:
  - Status becomes `broken`.
  - Stop watcher if still attached.
  - Keep terminal open when possible.

- Missing `spool/` or `spool/events/`:
  - Status becomes `broken`.
  - Stop watcher because the event channel is no longer trustworthy.

- Chmod-broken workspace, spool, events, or trash:
  - Status becomes `broken`.
  - Reads and close/discard return the permission error.
  - Terminal remains open if discard cannot complete.

- Unsafe entry in workspace:
  - Status becomes `broken`.
  - Submit and close/discard should not silently skip it.
  - Discard may move the whole directory to metadata trash if the trash move
    can preserve the unsafe entry without reading it. If not, report failure.

- Watcher provider error:
  - Mark this Rich Prompt watcher `failed`.
  - Stop using that event stream until an explicit reattach succeeds.
  - Surface a persistent warning.

- Terminal session missing on reload:
  - Status becomes `broken`.
  - Web can offer restart or discard, but Core should not silently attach the
    workspace to a new terminal.

- Close failure halfway:
  - Best-effort cleanup is acceptable only if the failure is returned.
  - Boot preflight must warn again while residue remains.

## Workspace Naming

Recommendation for the first implementation:

- Use monotonic `rich-prompt-N` draft names.
- Return the chosen name to Web.
- Store terminal ownership in Web session state and, if needed later, a small
  metadata file under the workspace.

Reason: terminal session ids are ephemeral and poor user-facing directory
names. User labels are useful UI, but they add collision and rename semantics
before the lifecycle is stable.

## Process File

Core should generate `spool/process.md` from a bundled static string or helper,
not by reading `docs/agents/bootstrap.md`. The handover explicitly says not to
use that bootstrap process unless @@Alex asks.

First version can be minimal:

- Host and lead identity placeholders.
- Event filename convention.
- Directory purpose for `events/`, `journals/`, and `tasks/`.
- MCP note that `Drafts/...` is metadata-backed and uncommitted.

If @@Alex wants the current bootstrap text adapted, that should be a separate
reviewed content task.

## Interaction With Teams

Do not merge Rich Prompt lifecycle with existing Team workspace lifecycle in
the first pass.

Current team workspaces are persistent `team-{name}/` directories with a
non-destructive watcher load/unload model. Rich Prompt workspaces are
terminal-owned and discarded on terminal close. Sharing one route or lifecycle
would make teardown rules ambiguous.

Spawn agents can still reuse team config types or terminal spawn helpers once
the Web contract is settled.

## Implementation Order

1. Add chan-drive Rich Prompt workspace create, inspect, submit, and discard
   primitives with unit tests.
2. Add terminal watcher status and a session-specific provider-error state.
3. Add `chan-server` Rich Prompt routes with blocking boundaries around
   chan-drive calls.
4. Wire create to attach the terminal watcher to `spool/events/` and return
   status.
5. Wire close to stop watcher, close shell, discard workspace, and report
   cleanup failures.
6. Update MCP descriptions only if the new workspace process needs more
   explicit guidance than the current `Drafts/...` wording.
7. Hand to Web for state-machine integration.

## Test Plan

Core unit and route tests:

- Create builds `draft.md`, `spool/process.md`, `spool/events/`,
  `spool/journals/`, and `spool/tasks/`.
- Create refuses name collisions and path separators.
- Inspect reports missing `draft.md`, missing `spool/events/`, unreadable
  directories, symlinks, FIFOs, sockets, and special files.
- Submit archives `draft.md` as `prompt-1.md`, then resets blank `draft.md`.
- Submit refuses archive collisions.
- Close detaches watcher before discard.
- Close returns an error when trash move fails and leaves the terminal
  recoverable.
- Watcher provider errors mark only the affected session as failed.
- Terminal rename and restart do not change watcher identity.
- Missing terminal session on status is reported as broken.
- `resolve_physical_dir("Drafts/rich-prompt-N")` remains valid while the
  workspace exists.

Live checks after Web integration:

- Cmd+P creates a fresh terminal and a Rich Prompt workspace.
- Reload restores terminal plus workspace or shows broken state.
- Deleting the active workspace enters broken state.
- Removing permissions from `draft.md`, `spool/`, `spool/events/`, or trash
  enters broken state and does not silently discard.
- Closing the terminal stops watcher, closes shell, and moves workspace to
  metadata trash.
- MCP can list, read, write, and resolve `Drafts/rich-prompt-N/...` while the
  workspace is active.

## Open Decisions

- Should close use one route that closes the shell and discards, or should Web
  close the terminal first and call discard second? Core recommendation is one
  route to keep teardown order testable.
- Should `spool/process.md` be generated from a static bundled template or
  from a richer config object? Core recommendation is a static first version.
- Should event counts come from server status or from Web polling
  `/api/terminal/:session/watcher/events`? Core recommendation is status for
  watcher health, existing events endpoint for counts until that proves too
  expensive.
- Should Rich Prompt active workspaces have a marker file for boot preflight?
  Core recommendation is yes if Web wants warnings that distinguish ordinary
  broken drafts from broken active Rich Prompt residue.

## Coordination Notes

- This plan agrees with Web's need for one atomic workspace creation contract.
- Web can start entrypoint and visual state work behind temporary API stubs, but
  should not create the workspace tree client-side.
- Core should not edit `TerminalRichPrompt.svelte`, `tabs.svelte.ts`, or Spawn
  agents UI files without explicit coordination.
- Web should not edit `chan-drive` draft primitives, terminal watcher internals,
  or Rich Prompt server routes once Core starts implementation.

## Implementation Log

2026-05-24: Core slice landed the server/drive contract agreed with Web:

- `chan-drive` has Rich Prompt workspace primitives for create, inspect,
  submit, discard, and preflight.
- Active workspaces get `.chan-rich-prompt-active`; legacy history-only
  `Drafts/rich-prompt-N/prompt.md` directories are not reported as active
  broken prompts.
- Create builds `draft.md`, `spool/process.md`, `spool/events/`,
  `spool/journals/`, and `spool/tasks/`.
- Submit archives the posted editor buffer into `prompt-N.md` and blanks
  `draft.md`.
- `chan-server` exposes `POST /api/rich-prompts`,
  `GET /api/rich-prompts/:name/status?session=<id>`,
  `POST /api/rich-prompts/:name/submit`, and
  `POST /api/rich-prompts/:name/close`.
- Terminal watcher status is now session-aware: detached, attached, failed, or
  missing session through the Rich Prompt status response.
- Terminal event ingestion now rejects matching unsafe or oversized event files
  with lstat-style checks and records a session watcher failure.

Verification:

- `cargo fmt --check`
- `git diff --check`
- `cargo test -p chan-drive rich_prompts --lib`
- `cargo test -p chan-drive drafts --lib`
- `cargo test -p chan-drive drive::tests::draft --lib`
- `cargo test -p chan-drive --lib -- --test-threads=1`
- `cargo test -p chan-server routes::rich_prompts --lib`
- `cargo test -p chan-server terminal_sessions::tests::watcher --lib`
- `cargo test -p chan-server --lib`

2026-05-24 FD pressure follow-up:

- Added `chan-drive` FD-budget probing for the current soft `nofile` limit and
  open descriptor count.
- The probed `nofile` value is capped at an internal effective ceiling of 4096
  for budgeting, so unlimited or very high process limits do not let indexing
  fanout grow without bound.
- Live `Drive` handles now acquire an adaptive descriptor-pressure permit.
  Under low process limits this caps concurrent open drives while leaving room
  for editor reads, writes, PTYs, and watcher handles.
- Graph reader pools now open lazily and shrink to one reader under low
  descriptor headroom.
- BM25/Tantivy writer fanout now uses one worker and one merge thread under
  low descriptor headroom instead of Tantivy's default worker plus merge pool.
- Search indexing read workers are capped under low descriptor headroom.

Verification:

- `cargo fmt --check`
- `cargo test -p chan-drive fd_budget --lib`
- `cargo test -p chan-drive index::facade::tests::search_aggression_budget_profiles_are_bounded --lib`
- `cargo test -p chan-drive index::bm25::tests::index_then_search --lib`
- `cargo test -p chan-drive --lib -- --test-threads=32`
- `cargo test -p chan-drive --lib -- --test-threads=16`
- `cargo test -p chan-drive --lib`
- `cargo test -p chan-server --lib`
- `HOME=/private/tmp/chan-fd-home-repo-check ./target/debug/chan index rebuild <repo>`

The isolated repo rebuild indexed 714 files and 14917 chunks with zero errors
under a 256 FD soft limit. After adding the live-drive permit, the artificial
`--test-threads=32` stress also passes under the same low limit.

2026-05-24 Rich Prompt watcher warning follow-up:

- Web browser validation reported repeated
  `watcher event stream lost scope; requesting rebuild` warnings during normal
  Rich Prompt workspace activity.
- Core classified path-less non-provider watcher events as notify noise for
  metadata churn, not a real loss-of-scope signal.
- The server indexer now ignores path-less create, modify, remove, and rename
  events. Provider errors and broadcast lag still request a rebuild.

Verification:

- `cargo test -p chan-server indexer::tests::classify_watch_event --lib`
- `cargo test -p chan-server --lib`
- `cargo fmt --check`
- `git diff --check`
