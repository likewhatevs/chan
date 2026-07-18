# Release v0.70.2

Patch round run 2026-07-18 off the v0.70.1 tag: six fixes implemented by orchestrated implementer agents on the `v0.70.2-dev` branch, integrator-owned diagnosis and gate, and shipped without an rc cycle by the owner's call. Coordination artifacts live in the untracked `dev/v0.70.2/` tree of the round host's checkout.

## Scope

Six fixes. Two are the same regression from v0.70.0's terminal-socket reconnect kit (`9511228a`), which was applied to every terminal: on a redial that cannot reattach the prior session the server spawns a fresh session running the tenant's default command. The control terminal (whose default command is the connect script) re-ran it in a loop, so it is now excluded from the kit entirely (no heartbeat, read deadline, connect deadline, or auto-redial). A normal terminal left idle had its resumable session id discarded by the attach budget after a handful of offline redials, replacing the running process with a fresh shell that inherited the dead program's mouse-tracking mode; the budget no longer clears the id (only the server's explicit close does), and a genuinely fresh shell now resets the terminal input modes first. Editor and slides: inline markdown renders inside table cells; exported slide decks constrain embedded Excalidraw diagrams to the slide (an absolute-pixel max-width that survives the PDF foreignObject rasterization); the New slide deck seed carries the default `zoom_factor: 2`; the editor page-width cap moved from the scroller to the content so the scrollbar sits at the window edge and the off-page band scrolls.

A seventh reported item (`chan upgrade --version <x>`) was investigated and deferred by the owner: the CLI threads the version correctly, but the deploy hosts only the latest per-version metadata at `/dl/cli/vX.Y.Z.json`, so a pinned non-latest version 404s. The fix is a deploy-pipeline change owned separately.

## Branch And Commits

`v0.70.2-dev` cut from v0.70.1 (`b9db8c95`); five fix commits plus the GA pin commit. The terminal fix is one commit (bugs 1 and 6 share `TerminalTab.svelte` and the single root); the four editor and slides fixes are one commit each. The GA commit bumps the pins straight to 0.70.2, dates the CHANGELOG, pins the fedora specs, and adds this document.

## Validation

Each lane was implemented by a subagent and own-gated (svelte-check plus the full workspace-app vitest, 2826 tests; per-crate clippy `-D warnings`, focused cargo tests, and `cargo fmt --check` after the last edit). The integrator verified every diff against live code before committing, and independently confirmed the two terminal regressions by git archaeology (ruling out the exit watcher, connect idempotency, the control-tenant PTY, and the launcher poll before pinning `9511228a`). Full `make pre-push` green on the fix tip, both workspaces, including the gateway build, web-check with the full vitest and production build, and the marketing smokes. Bug 2 carries a jsdom CodeMirror render test asserting `<strong>` inside a cell; bug 4 a byte-exact seed test. The served bundle was grep-confirmed to carry every fix (freshness check against a stale embed). The interactive headless-browser smoke of bugs 3 and 5 was blocked by an unrelated cap-std file-open quirk on the tmpfs test workspace; those two are best confirmed by an owner eyeball on the build.

Bugs 1 and 6 cannot be reproduced on the Linux round host (they need the macOS desktop plus a lima or tunnel devserver); the owner host-smoke owns them.

## Release Workflow

No rc pin cycle for this patch. GA validation via a `release.yml publish=false` dispatch on the GA-pinned branch (the only macOS compile, sign, and notarize signal), artifacts checked before the tag; GA is the standard tag-push publish, owned by @fiorix, with distros-publish verified after.
