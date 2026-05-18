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
