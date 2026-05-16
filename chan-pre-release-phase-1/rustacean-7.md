# rustacean-7: Harden self-upgrade install

Owner: rustacean. Surfaced by `architect-syseng-4.md`.

Status: REVIEW.

## Findings closed

`crates/chan/src/update.rs` had two release-relevant gaps:

- Windows self-upgrade tried to replace the running executable directly.
  POSIX supports that rename pattern, but Windows can reject it because the
  active `.exe` cannot be replaced in place.
- Archive downloads were built from the hardcoded HTTPS `DL_BASE`, but they
  did not call the same explicit HTTPS guard as metadata fetches.

## Fix

- Added `ensure_https_url` and call it from both metadata fetches and the
  streamed archive download path.
- Split final binary installation into `install_replacement`.
- Non-Windows keeps the existing atomic `rename(new_bin, exe_path)` behavior.
- Windows now parks the running executable at a sibling
  `.chan.upgrade-old.<exe>.<pid>` path, renames the new binary into place, and
  attempts to restore the old binary if the second rename fails.
- Added best-effort cleanup of stale `.chan.upgrade-old.*` files in the
  binary directory before starting a new upgrade attempt.

The Windows branch avoids a new dependency and follows the existing module
style: standard library filesystem operations, explicit error context, and
best-effort cleanup for leftovers that may still be locked by the OS.

## Tests

Added in `crates/chan/src/update.rs::tests`:

- `test_ensure_https_url_rejects_plain_http`
- `test_upgrade_backup_path_shape`
- `test_cleanup_upgrade_backups_only_removes_matching_files`
- `test_install_replacement_renames_on_non_windows`
- `test_install_replacement_parks_old_binary_on_windows` (`#[cfg(windows)]`)

## Verification

```
cargo fmt --all -- --check                # clean
cargo test -p chan update::               # 12 passed
cargo test -p chan                        # 50 passed
cargo clippy --all-targets -- -D warnings # clean
cargo build --no-default-features         # ok
```

## Residual

No release blocker remains on this surface from `architect-syseng-4.md`.
Rollback after launching a bad replacement and signed checksums remain valid
post-release hardening items, but were explicitly non-blocking in the audit.
