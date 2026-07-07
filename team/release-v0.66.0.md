# v0.66.0 Release

v0.66.0 closes the `release-v0660` cycle that started from `origin/main` at `v0.65.0` (`1c47ed71`). The RC branch was validated as a pin state only. No RC tag was pushed.

## Scope

- Settings opens as a focused overlay, supports keyboard navigation through its section list, and restores focus to the active Terminal or Editor tab when closed.
- Pane A/B side flips keep one visual rotation direction across A to B and B to A.
- Empty-pane dotted waves fill the bottom field, start at the top of that field below the workspace path, and stay pinned to the bottom edge while the window resizes.
- Editor tab menus now expose Copy path to file, Delete, and Duplicate between Page width and Close.
- Launcher startup subscribes to the window feed before workspace/devserver registry restoration settles, so local terminal creation is not blocked by slow listing.
- Reconnect and Abandon show pending/error state and use one cached window-label-to-devserver lookup for loopback and tunnel windows.
- macOS desktop updater completion emits `desktop-update-ready { version }`; the launcher shows the restart dialog and calls the narrow `restart_desktop_after_update` app command through a launcher-scoped capability.
- Windows release packaging signs the CLI exe, desktop exe, and NSIS installer through SSL.com eSigner, verifies Authenticode signatures, and keeps the public `release-windows` artifact contract.
- `chan devserver --service=chan` is the portable background daemon backend with detached `__devserver-daemon`, log redirection, readiness waiting, idempotent start, join-as-watchdog, and status/stop/restart over the same pidfile.

## Branch And Commits

- Base: `1c47ed71` (`v0.65.0`, `origin/main`).
- Branch: `release-v0660`.
- GA target: `main` fast-forwarded to the GA pin commit, then annotated tag `v0.66.0`.
- Accepted commits before the GA pin:
  - `eeec91b3` `feat(release): open 0.66.0-rc1`
  - `a7d48716` `docs(release): add 0.66.0-rc1 report`
  - `fb5be898` `feat(release): finish v0.66.0 ui`
- GA pin commit: the commit that adds this report and strips `-rc1` from release pins.

## Validation

Focused checks completed before GA closeout:

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
- `npm run test -w @chan/launcher -- App.test.ts api/desktop.test.ts`
- `npm run test -w @chan/launcher -- App.test.ts api/desktop.test.ts state/library.svelte.test.ts`
- `npm run test -w @chan/workspace-app -- components/SettingsOverlay.render.test.ts components/SettingsOverlay.test.ts components/Pane.test.ts components/disconnectOverlay.test.ts components/dashboardTabAndCarousel.test.ts api/desktop.test.ts`
- `npm run test -w @chan/workspace-app -- components/editorRightClickRevamp.test.ts components/dashboardTabAndCarousel.test.ts`
- `npm run check -w @chan/launcher`
- `npm run check -w @chan/workspace-app`
- `npm run check -w @chan/marketing`
- `git diff --check`

Host smoke:

- The part 2 browser smoke was accepted by the release owner and is treated as the RC pass for the final UI bits.
- The Windows marketing download contract was rechecked after signing changes. The release workflow still stages `release-windows` with `Chan_${VERSION}_x64-setup.exe` and `chan-x86_64-pc-windows-msvc.zip`; the marketing metadata still resolves `desktop-windows-nsis` and `cli-windows-x64` to those assets.

Required pre-tag gate:

- Run `make pre-push` on the GA commit before pushing `main` or `v0.66.0`.

## Release Workflow

RC dry run:

- Workflow: `.github/workflows/release.yml`
- Ref: `release-v0660`
- Input: `publish=false`
- Run id: `28851020400`
- URL: `https://github.com/fiorix/chan/actions/runs/28851020400`
- Head SHA: `a7d48716c8dd49c3db8a5a4be23e23795f9f46ec`
- Result: passed

GA publish:

- Trigger: push annotated tag `v0.66.0` after the pre-tag gate passes.
- Expected workflow: `.github/workflows/release.yml` with publish semantics from the tag.

## Known Limitations

- Windows desktop self-upgrade remains unsupported. The Windows installer and CLI artifacts are signed, but the desktop updater path is not claimed for Windows.
- Linux desktop self-upgrade is not claimed; signed AppImage updater payload/feed support was not added or validated.
- Windows signing requires the SSL.com eSigner workflow secrets to be present in GitHub Actions.
