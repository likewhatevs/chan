# @@Systacean's phase-7 journal

Author: @@Systacean
Date: 2026-05-18

@@Systacean is the merged Syseng + Rustacean profile for phase 7.
Owns Rust code quality, the build, CI, dependencies, the pinned
toolchain, and overall project hygiene.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-18 11:29 BST - Bootstrap blocked

Bootstrapped @@Systacean in the phase 7 tree.

Read:

* `docs/agents/systacean.md`
* `docs/agents/systacean/skills/syseng.md`
* `docs/agents/systacean/skills/rustacean.md`
* `docs/journals/phase-7/process.md`
* `docs/journals/phase-7/request.md`
* `docs/journals/phase-7/systacean/journal.md`
* `CLAUDE.md`

Note: the bootstrap prompt referenced `docs/journals/phase-7/`, but the
existing phase directory is `docs/journals/phase-7/`.

Blocked: no `docs/journals/phase-7/systacean/systacean-*.md`
task files exist, so there is no numbered task to start.

## 2026-05-18 12:49 BST - systacean-2 ready for review

Resumed after terminal recycle and completed the write-timeout pass.

Changed:

* `crates/chan-server/src/routes/files.rs`
* `crates/chan-drive/src/drive.rs`
* `web/src/state/tabs.svelte.ts`
* `web/src/state/tabs.test.ts`

