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

* `docs/agents/systacean/contact.md`
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
