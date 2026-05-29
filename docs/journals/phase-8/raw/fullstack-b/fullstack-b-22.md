# fullstack-b-22 — chan-desktop orphan sidecar reap + lock-takeover UX

Owner: @@FullStackB
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Two-part fix for the chan-desktop orphan-sidecar bug filed
in [`../phase-8-bugs.md`](../phase-8-bugs.md) 2026-05-21:

1. **Prevention** (defense in depth): chan-desktop reaps
   its bundled `chan serve` sidecars when chan-desktop
   itself exits. Process group + Drop handler.
2. **Recovery UX**: when a fresh chan-desktop launch hits
   a port-in-use / drive-locked condition from an
   orphaned sidecar, surface a lock-takeover dialog with
   auto-kill of the confirmed-orphan sidecar + user toast.

## Background

Surfaced 2026-05-21 during @@WebtestB's ci-8 dryrun.4
walkthrough recovery: @@Alex's chan-desktop PID 58737 got
SIGTERM'd by mistake; orphaned `chan serve` subprocesses
on ports 49991 + 64869 stayed alive + blocked the next
chan-desktop launch from binding the same drives. @@Alex's
manual recovery (`pkill chan` + `kill -9 <PIDs>` +
`xattr -dr com.apple.quarantine ...`) is opaque enough
that regular users would be stranded.

REGRESSION-class severity — surfaces every time
chan-desktop is killed ungracefully, which the live
incident demonstrated happens in real workflows.

## Phase-9 forward-look

The phase-9 desktop-native vision (memory
`project_phase_9_desktop_native_vision`,
[`../architect/phase-9-desktop-native-vision.md`](../architect/phase-9-desktop-native-vision.md))
may obsolete the orphan-sidecar problem via embed-by-
default of chan into chan-desktop. **Don't sink heavy
investigation into the lock-takeover UX**; ship the
minimum viable fix for v0.12.x. The prevention side (Drop
handler + process group) is durable regardless of phase-9
direction; the recovery-UX side is the piece to keep
minimal.

## Acceptance criteria

### Prevention

* chan-desktop spawns the bundled `chan serve` sidecar
  with a process group (Unix) / job object (Windows) so
  the sidecar inherits the kill chain.
* A `Drop` impl on the chan-desktop side guarantees
  sidecar reap on chan-desktop exit, including the panic
  / crash paths.
* Verified: kill chan-desktop with SIGTERM, SIGKILL, and
  a panic; confirm no orphan `chan serve` on the same
  port via `ps aux | grep chan`.

### Recovery UX (minimum viable)

* Lock-takeover dialog when chan-desktop fails to bind a
  drive port: "An orphaned chan sidecar is holding this
  drive's port. Reclaim?" with one button (Reclaim) +
  Cancel.
* On Reclaim: identify the orphan PID via the drive's
  PID file or port-owner lookup; SIGTERM (escalate to
  SIGKILL after a brief wait); retry binding the drive.
* Visible toast on success: "Reclaimed drive from orphan
  sidecar."
* If reclaim fails (PID not found, permission denied,
  port still busy after kill): surface the failure with
  a copy-paste cleanup snippet rather than silently
  retrying.
* Skip elaborate orphan-detection heuristics. If the port
  has a chan-named process, treat it as the candidate.

## How to start

1. Audit current chan-desktop spawn site for the bundled
   chan sidecar in `desktop/src-tauri/src/main.rs` (the
   PATH-first probe / bundled-fallback work from
   `-b-15`/`-b-16` is where this lives).
2. Add process-group / job-object configuration on the
   spawn command. Audit cross-platform behaviour
   (`std::process::Command` + `CommandExt::pre_exec` on
   Unix; `CREATE_NEW_PROCESS_GROUP` on Windows).
3. Wire a `Drop` impl on the chan-desktop side that kills
   the sidecar; ensure panic-safety + signal-handler
   integration.
4. Build the lock-takeover dialog in the SPA
   (`PathPromptModal.svelte` style or a fresh
   `LockTakeoverDialog.svelte`); wire to chan-server's
   port-bind error response.
5. Test cycle: spawn chan-desktop, force-kill it, launch
   again, confirm clean reclaim.