Diagnosis and verification are in
[systacean-2.md](systacean-2.md#2026-05-18-1249-bst---diagnosis-and-fix).

Did not touch frozen `systacean-1` files.

## 2026-05-18 14:55 BST - systacean-4 ready, uncommitted

With `systacean-5` blocked on pending @@FullStack commits,
used the idle-task allowance for `systacean-4`.

Implemented `chan open <dir>` directory-enter semantics:
the control socket now emits `enter: true` for directory
browser commands, and the frontend expands the target
directory itself so its contents load directly.

Verification:

* `cargo fmt --check`
* `cargo test -p chan-server control_socket`
* `cd web && npm run test -- src/state/store.test.ts`

No commit made. Full pre-push gate still required.

## 2026-05-18 16:55 BST - round 1 closeout pushed

Closed out round 1 and pushed `main` plus tag `v0.10.1`.

Commits:

* `f8014a9` - restored terminal prompt mode toggle after
  `npm run test` caught the shared-toolbar regression.
* `f996f4c` - committed `systacean-4`.
* `9e48367` - bumped version to `0.10.1`.

Verification and bundle details are in
[systacean-5.md](systacean-5.md#2026-05-18-1655-bst---closeout-shipped).

## 2026-05-18 17:18 BST - systacean-3 ready for review

Resumed post-recycle and completed the cross-drive static-serving
mitigation in [systacean-3.md](systacean-3.md).

Changed:

* `crates/chan-server/src/static_assets.rs`

Verification:

* `cargo fmt --check`
* `cargo test -p chan-server static_assets`
* `cargo clippy -p chan-server --all-targets -- -D warnings`

Known gap: @@WebtestB still needs to re-run the 8801 / 8810 drift
repro after the patch lands.

## 2026-05-18 17:46 BST - systacean-3 pushed

Committed and pushed `systacean-3`.

Commit:

* `f94c4b5` - Scope SPA shell and asset caching per
  chan-serve instance

Gate:

* `scripts/pre-push` - passed

Pushed `main` to `origin/main`.

## 2026-05-18 18:11 BST - systacean-6 ready to land

Scoped the SPA bearer token and per-tab session window id storage
keys by `window.location.origin` plus injected chan prefix, and
documented the scheme in the static-assets module and design doc.

Verification:

* `cd web && npm run test -- src/api/client.test.ts`
* `cd web && npm run check`
* `cd web && npm run test`
* `scripts/pre-push`

## 2026-05-18 18:34 BST - systacean-7 ready to gate

Fixed the desktop Makefile signing environment so empty
`APPLE_SIGNING_IDENTITY` / `APPLE_TEAM_ID` values are not exported
into default Tauri builds.

Verification so far:

* `make -C desktop build` outside sandbox
* `cargo tauri build --bundles app`
* mounted `target/release/bundle/dmg/Chan_0.10.1_aarch64.dmg`
  and verified the standard `Chan.app` plus `Applications`
  symlink layout
* `scripts/pre-push` in a clean temporary worktree with only
  the `systacean-7` Makefile patch applied

## 2026-05-18 18:43 BST - systacean-8 ready to land

Fixed terminal scrollback loss on browser reload by no longer
persisting/restoring the terminal replay cursor (`tseq`) in
per-window session layout. Reloaded terminal tabs keep the
server PTY session id but reconnect from the start of the
server-side ring, so the fresh xterm buffer is repopulated.

Verification:

* `cd web && npm run test -- src/state/tabs.test.ts src/terminal/session.test.ts`
* `cd web && npm run check`
* `cd web && npm run test`
* `scripts/pre-push` passed on rerun after a transient
  `Too many open files` failure in an unrelated chan-drive test

## 2026-05-18 19:10 BST - online, starting systacean-9

Online, starting [systacean-9.md](systacean-9.md).

## 2026-05-18 19:38 BST - systacean-9 landed

Implemented and committed the terminal-scoped event watcher.

Commit: `Add terminal-scoped event watcher`.

Verification:

* `scripts/pre-push`

## 2026-05-18 20:18 BST - systacean-8 B19 follow-up investigated

Investigated @@WebtestB's B19 reload/scrollback FAIL and left a
candidate server-side reattach patch uncommitted pending follow-up
task / commit clearance.

Notes:

* [systacean-8.md](systacean-8.md#systacean-follow-up--2026-05-18-2018-bst)
* Verification on candidate patch: `scripts/pre-push`

## 2026-05-18 20:31 BST - systacean-8 B19 follow-up pushed

Committed and pushed the B19 reattach follow-up.

Commit:

* `1cd4ef2` - Reattach terminal PTY by window and tab

Verification:

* `scripts/pre-push`

## 2026-05-18 20:46 BST - systacean-11 ready

Implemented the chan-server survey-reply atomic-write seam and
passed the gate.

Notes:

* [systacean-11.md](systacean-11.md#2026-05-18-2046-bst---ready-to-land)

Verification:

* `cargo test -p chan-server event_reply --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `scripts/pre-push`

## 2026-05-18 20:48 BST - systacean-11 pushed

Committed and pushed the survey-reply atomic-write seam.

Commit:

* `530e30f` - Add terminal event-reply writer

## 2026-05-18 20:56 BST - systacean-10 ready

Verified `systacean-6` storage scoping is not load-bearing for the
documented warm-cache drift once `systacean-3` cache headers are in
place. Reverted the storage key namespacing and docs.

Notes:

* [systacean-10.md](systacean-10.md#2026-05-18-2056-bst---verification-and-decision)

Verification:

* `npm run test -- src/api/client.test.ts`
* `npm run check`
* `npm run build`
* `cargo test -p chan-server static_cache_headers --no-default-features`
* `cargo fmt --check`

## 2026-05-18 21:02 BST - systacean-10 pushed

Committed and pushed the storage scoping revert.

Commit:

* `4ca7dc4` - Revert SPA storage key scoping

## 2026-05-19 04:49 BST - systacean-12 started

New Wave-B assignment received. Starting
[systacean-12.md](systacean-12.md): HTTP agent control
channel for terminal spawn / restart / close.

## 2026-05-19 04:58 BST - systacean-12 ready

Implemented and gate-verified the HTTP terminal control channel.

Notes:

* [systacean-12.md](systacean-12.md#2026-05-19-0458-bst---ready-to-land)

Verification:

* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `scripts/pre-push`

## 2026-05-19 04:58 BST - systacean-12 pushed

Committed and pushed the HTTP terminal control channel.

Commit:

* `314a68b` - Add HTTP terminal control channel

## 2026-05-19 05:03 BST - systacean-13 started

New Wave-B queue item started:
[systacean-13.md](systacean-13.md), terminal tab activity
indicator for output since last focus.

## 2026-05-19 05:10 BST - systacean-13 ready

Implemented and gate-verified terminal activity indication for
PTY output since last focus.

Notes:

* [systacean-13.md](systacean-13.md#2026-05-19-0510-bst---ready-to-land)

Verification:

* `cargo test -p chan-server activity --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `npm run test -- src/components/Pane.test.ts src/state/tabs.test.ts`
* `npm run check`
* `npm run build`
* `scripts/pre-push`

## 2026-05-19 05:12 BST - systacean-13 pushed

Committed and pushed the terminal tab activity indicator.

Commit:

* `1694041` - Add terminal tab activity indicator

## 2026-05-19 05:32 BST - systacean-14 started

New Wave-B queue item started:
[systacean-14.md](systacean-14.md), MCP auto-discovery for
external agents launched inside chan terminals.

## 2026-05-19 05:37 BST - systacean-14 ready

Implemented and gate-verified MCP auto-discovery publication for
Claude Code, Codex, and Gemini CLI.

Notes:

* [systacean-14.md](systacean-14.md#2026-05-19-0537-bst---ready-to-land)

Verification:

* `cargo test -p chan-server mcp_discovery --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `scripts/pre-push`
