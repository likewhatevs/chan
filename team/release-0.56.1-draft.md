# v0.56.1 draft

## Control-terminal exit attention

- Script-backed devserver control terminals now treat the control script as the connection owner. Any script completion, including exit code 0, nonzero status, Ctrl-C / SIGINT, SIGTERM, or an unknown exit state, marks the devserver connection dead.
- Terminal exit state is now explicit and sticky at the control tenant level, so desktop polling can still observe the exit after the terminal websocket removes the live session.
- The desktop starts the control-exit watcher immediately after registering the control tenant prefix, before token scraping and devserver watcher setup. The watcher uses the captured script-backed run kind and generation, so later config edits or overlapping connect attempts cannot suppress or misroute the event.
- Stale concurrent connect attempts are coalesced/generation-checked so an old control process cannot overwrite the active prefix or emit against a newer run.
- The launcher keeps retained control rows visible for disconnected devservers when attention is pending, resolves pending attention by the stable `control-terminal-{devserverId}` row first, and only clears the flash after a real reconnect or successful focus/show action.
- Explicitly closing the control terminal window now reaps its launcher row instead of leaving stale attention behind; script/PTY exit still keeps the row flashing so the user can inspect or re-run it.
- The flashing control cue now says `disconnected...` instead of `reconnecting...`, including the marketing launcher demo mock.
- The devserver design doc now calls out the foreground-script contract and recommends long-running scripts use a foreground command such as `exec ssh -N ...` rather than daemonizing.

## Launcher hover polish

- Machine cards now own hover wobble as whole-card motion.
- Buttons and workspace cards no longer wobble; they keep color and background hover cues only.
- The window-count badge toggles workspace window visibility.
- The launcher demo spacing under the library header was bumped.

## Desktop package targets

- macOS and Windows desktop packaging are now split instead of sharing the old generic Tauri `build` path. `make macos-chan-dmg` builds the unsigned local `.app` with `--no-sign` and wraps it with the Finder-less DMG builder; signed/notarized release builds still use `make macos-chan-dmg-notarised`.
- Windows NSIS settings moved from the base Tauri config into `tauri.windows.conf.json`, so macOS builds no longer validate Windows-only installer config.
- `make windows-chan-installer` now mirrors the CI Windows packaging path: build both web bundles, build the console `chan.exe`, then run `cargo tauri build --bundles nsis --config tauri.windows.conf.json`.

## Validation

- `cargo test -p chan-library terminal`
- `cargo test -p chan-desktop control_window_close_reaps_without_attention_event`
- `cargo test -p chan-server terminal`
- `cargo check -p chan-desktop`
- `make macos-chan-dmg`
- `make -n windows-chan-installer`
- Windows overlay schema check: `cargo tauri build --config tauri.windows.conf.json --no-bundle --no-sign` progressed past config validation and stopped on macOS at the expected missing `target/release/chan.exe` sidecar.
- `npm --prefix web run test --workspace @chan/launcher`
- `npm --prefix web run check --workspace @chan/launcher`
- `npm --prefix web run build --workspace @chan/launcher`
- `npm --prefix web run check --workspace @chan/marketing`
- `npm --prefix web run build --workspace @chan/marketing`