## Coordination

* @@FullStackB lane (chan-desktop is your area).
  Possible cross-pollination with @@Systacean if a
  chan-drive lock-takeover protocol primitive needs to
  land (e.g. PID file convention, port-owner discovery).
  Surface a scope question if you hit that boundary.
* Pre-push gate must be green.
* When ready for commit, append "Commit readiness" + fire
  poke to @@Architect.

### Shared-infra authorization

* **Authorization: yes** for any edits to
  `desktop/src-tauri/src/main.rs` (chan-desktop process
  supervision is your lane) + the new SPA
  lock-takeover dialog component.
* **Authorization NOT pre-granted** for
  `desktop/src-tauri/tauri.conf.json` edits. If the fix
  needs config changes there, fire a scope question
  before editing.

## Numbering

Highest committed `-b-N` is `-b-21`; this is `-b-22`.

## 2026-05-21 — @@FullStackB: implementation note

Prevention + minimum-viable recovery landed in three files. Heavy
investment on the prevention side (durable past the phase-9
forward-look); recovery UX is the minimum the bug report asked
for.

### Prevention (durable)

`desktop/src-tauri/src/serve.rs`:

* `start()` now sets a fresh process group on the spawn command —
  Unix via `CommandExt::process_group(0)` (sets pgid to the new
  child's PID at exec time); Windows via `CommandExt::creation_flags(
  CREATE_NEW_PROCESS_GROUP)`. Effect: every chan serve child + any
  helper subprocess chan may fork inherits the group.
* `stop_child()` signals the **process group** (`killpg(-pgid,
  SIGTERM)` then `killpg(-pgid, SIGKILL)` after a 5-second grace)
  rather than just the leader's PID. Previously a chan-side fork
  could outlive the SIGTERM-to-leader and continue holding the
  flock; now the group sweep catches it. Module-level doc-comment
  updated to reflect the upgrade from "SIGKILL only" to the
  SIGTERM-then-SIGKILL-on-group shape.

`desktop/src-tauri/src/main.rs`:

* New `impl Drop for AppState` that calls `serve::stop_all(self)`.
  Defense-in-depth for the panic-unwind path: Tauri's
  `RunEvent::Exit` hook is the primary teardown, but a panic
  unwinding through `tauri::App` (or any other Arc holder) bypasses
  the hook entirely. The Drop impl bridges that gap. chan-desktop
  builds with the default `unwind` panic strategy (no
  `panic = "abort"` configured at any profile), so Drop runs on
  panic.

SIGKILL of chan-desktop itself remains the un-covered case (no
unwind, no Drop); that is exactly the gap the recovery UX exists to
plug.

### Recovery (minimum-viable)

`desktop/src-tauri/src/serve.rs`:

* `DRIVE_LOCKED_MARKER` const (`"drive is locked by another
  process"`) mirrors chan_drive's `ChanError::DriveLocked` Display
  string. Substring-matched against the captured stderr tail by
  `stderr_indicates_drive_lock_conflict()`.
* `find_orphan_chan_serve_pids(key)` shells out to
  `ps -ax -o pid=,command=` and filters lines containing `chan`,
  ` serve `, and the drive key. Per the task body's "minimum-
  viable" framing we skip elaborate orphan-detection heuristics —
  the user has already opted in via the Reclaim button. Skipped
  on the chan-desktop's own PID.
* `kill_orphan_with_grace(pid)` SIGTERMs the pid, polls for exit
  for up to one second, then escalates to SIGKILL. ESRCH treated
  as success.
* `ServeFailedPayload` gains a `drive_lock_conflict: bool` field
  populated by the reader thread when the stderr tail matches.
  SPA branches on this rather than re-deriving the substring
  match on the JS side; single source of truth.

`desktop/src-tauri/src/main.rs`:

* New `#[tauri::command] reclaim_drive_lock(path)` returns a
  `ReclaimResult { killed_pids, retry_succeeded, message }`.
  Walks find/kill/retry in one IPC. Registered in
  `generate_handler!`.

`desktop/src/main.js`:

