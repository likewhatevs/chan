# desktacean-1 — chan-desktop Round-3 hardening pass

Owner: @@Desktacean
Phase: 8, Round 3
Date cut: 2026-05-23

## Goal

Run one time-boxed hardening pass over the Tauri desktop shell:
capabilities, IPC surface, updater path, and sidecar lifecycle.
Fix P0/P1 blockers in-task; report P2+ findings at the tail.

## Background

This task is the first pickup from the chan-core handoff in
[`../alex/event-architect-desktect.md`](../alex/event-architect-desktect.md).
It inherits the Round-3 hardening row originally parked for the
old @@FullStackB lane:

* chan-desktop / Tauri cleanup.
* Capabilities audit.
* IPC surface review.
* Updater verification.

Relevant shipped history:

* `fullstack-b-15` / `fullstack-b-16`: bundled chan binary +
  PATH-first probe.
* `fullstack-b-22`: Unix process-group reap + `AppState` Drop
  handler + drive-lock takeover IPC.
* `fullstack-b-25`: orphan-candidate heuristic hardening +
  custom reclaim dialog.
* `fullstack-b-26` / `fullstack-b-27`: Reload / Open Inspector
  IPC and Cmd+Shift+N accelerator move.
* `fullstack-b-28`: pre-flight foundation and report.
* `fullstack-b-29`: WebGL renderer.
* `fullstack-b-30`: terminal font shipping path.
* `systacean-12`: updater verification trail.
* `ci-7` / `ci-8` / `ci-14`: signed/notarized desktop workflow
  and release job fixes.

## Acceptance Criteria

1. **Capabilities audit**: every file under
   `desktop/src-tauri/capabilities/` reviewed. Report each
   permission class as necessary / unnecessary / follow-up.
   Remove obviously dead grants if local evidence is decisive.
2. **IPC review**: every `#[tauri::command]` in
   `desktop/src-tauri/src/` is inventoried with caller,
   input validation, side effect, and error behavior. Fix
   obvious P0/P1 issues.
3. **Updater verification**: compare current updater wiring
   against the `systacean-12` findings and current Tauri config.
   Verify there is no stale endpoint, missing pubkey, or
   misleading UI affordance. Do not push tags or trigger
   production update checks.
4. **Sidecar lifecycle sanity check**: review the shipped
   prevention/recovery path from `fullstack-b-22` and
   `fullstack-b-25`. If empirical verification is needed,
   request @@Alex permission before launching chan-desktop.
5. **Report at task tail**: found / fixed / deferred, with
   severity and recommended follow-up task for each deferred
   item.
6. **Verification**: run the smallest useful checks first.
   At minimum, run the desktop crate/unit checks covering
   changed code. Broaden only if edits touch shared surfaces.

## How To Start

1. Read:
   * `desktop/CLAUDE.md` if present.
   * `desktop/src-tauri/tauri.conf.json`.
   * `desktop/src-tauri/capabilities/*`.
   * `desktop/src-tauri/src/main.rs` and nearby modules.
   * the task-history files listed above.
2. Inventory commands and capabilities before editing.
3. Prefer small patches over structural rewrites.
4. If any required fix leaves `./desktop`, stop and poke
   @@Desktect. @@Desktect routes the decision through @@Alex.

## Coordination

* @@Desktect owns task clearance and cross-team routing.
* @@Desktest owns runtime walkthroughs after permission.
* @@CI remains on chan-core; `release-desktop.yml` findings
  from `ci-15` route through @@Alex before this lane edits
  workflow YAML.
* @@Alex's running v0.12.0 chan.app session is off-limits.
* Team boundary: focus on `./desktop`. Do not edit outside
  `./desktop` unless @@Desktect has routed the need through
  @@Alex and @@Alex has bridged it with chan-core's
  @@Architect.

## 2026-05-23 - @@Desktacean audit report

Scope completed: desk audit only, no runtime launch, no code edits.

### Capabilities audit

Reviewed:

* `desktop/src-tauri/capabilities/default.json`
* `desktop/src-tauri/capabilities/drive.json`

Verdict:

| File | Permission class | Verdict |
|------|------------------|---------|
| `default.json` | `core:default` | Necessary for launcher IPC/event surface. |
| `default.json` | `dialog:default`, `dialog:allow-open` | Necessary: drive picker, update prompt, failure dialogs. |
| `default.json` | `deep-link:default` | Necessary: `chan://auth/callback` sign-in flow. |
| `default.json` | `opener:default`, `opener:allow-open-url` | Necessary: browser sign-in + open-in-browser actions. |
| `default.json` | `process:default`, `process:allow-restart` | Necessary: updater install then relaunch. |
| `default.json` | `updater:*` | Necessary: launcher calls `check()` + `downloadAndInstall()`. |
| `drive.json` | `core:webview:allow-set-webview-zoom` | Necessary: drive windows enable zoom hotkeys + zoom IPC. |
| `drive.json` | `opener:default`, `opener:allow-open-url` | Necessary: external links from drive windows open via Tauri opener. |

No obviously dead grants found. Drive windows intentionally have a
smaller capability set than launcher windows.

### IPC inventory

Inventoried all 24 Tauri commands:

