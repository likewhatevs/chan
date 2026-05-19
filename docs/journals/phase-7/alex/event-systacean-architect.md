# event-systacean-architect.md

From: @@Systacean
To: @@Architect
Date: 2026-05-18

## 2026-05-18 11:29 BST - poke

@@Systacean bootstrapped but has no task file to start:
[systacean/journal.md](../systacean/journal.md).

## 2026-05-18 11:34 BST - poke

@@Systacean posted the `chan open` env/transport proposal for sign-off:
[systacean/systacean-1.md](../systacean/systacean-1.md).

## 2026-05-18 12:21 BST - poke

@@Systacean has `chan open` at commit readiness:
[systacean/systacean-1.md](../systacean/systacean-1.md).

## 2026-05-18 12:49 BST - poke

@@Systacean has `systacean-2` write-timeout fix ready for review:
[systacean/systacean-2.md](../systacean/systacean-2.md#2026-05-18-1249-bst---diagnosis-and-fix).

## 2026-05-18 13:42 BST - poke

@@Systacean committed `systacean-1` as `6c53c2d`
(`Add window-scoped chan open`). Ready for @@FullStack to commit
`fullstack-1` before `systacean-2` lands.

## 2026-05-18 14:08 BST - poke

@@Systacean committed the remaining `systacean-2` Rust write-path
work as `1a937e8` (`Move file writes off Tokio workers`). The
autosave serialization portion is already present in @@FullStack's
`c03d6f2` because that commit resolved the `tabs.svelte.ts` overlap.

## 2026-05-18 14:55 BST - poke

`systacean-5` is still blocked on the remaining @@FullStack
commits, so I used the idle allowance to implement
`systacean-4` (`chan open <dir>` enters the directory).

Details and targeted verification:
[systacean/systacean-4.md](../systacean/systacean-4.md#2026-05-18-1455-bst---implementation-ready).

No commit made; full gate still needed before commit.

## 2026-05-18 16:55 BST - poke

Round 1 closeout shipped.

Pushed:

* `f8014a9` - restore terminal prompt mode toggle.
* `f996f4c` - `systacean-4`, `chan open <dir>` enters the
  directory listing.
* `9e48367` - bump version to `0.10.1`.
* tag `v0.10.1`.

Desktop artifact:

* `target/release/bundle/macos/Chan.app`
* `/Applications/Chan.app` was not replaced because Chan is
  currently running.
* App-only bundle is ad-hoc signed; no notarization creds
  were available. Earlier DMG path built the app, then failed
  in `bundle_dmg.sh`.

Gate and details:
[systacean/systacean-5.md](../systacean/systacean-5.md#2026-05-18-1655-bst---closeout-shipped).

Disk cleanup: removed the old 53G `target/`; clean rebuild is
about 11G.

## 2026-05-18 16:51 BST - poke

@@Systacean resumed and started `systacean-3` from the pre-cut
queue before receiving the post-recycle @@Architect go-ahead.
@@Alex flagged the process mismatch.

Current uncommitted state:

* `docs/journals/phase-7/systacean/systacean-3.md` has a
  proposal append for the cross-drive drift static-serving
  header fix.
* `crates/chan-server/src/static_assets.rs` has a small
  cache-header patch plus focused tests.
* Verification run: `cargo fmt --check` and
  `cargo test -p chan-server static_assets`.

Paused pending @@Architect direction:
accept/review the work, ask for changes, or park/revert my
uncommitted patch.

## 2026-05-18 17:18 BST - poke

@@Alex instructed @@Systacean to continue assigned tasks.
`systacean-3` is now ready for @@Architect review:
[systacean/systacean-3.md](../systacean/systacean-3.md#2026-05-18-1718-bst---implementation-ready).

Patch summary:

* `crates/chan-server/src/static_assets.rs` marks SPA shell
  responses `Cache-Control: no-store`.
* Static asset responses stay immutable-cacheable.
* Both shell and asset responses now include `Vary: Host`.

Verification:

* `cargo fmt --check`
* `cargo test -p chan-server static_assets`
* `cargo clippy -p chan-server --all-targets -- -D warnings`

Known gap: needs @@WebtestB Lane B re-repro after landing.
Commit remains gated on @@Alex.

## 2026-05-18 17:46 BST - poke

`systacean-3` committed and pushed.

Commit:

* `f94c4b5` - Scope SPA shell and asset caching per
  chan-serve instance

Pre-push gate:

* `scripts/pre-push` - passed

Pushed `main` to `origin/main` (`13eadfb..f94c4b5`).

Ready for @@WebtestB Lane A + Lane B drift re-repro on the
new headers.

## 2026-05-18 18:10 BST - poke

`systacean-6` committed and pushed.

Commit:

* `83fbb20` - Scope SPA storage keys per serve instance

Patch summary:

* Scoped SPA bearer-token storage and per-tab session window ids
  by `window.location.origin` plus injected chan prefix.
* Confirmed cookies are not the auth/session routing mechanism in
  this tree; only tunnel-header scrubber code mentions them.
* Documented the scheme in `crates/chan-server/src/static_assets.rs`
  and the chan-server section of `crates/chan-drive/design.md`.

Verification:

* `cd web && npm run test -- src/api/client.test.ts`
* `cd web && npm run check`
* `cd web && npm run test`
* `scripts/pre-push`

Pushed `main` to `origin/main` (`7e09d20..83fbb20`).

Ready for @@WebtestA same-recipe re-repro.

## 2026-05-18 18:40 BST - poke

`systacean-7` committed and pushed.

Commit:

* `f975ee7` - Fix desktop DMG build signing env

Patch summary:

* `desktop/Makefile` now exports `APPLE_SIGNING_IDENTITY`
  and `APPLE_TEAM_ID` only when non-empty.
* This avoids passing an explicit empty codesign identity to
  Tauri's default build, which failed with `no identity found`
  before DMG bundling.
* Signed/notarized paths still use `sign-prereqs` /
  `notarize-prereqs` for clear failure messages.

Verification:

* `make -C desktop build` outside sandbox — passed
* produced `target/release/bundle/macos/Chan.app`
* produced `target/release/bundle/dmg/Chan_0.10.1_aarch64.dmg`
* `cargo tauri build --bundles app` — passed
* mounted the DMG and verified `Chan.app` plus
  `Applications -> /Applications`
* `scripts/pre-push` — passed in a clean temporary worktree
  with only the `systacean-7` Makefile patch applied

Main workspace note: direct `scripts/pre-push` is currently
blocked by unrelated dirty formatting in
`desktop/src-tauri/src/serve.rs`.

Pushed `main` to `origin/main` (`776aebd..f975ee7`).

## 2026-05-18 18:44 BST - poke

`systacean-8` committed and pushed.

Commit:

* `65534d3` - Replay terminal scrollback after reload

Diagnosis:

* chan-server already retained PTY output in the configured
  server-side ring (`server.terminal.ring_bytes`, default 1 MiB).
* The browser reload path restored `terminalSessionId` and the old
  terminal byte cursor (`tseq`), then reattached with
  `since=<old lastSeq>`.
* A fresh xterm buffer therefore asked for only bytes after the
  pre-reload end offset and skipped the retained scrollback.

Patch summary:

* Stop serializing `tseq` into per-window session layout.
* Ignore legacy `tseq` during restore/hydration.
* Keep in-memory `lastSeq` for same-page WebSocket reconnects, so
  live reconnects still avoid duplicating already-rendered output.

Verification:

* `cd web && npm run test -- src/state/tabs.test.ts src/terminal/session.test.ts`
* `cd web && npm run check`
* `cd web && npm run test`
* `scripts/pre-push` passed on rerun after a transient unrelated
  `Too many open files` failure in one chan-drive test

Pushed `main` to `origin/main` (`f975ee7..65534d3`).

Ready for @@WebtestB reload/scrollback walkthrough.

## 2026-05-18 19:10 BST - poke

Online, starting
[../systacean/systacean-9.md](../systacean/systacean-9.md).

## 2026-05-18 19:38 BST - poke

`systacean-9` landed; notes and verification in
[../systacean/systacean-9.md](../systacean/systacean-9.md#2026-05-18-1935-bst---ready-to-land).

## 2026-05-18 20:18 BST - poke

Investigated the `systacean-8` B19 FAIL. Candidate patch is
uncommitted and gate-green; diagnosis and verification in
[../systacean/systacean-8.md](../systacean/systacean-8.md#systacean-follow-up--2026-05-18-2018-bst).

## 2026-05-18 21:03 BST - poke

`systacean-8` B19 follow-up landed as `1cd4ef2`; ready for
@@WebtestB reload/scrollback re-run.

## 2026-05-18 20:46 BST - poke

`systacean-11` is implemented and gate-green. It adds
`POST /api/terminal/:session/event-reply`, validates the locked
`survey-reply` schema with `400` on invalid bodies, returns `409`
without an attached watcher, and writes JSON into the active watcher
dir via `tokio::fs` temp-file + rename.

Verification:

* `cargo test -p chan-server event_reply --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `scripts/pre-push`

## 2026-05-18 20:48 BST - poke

`systacean-11` landed as `530e30f` (`Add terminal event-reply writer`).
Ready for @@FullStack to use the endpoint and for Webtest coverage.

## 2026-05-18 20:56 BST - poke

`systacean-10` is implemented and gate prep is green so far. Decision:
`systacean-6` storage scoping is no-op for the documented drift after
`systacean-3`; I reverted `storageScopeKey` usage and the associated
docs/tests.

Local no-`systacean-6` smoke used `18810 -> 18801` because `8801/8810`
were already bound by existing `chan` processes. Both shells reported
`cache-control: no-store` + `vary: Host`, and the browser stayed on
`18801` after warm-cache navigation. Chrome MCP is not exposed in this
runtime, so this is paired with @@WebtestA's earlier Chrome MCP
`systacean-3` warm-cache PASS in `webtest-a-5`.

Verification:

* `npm run test -- src/api/client.test.ts`
* `npm run check`
* `npm run build`
* `cargo test -p chan-server static_cache_headers --no-default-features`
* `cargo fmt --check`

## 2026-05-18 21:02 BST - poke

`systacean-10` landed as `4ca7dc4` (`Revert SPA storage key scoping`).
FullStack's dirty `web/src/api/client.ts` endpoint addition was left
unstaged; only the `storageScopeKey` removal was committed from that
file.

## 2026-05-18 20:31 BST - poke

Online and idle; no newer @@Architect assignment found after
`systacean-10` / `systacean-11` handoff.

## 2026-05-18 20:37 BST - poke

Online and ready; no newer @@Architect assignment found.

## 2026-05-18 20:54 BST - poke

Online and ready; still no newer @@Architect assignment after the
completed `systacean-10` / `systacean-11` queue.

## 2026-05-18 21:05 BST - poke

Online and ready; no newer @@Architect assignment found.

## 2026-05-19 04:31 BST - poke

Online and ready; no newer @@Architect assignment found after the
completed `systacean-10` / `systacean-11` queue.

## 2026-05-19 04:33 BST - poke

Online and ready; no newer @@Architect assignment found.

## 2026-05-19 04:49 BST - poke

Online, starting
[../systacean/systacean-12.md](../systacean/systacean-12.md).

## 2026-05-19 04:58 BST - poke

`systacean-12` is implemented and gate-green. It adds the HTTP
terminal control channel (`POST /api/terminals`,
`POST /api/terminals/:session/restart`, `DELETE /api/terminals/:session`)
with stored command/env restart semantics and optional
`orchestrator_session` pre-flight event emission for FullStack.

Verification:

* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `scripts/pre-push`

## 2026-05-19 04:58 BST - poke

`systacean-12` landed as `314a68b` (`Add HTTP terminal control
channel`). Ready for `fullstack-20` to consume the API; visible tab
placement remains on the frontend side.

## 2026-05-19 05:03 BST - poke

Online, starting
[../systacean/systacean-13.md](../systacean/systacean-13.md).

## 2026-05-19 05:10 BST - poke

`systacean-13` is implemented and gate-green. It adds the PTY
`bytes_since_focus` signal, terminal WebSocket focus/activity frames,
and the tab-strip activity dot that clears on focus.

Verification:

* `cargo test -p chan-server activity --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `npm run test -- src/components/Pane.test.ts src/state/tabs.test.ts`
* `npm run check`
* `npm run build`
* `scripts/pre-push`

## 2026-05-19 05:12 BST - poke

`systacean-13` landed as `1694041` (`Add terminal tab activity
indicator`). Ready for the next queue item, `systacean-14`.

## 2026-05-19 05:32 BST - poke

Online, starting
[../systacean/systacean-14.md](../systacean/systacean-14.md).

## 2026-05-19 05:37 BST - poke

`systacean-14` is implemented and gate-green. It publishes chan's
current `__mcp-proxy` descriptor to Claude Code, Codex, and Gemini
CLI discovery surfaces at `chan serve` startup, preserving user-owned
entries and refreshing only chan-owned entries.

Verification:

* `cargo test -p chan-server mcp_discovery --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `scripts/pre-push`

## 2026-05-19 05:37 BST - poke

`systacean-14` landed as `96f4f40` (`Auto-publish chan MCP
discovery`). Wave-B Systacean queue is complete.

## 2026-05-19 05:55 BST - poke

Online and ready; no newer @@Architect assignment found after the
completed Wave-B Systacean queue.

## 2026-05-19 06:06 BST - poke

Online, starting
[../systacean/systacean-15.md](../systacean/systacean-15.md).

## 2026-05-19 06:11 BST - poke

`systacean-15` landed as `21d6fe5` (`Fix terminal activity focus
tracking`). Root cause was SPA pane focus semantics; WebtestA should
re-run item 7.

## 2026-05-19 06:37 BST - poke

Online, starting
[../systacean/systacean-16.md](../systacean/systacean-16.md).

## 2026-05-19 06:39 BST - poke

`systacean-16` landed as `538eeb8` (`Tune terminal activity byte
counting`). ANSI/control-only PTY writes no longer trip the activity
dot; real visible text still does. Ready for @@WebtestA re-test.

## 2026-05-19 06:56 BST - poke

Online and ready; no newer @@Architect assignment found after
`systacean-16`.

## 2026-05-19 06:56 BST - poke

Still online and ready; no newer @@Architect assignment found after
`systacean-16`.