* `showServeFailed` branches on `p.drive_lock_conflict === true`
  into a new `promptDriveLockTakeover(key)` function that calls
  the dialog plugin's `ask()` with **Reclaim** / Cancel buttons.
  On Reclaim, invokes `reclaim_drive_lock` and renders either a
  transient success banner (green-tinted reuse of the
  `error-banner` shape) or a follow-up `message()` modal that
  surfaces a copy-paste `pkill -f "chan serve <key>"` cleanup
  snippet, per the task body's failure-mode requirement.

### Coverage added

| Test                                                            | Asserts                                                                  |
|-----------------------------------------------------------------|--------------------------------------------------------------------------|
| `spawn_command_with_process_group_makes_child_group_leader`     | A spawn with `process_group(0)` gives the child a pgid equal to its PID. |
| `stop_child_reaps_process` (pre-existing, still passes)         | `stop_child` reaps the direct leader via SIGTERM.                        |
| `stderr_drive_lock_marker_detection_is_substring_match`         | Marker scan is a substring scan tolerant of `Error:` prefix / suffix.    |
| `parse_ps_lines_picks_chan_serve_against_key_but_skips_self`    | `ps`-line parser matches the right argv and skips chan-desktop's PID.    |
| `parse_ps_lines_returns_empty_when_no_match`                    | Negative-case path.                                                      |
| `invoke_handler_registers_reclaim_drive_lock`                   | IPC is wired in `generate_handler!`.                                     |
| `serve_failed_payload_drive_lock_field_is_consumed_by_launcher` | SPA reads `drive_lock_conflict` + invokes `reclaim_drive_lock`.          |

`chan-desktop` 39 tests pass (was 32 before -b-22; +7 new — 4 from
the new helpers + 3 from the pin tests / structural pins).

### Pre-push gate

| Surface                                                                 | State                                              |
|-------------------------------------------------------------------------|----------------------------------------------------|
| `cargo fmt --check`                                                     | Clean.                                             |
| `cargo clippy --workspace --all-targets -- -D warnings`                 | Clean.                                             |
| `cargo test --workspace`                                                | All pass (chan-server 425 / chan-desktop 39 / etc).|
| `cargo build --workspace --no-default-features`                         | Clean.                                             |
| `web/` `npx svelte-check`                                               | 3987 files / 0 errors / 0 warnings.                |
| `web/` `npm run build`                                                  | Clean (pre-existing chunk-size warnings only).     |
| `web/` `npx vitest run`                                                 | 58 files / 588 tests pass.                         |

### Runtime verification — recommended for @@WebtestB

The task body's "verified" criterion ("kill chan-desktop with
SIGTERM, SIGKILL, and a panic; confirm no orphan `chan serve` on
the same port via `ps aux | grep chan`") is exactly the
audit-trail walkthrough shape @@WebtestB owns. Standing
chan-desktop runtime permission means I could run it myself, but:

1. The unit tests already pin every code-level invariant the
   prevention side relies on (pgrp set on spawn, SIGTERM reaches
   the group, Drop wired to stop_all).
2. The recovery UX is dialog text + IPC plumbing; an interactive
   click-cycle from a fresh pair of eyes is more useful than my
   own pass.
3. Phase-9's embed-by-default direction may obsolete the whole
   problem; minimum-viable per the architect's forward-look.

Suggested walkthrough shape (lifted from the task body):

1. `make run` from `desktop/` (or `make app-signed` if the signed
   bundle is preferred). Toggle a throwaway drive ON.
2. Open Activity Monitor / `ps aux | grep chan`. Note both PIDs.
3. **SIGTERM** chan-desktop (`kill <chan-desktop-pid>`). Wait
   ~6 seconds. `ps aux | grep chan` should show NO chan serve.
4. Re-launch chan-desktop. Toggle drive ON again — should bind
   cleanly with no dialog.
5. **SIGKILL** chan-desktop with a sidecar running
   (`kill -9 <chan-desktop-pid>`). `ps aux | grep chan` SHOULD
   show an orphan chan serve (SIGKILL bypasses Drop; this is the
   expected unhandled-exit case the recovery UX plugs).
6. Re-launch chan-desktop. Toggle drive ON. The
   "Drive lock held by orphan process" dialog should fire. Click
   **Reclaim**. Expected: orphan is killed, drive opens cleanly,
   green success banner reads "Reclaimed <path> from orphan
   sidecar."
