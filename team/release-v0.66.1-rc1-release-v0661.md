# v0.66.1-rc1 - release-v0661

Bugfix candidate off `v0.66.0` (`aef78ecb`) fixing the six bugs the release owner reported after the v0.66.0 cut. Three file-disjoint lanes, integrated and gated on the `release-v0661` branch in the `../chan-v0661` worktree.

## Scope

- Rich prompt: stays mounted across tab switches; caret and bubble height persist per terminal across tab switches, window switches, and reloads; background deliveries do not steal focus.
- Excalidraw embeds: View button opening the pan/zoom diagram overlay on the rendered SVG; Edit reveals the source markdown instead of opening the raster image bubble with a broken preview over the source.
- Devserver control terminals: a clean script exit with the connection up auto-reaps the control terminal (no down-mark, no reconnect block); failing or premature exits keep the terminal so the failure is readable. Reconnect is teardown-then-connect and Abandon is teardown, both killing a still-running script first.
- Control sockets: a devserver binds stable per-library control-socket paths and a restarted instance rebinds them, so `$CHAN_CONTROL_SOCKET` in already-open shells survives restarts; window-spawned servers keep pid-scoped paths. Discovery resolves stable-named sockets with a bounded Identify probe carrying the server pid.
- Restored terminals: EIO after slave close on an fdstore-restored PTY is a clean end-of-stream; the exit frame's code is optional and a restored session exits codeless instead of with a fabricated 1.
- Editor tab menu: single separator between Page width and Copy path to file.

## Branch And Commits

- Base: `aef78ecb` (`v0.66.0`, `origin/main`).
- Branch: `release-v0661`.
- Commits:
  - `5236f1f0` `fix(server): stable control sockets and clean restored-pty exits`
  - `1c91af74` `fix(desktop): reap clean-exit control terminals, revive reconnect`
  - `0a3a7ff8` `fix(web): rich prompt persistence, excalidraw actions, menu separator`
  - `f864a44a` `docs(release): add v0.66.1 unreleased notes`
  - `fee6dcd6` `chore(release): open 0.66.1-rc1`

## Validation

Per-lane gates (each run in the worktree after the lane's last edit):

- Web: `npm run check -w @chan/workspace-app` (0 errors); full `npm run test -w @chan/workspace-app` (264 files, 2518 tests, green, no flake re-runs needed); targeted isolation run of every lane-touched test file.
- Desktop: `cargo clippy -p chan-desktop --all-targets -- -D warnings`; `cargo test -p chan-desktop` (172 passed, including the rewritten `control_script_clean_exit_reaps_and_failed_exit_keeps_terminal` and the kept `workspace_poll_emits_control_attention_while_still_connected` and `token_rotation_retires_old_watcher_without_closing_windows`); `cargo fmt -p chan-desktop --check`.
- Server: `cargo clippy -p chan-shell -p chan-server -p chan-library -p chan --all-targets -- -D warnings`; `cargo test -p chan-shell -p chan-server -p chan-library` (817 passed); `cargo test -p chan` (124 passed, including the new end-to-end `devserver_restart_rebinds_the_same_control_socket_paths`); `cargo fmt` scoped check; `cargo check -p chan-desktop` cross-crate safety pass.

Integration:

- Each lane's diff was adversarially reviewed against its brief; the one blocker (the SPA still rendered a mandatory exit code after the wire made it optional) was fixed in `TerminalTab.svelte` and re-verified before commit.
- Full `make pre-push` green on the branch at `f864a44a` (fmt, clippy, all-targets tests, no-default-features build, gateway build, web-check, web-marketing-check).

Host smoke: the per-bug pass is the release owner's `dev/v0.66.1/host-smoke.md`; bugs 3 (desktop WKWebView), 4, and 5 (systemd user-service restart path) are only reachable on the owner's setup.

## Release Workflow

RC dry run:

- Workflow: `.github/workflows/release.yml`
- Ref: `release-v0661`
- Input: `publish=false`
- Run id / result: pending dispatch.

No `v*` tag is pushed for an rc; the rc is a pin state only.

## Known Risks

- The new control-socket cfg surface (stable path, flock takeover, Windows named-pipe branch) compiles for Windows only in CI; the `publish=false` dry run is the verification vehicle before any tag.
- `Identity` gained a required `pid` field. During version skew a new CLI probing a pre-v0.66.1 server degrades to no-match (`chan ps` BY column shows `-`); accepted under the pre-release no-back-compat norm.
- Shells opened under v0.66.0 or earlier still carry the old pid-scoped `$CHAN_CONTROL_SOCKET` until respawned; sessions saved before the rich-prompt change restore the caret at offset 0 once. Both one-time upgrade effects.
- Decided-semantics tradeoffs, changelog-noted: a transport script that exits cleanly mid-session is reaped (outage surfaces via the workspace poll), and a clean-exit script whose devserver never answers fails only after the full connect dial budget.

## Changelog

The `CHANGELOG.md` `[Unreleased]` section added in `f864a44a` carries the user-facing entries for all six fixes; the GA cut renames it to `[v0.66.1]`.
