# v0.66.0-rc1 UI Part 1

v0.66.0-rc1 starts from `origin/main` at `v0.65.0` (`1c47ed71`) on branch `release-v0660`, worktree `../chan-v0660`. This rc is a pin state only: no rc tag, no GA tag, and no publish=true release workflow.

## Scope

- Settings opens with focus, supports keyboard navigation through its section list, and returns focus to the active tab after close.
- Pane A/B side flips keep one visual rotation direction across A to B and B to A.
- Empty-pane dotted waves are 25% taller and anchored below the absolute path label.
- Launcher startup subscribes to the window feed before workspace/devserver registry restoration settles, so local terminal creation is not blocked by slow listing.
- Reconnect and Abandon show pending/error state and use one cached window-label-to-devserver lookup for loopback and tunnel windows.
- macOS desktop updater completion emits `desktop-update-ready { version }`; the launcher shows the restart dialog and calls the narrow `restart_desktop_after_update` app command through a launcher-scoped capability.
- `chan devserver --service=chan` is the portable background daemon backend with detached `__devserver-daemon`, log redirection, readiness waiting, idempotent start, join-as-watchdog, and status/stop/restart over the same pidfile.

## Commit Range

- Base: `1c47ed71` (`v0.65.0`, `origin/main`).
- Branch: `release-v0660`.
- Range: `1c47ed71..release-v0660`.
- Commits:
  - `eeec91b3` `feat(release): open 0.66.0-rc1`
  - this report/changelog commit

## Validation

Completed focused checks before full-gate closeout:

- `cargo fmt`
- `cargo fmt --check`
- `cargo check -p chan`
- `cargo check -p chan-desktop`
- `cargo test -p chan --lib devserver`
- `cargo test -p chan --test devserver_resilience chan_service_ -- --test-threads=1`
- `cargo test -p chan-desktop desktop_update`
- `cargo test -p chan-desktop devserver_window_label_lookup`
- `cargo test -p chan-desktop launcher_update_capability`
- `cargo test -p chan-desktop default_capability`
- `npm install` in `web/`
- `npm install --package-lock-only` in `web/`
- `cargo update -w`
- `cargo update -w` in `gateway/`
- `npm run test -w @chan/launcher -- App.test.ts api/desktop.test.ts`
- `npm run test -w @chan/launcher -- App.test.ts api/desktop.test.ts state/library.svelte.test.ts`
- `npm run test -w @chan/workspace-app -- components/SettingsOverlay.test.ts components/Pane.test.ts components/disconnectOverlay.test.ts components/dashboardTabAndCarousel.test.ts api/desktop.test.ts`
- `npm run test -w @chan/workspace-app -- components/SettingsOverlay.render.test.ts components/SettingsOverlay.test.ts components/Pane.test.ts components/disconnectOverlay.test.ts components/dashboardTabAndCarousel.test.ts api/desktop.test.ts`
- `npm run check -w @chan/launcher`
- `npm run check -w @chan/workspace-app`
- `git diff --check`
- Stale version-pin check: no `0.65.0` remains in release pin surfaces; no `v0.66.0*` tag exists.

Pending before rc acceptance:

- Run full `make pre-push`.
- Push `origin/release-v0660`.
- Dispatch `.github/workflows/release.yml` on `release-v0660` with `publish=false`.
- Record the workflow run id and result here after dispatch.

## Dry Run

- Workflow: `.github/workflows/release.yml`
- Ref: `release-v0660`
- Input: `publish=false`
- Run id: pending
- Result: pending

## Known Limitations

- Windows desktop self-upgrade remains unsupported.
- Linux desktop self-upgrade is not claimed in rc1; no signed AppImage updater payload/feed support was added or validated.
- macOS updater signing/notarization and the update-ready event path require the `publish=false` workflow dry run for validation.
- Manual Reconnect/Abandon smoke on live loopback and tunnel chan-desktop windows remains pending host verification.
- No rc tag or GA tag is created for this candidate.