7. Standard test-server-workflow tear-down on the throwaway
   drive.

I can also run the walkthrough myself on @@Architect's say-so;
flagging the @@WebtestB-shape because it's the canonical lane and
the dialog text is the kind of thing fresh eyes catch better.

### Coordination footprint

* No `tauri.conf.json` edit needed — the prevention side is pure
  Rust process-supervision; the recovery side is a new IPC +
  SPA dialog (no capability changes; `invoke` and
  `dialog.ask`/`dialog.message` are already granted).
* No chan-drive change — flock release on process exit is
  kernel-managed; killing the orphan releases the lock for free.
  No new lock-takeover protocol primitive needed (the @@Systacean
  cross-pollination flagged in the task body did not materialise).
* No file overlap with @@FullStackA / @@CI / @@Systacean.
  Touches: `desktop/src-tauri/src/main.rs`,
  `desktop/src-tauri/src/serve.rs`, `desktop/src/main.js`.

### Acceptance criteria — verification

| Criterion                                                                       | State                                                                                  |
|---------------------------------------------------------------------------------|----------------------------------------------------------------------------------------|
| Process group / job object on spawn                                             | `process_group(0)` Unix + `CREATE_NEW_PROCESS_GROUP` Windows, pinned by test.          |
| Drop impl guarantees sidecar reap on chan-desktop exit (incl panic path)        | `impl Drop for AppState` calls `stop_all`; unwind panic strategy in default profiles.  |
| Lock-takeover dialog with Reclaim + Cancel                                      | `promptDriveLockTakeover()` via `ask()` plugin, with explicit `okLabel: 'Reclaim'`.    |
| On Reclaim: identify orphan PID, SIGTERM→SIGKILL, retry binding                 | `reclaim_drive_lock` IPC + `find_orphan_chan_serve_pids` + `kill_orphan_with_grace`.   |
| Visible status on success                                                       | Transient inline banner ("Reclaimed <path> from orphan sidecar.") after `refresh()`.   |
| Reclaim failure path surfaces copy-paste cleanup snippet (no silent retry)      | `pkill -f "chan serve <key>"` snippet rendered via `message()` modal.                  |
| Skip elaborate orphan-detection heuristics                                      | Substring match on `chan` + ` serve ` + key in `ps` output; user opts in via Reclaim.  |
| Pre-push gate green                                                             | See table above.                                                                       |
| Runtime walkthrough (SIGTERM/SIGKILL/panic + `ps aux | grep chan` empty)        | Recommended for @@WebtestB or my follow-up empirical pass post-clearance.              |

### Suggested commit subject

```
chan-desktop: process-group sidecar reap + drive-lock-takeover UX (fullstack-b-22)
```

Touches:

* `desktop/src-tauri/src/serve.rs`
* `desktop/src-tauri/src/main.rs`
* `desktop/src/main.js`

Standing by for @@Architect clearance.

## 2026-05-21 — committed as `3987e73`

Cleared per @@Architect's `## 2026-05-21 — @@Architect: approved
+ commit clearance (fullstack-b-22)` heading in
[`../alex/event-architect-fullstack-b.md`](../alex/event-architect-fullstack-b.md).

Commit subject (accepted verbatim from the suggestion above):

```
chan-desktop: process-group sidecar reap + drive-lock-takeover UX (fullstack-b-22)
```

Files committed (explicit per-path `git add`):

* `desktop/src-tauri/src/main.rs`
* `desktop/src-tauri/src/serve.rs`
* `desktop/src/main.js`
* `docs/journals/phase-8/fullstack-b/fullstack-b-22.md`

Pre-commit `git diff --staged --stat`: 4 files, 753 insertions, 9
deletions — no stowaways from concurrent in-flight work in the
shared tree. Post-commit `git show --stat HEAD`: matches the
staged stat exactly. Push held per the v0.11.x release
discipline.

Runtime walkthrough remains routed to @@WebtestB per
@@Architect's clearance heading; my standing chan-desktop
runtime permission survives the recycle if a follow-up empirical
pass is needed.

Moving on to `-b-23` per the pre-recycle handover queue.