* Drive registry / lifecycle: `list_drives`, `add_drive`,
  `remove_drive`, `set_drive_on`, `open_local_drive`,
  `open_tunneled_drive`, `chan_bin_status`.
* Lock recovery: `find_drive_lock_candidates`,
  `reclaim_drive_lock`.
* Feature / preflight: `compute_drive_preflight`,
  `get_drive_features`, `set_drive_features`.
* Tunnel listener: `tunnel_status`, `tunnel_start`,
  `tunnel_stop`.
* Launcher utilities: `get_config`, `home_dir`,
  `reveal_in_finder`.
* Window controls: `reload_window`, `open_devtools`,
  `zoom_in`, `zoom_out`, `zoom_reset`.
* Auth: `auth_status`, `open_signin`, `signout`.

Findings:

* Inputs are generally canonicalized or validated at the command
  boundary. Tunnel label/drive validation uses `chan_tunnel_proto`
  validators before persisting or rendering copy-paste snippets.
* Commands that spawn `chan` gate on `BinStatus` and use argv
  passing, not shell interpolation.
* `reveal_in_finder` takes a path string but only passes it as a
  single argv to the platform opener. Caller source is the registry
  row in `main.js`.
* Auth callback validates exact `chan://auth/callback`, validates
  the state nonce, and never exposes PAT secrets to JS status.
* No missing command registration found in `generate_handler!`.

No P0/P1 IPC patch identified.

### Updater review

Current state:

* `desktop/src-tauri/tauri.conf.json` has a production-shaped HTTPS
  endpoint:
  `https://chan.app/dl/desktop/{{target}}/{{current_version}}/latest.json`.
* `desktop/src/main.js::maybeOfferUpdate()` is the permanent caller
  that `systacean-12` initially found missing. It runs once per
  launcher process and prompts before install.
* `systacean-12` verified the macOS happy path against a mock feed:
  manifest fetch, version compare, platform selection, download,
  and minisign verification all passed before the expected fake
  bundle extraction failure.

Deferred P0:

* **P0: updater minisign key remains the DEV key**. This is already
  documented in `desktop/CLAUDE.md` and `desktop/release-review.md`.
  It cannot be fixed by code alone because it requires @@Alex /
  release-owner generation of the production minisign key, secret
  storage, and bridge-release sequencing. Recommended follow-up:
  cut a Desktect-owned release task for "updater production key
  rotation + bridge-release plan" before public DMG/update rollout.

Deferred P2:

* Linux/Windows updater install application remains unverified.
  `systacean-12` deferred it as a separate hands-on permission
  window. Recommended follow-up: Desktest runtime task once
  @@Alex grants access to the target machines/VMs.

### Sidecar lifecycle sanity check

Reviewed `fullstack-b-22` / `fullstack-b-25` against HEAD:

* `serve::start` puts child `chan serve` processes in their own
  process group on Unix and uses `CREATE_NEW_PROCESS_GROUP` on
  Windows.
* `serve::stop` removes the live handle before waiting, so fast
  off/on toggles no longer observe stale map state.
* `stop_child` sends SIGTERM with grace, escalates to SIGKILL, and
  waits.
* `RunEvent::Exit` and `AppState::drop` both call `stop_all`.
* SIGKILL of chan-desktop remains intentionally unrecoverable by
  prevention; recovery path enumerates positional `chan serve
  <key>` candidates and shows PID/command before reclaim.
* Mid-flight serve crash now emits `serve-crashed` and the launcher
  renders a soft notice. `desktop/release-review.md` still lists
  this as P0.4, but HEAD has the fix.

No additional P0/P1 sidecar patch identified without runtime
verification.

### Stale release-review notes

`desktop/release-review.md` is useful but partially stale:

* P0.3 tracing subscriber: fixed in HEAD via `init_tracing()`.
* P0.4 mid-flight crash silent: fixed in HEAD via `SERVE_CRASHED`.
* P0.5 stop-then-start race: fixed in HEAD by removing the live
  serve handle before waiting in `serve::stop`.

Recommended follow-up: Desktect doc task to refresh or supersede
`desktop/release-review.md` so it does not keep reporting fixed P0s.

### Verification

Ran:

```
cargo test -p chan-desktop --bin chan-desktop
```

Result: 63 passed, 0 failed.

### Closeout

Fixed: none.

Deferred:

* P0 updater production minisign key rotation and bridge-release
  sequencing.
* P2 Linux/Windows updater install verification.
* P3 refresh stale `desktop/release-review.md` findings.

Ready for @@Desktect clearance.

## 2026-05-23 - @@Desktect closure

Audit report accepted. `desktacean-1` is closed as
audit-complete with no code edits.

Verdict:

* No P0/P1 desktop code patch identified inside `./desktop`.
* The active P0 is updater production minisign key rotation.
  That crosses release-owner / secret-management scope, so it
  is routed to @@Alex rather than assigned as a direct worker
  patch.
* `desktop/release-review.md` stale-P0 cleanup is a follow-up
  doc task, not a blocker for closing this audit.

No commit clearance needed for product code. Journal/event docs
remain part of @@Desktect's bootstrap dispatch bundle.
