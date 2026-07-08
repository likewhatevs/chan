# v0.66.1 Release

v0.66.1 closes the `release-v0661` cycle that started from `origin/main` at `v0.66.0` (`aef78ecb`). Two release candidates were validated as pin states; no rc tag was pushed.

## Scope

- Devserver control terminals resolve deterministically: the daemonize handshake (clean script exit within a 10s registration grace) keeps the connection and reaps the terminal; any later script exit stops the connection, with a failing exit keeping its terminal readable while windows close.
- Reconnect and Abandon kill a still-running connect script and drive the disconnect (and, for Reconnect, connect) flows unconditionally.
- Devservers bind stable per-library control-socket paths, so `$CHAN_CONTROL_SOCKET` in open shells survives restarts; the stable name is a single hash that fits macOS socket dirs.
- Terminals restored across a restart through the systemd fd store exit cleanly (no EIO error, no fabricated exit 1; the exit frame's code is optional on the wire).
- Terminal surveys queue per target through a bounded server-side FIFO with an explicit queue-full error.
- The rich prompt persists caret, height, and content across tab switches, window switches, and reloads, without stealing focus from the background.
- Excalidraw embeds: View opens the pan/zoom overlay, Edit reveals source, failed embeds are clickable-to-fix; rendered diagrams (mermaid, mermaid-to-excalidraw, excalidraw) copy to the clipboard as PNG.
- Editor tab menu: single separator, no misleading Backspace hint; the command launcher no-ops on a no-match Enter; standalone servers stop probing the desktop-only focus-colour websocket.
- Empty pane: no workspace-path label, mark hides on short panes; the app-spawn rows live in the pane hamburger (workspace windows only); New slide deck seeds the slides frontmatter and opens with the caret on the first heading.
- Group broadcast gets a macOS desktop chord (Cmd+Shift+I); the dev-server form tip is shortened.

## Branch And Commits

- Base: `aef78ecb` (`v0.66.0`, `origin/main`).
- Branch: `release-v0661`.
- GA target: `main` fast-forwarded to the GA pin commit, then annotated tag `v0.66.1`.
- Accepted commits: the rc1 set (`5236f1f0`, `1c91af74`, `0a3a7ff8`, `d7085700`, `6e7839a1`), the rc2 set (`8b59e181`, `8298f328`, `d1da3711`), and the post-rc2 polish (`6b863dab`, `e03a8807`, `8331a3e0`, `d319f0a2`), with the docs/pins commits between.
- GA pin commit: the commit that adds this report, strips `-rc2` from every pin, and cuts the CHANGELOG `[v0.66.1]` section.

## Validation

- Per-lane gates each round: scoped clippy + tests + fmt for the Rust lanes (chan-server 553, chan-shell 85, chan-library 194, chan 124, chan-desktop 173 at their respective tips), svelte-check plus the full workspace-app vitest for the web lanes (2547 tests at the last full run), launcher tests for the dialog change.
- Adversarial review on every lane diff; findings fixed before commit (SPA exit-frame contract, launcher ArrowUp wrap, canvas test hygiene, terminal-only hamburger gating, chord supersede contract).
- Full `make pre-push` green at `44634cd4` (rc2) and required again on the GA commit before tagging.
- Headless-Chrome browser verification against real test-server builds: rc1 4/4, rc2 7/7 (including a real clipboard PNG write); evidence in the owner's dev tree.
- Host smoke by the release owner: rc1 pass validated five of the six original bugs and spawned the rc2 scope; the rc2/rc3-state browser pass was accepted on the live test server ("just tested, it worked").

## Release Workflow

- Run 1 (rc1, `68d5a791`): FAILURE, macOS-only: the stable control-socket name overflowed the 104-byte macOS `sun_path`; fixed by `d7085700`.
- Run 2 (rc1, `8e5fd15c`): SUCCESS. `https://github.com/fiorix/chan/actions/runs/28891608356`
- Run 3 (rc2, `77b151f3`): SUCCESS. `https://github.com/fiorix/chan/actions/runs/28906533900`
- Run 4 (GA pins, `68758bf4`): `publish=false`, SUCCESS. `https://github.com/fiorix/chan/actions/runs/28921365205`
- GA publish: annotated tag `v0.66.1` on `68758bf4`, pushed after the pre-tag gate and run 4 passed; publish run `28923462155`.

## Known Limitations

- Grace-window heuristics on control-terminal exits: a clean ^C within 10s of a connection (re)registration reads as the daemonize handshake; a handshake slower than 10s would tear down a healthy connect. The constant is fn-local and tunable.
- Survey queueing keys on the resolved target; overlapping group surveys are not serialized against each other.
- The desktop clipboard-image IPC is PNG-only; SVG-on-clipboard was not taken.
- One-time upgrade effects: shells opened under v0.66.0 or earlier keep the old pid-scoped `$CHAN_CONTROL_SOCKET` until respawned; sessions saved before the rich-prompt change restore the caret at the start once.
- Windows desktop self-upgrade remains unsupported (unchanged from v0.66.0).
