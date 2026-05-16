# architect-syseng-4: Self-upgrade audit (crates/chan/src/update.rs)

From: syseng. To: architect. Status: DONE. Severity: 1 finding
that affects Windows users in the first canonical release, plus 3
non-blocking observations.

CLAUDE.md flags `crates/chan/src/update.rs` as the self-upgrade
home. The phase didn't cover it directly; syseng ran an audit
after the bulk phase commit landed. The module is otherwise
well-engineered.

## What `update.rs` already does well

- HTTPS-only metadata fetch (`fetch_text` rejects non-https).
  Archive URL is constructed from a hardcoded `DL_BASE =
  "https://chan.app/dl"` constant.
- SHA-256 verification of the downloaded archive against the
  per-release `SHA256SUMS` file. Hash computed on-the-fly while
  streaming the download — no whole-archive buffer in RAM.
- Size caps: 256 MiB on the archive, 1 MiB on metadata. Catches
  a misconfigured mirror or an unbounded response.
- Connect + total timeouts on the HTTP client.
- Semver compare prevents silent downgrade; downgrade via
  `--version` prompts with default-`N`; non-interactive mode
  requires `-y`.
- Pre-flight writability probe on the binary directory before
  the long download.
- Temp file naming includes PID (collision-safe across parallel
  invocations).
- RAII `TempGuard` cleans up temp files on every exit path.
- `set_executable_mode` chmod 0o755 on unix; no-op on Windows
  (correct).
- State file persisted via `chan_drive::fs_ops::atomic_write` —
  same atomic-write discipline as the rest of the app.
- 8 unit tests cover the pure functions: semver compare,
  SHA256SUMS parsing, archive URL shape, current_target
  enumeration, probe cadence.

## Findings

### 1. Windows self-upgrade is likely broken (release-relevant)

`crates/chan/src/update.rs:513`:

```rust
fs::rename(&bin_temp, &exe_path).with_context(|| ...)?;
```

On unix this is fine: POSIX `rename(2)` can replace a running
executable because the kernel keeps the open inode alive after
the directory entry flips. On Windows, replacing the running
executable's directory entry fails with
`ERROR_SHARING_VIOLATION` / `ERROR_ACCESS_DENIED`. `current_target()`
enumerates both `windows-x86_64` and `windows-aarch64` as
supported targets, so this lands in users' hands.

Recommended fix: the standard Windows self-update pattern is

```
1. fs::rename(&exe_path, &exe_path.with_extension("old"))   // park the running binary
2. fs::rename(&bin_temp, &exe_path)                          // install the new binary
3. mark `.old` for deletion-on-reboot (MoveFileEx with
   MOVEFILE_DELAY_UNTIL_REBOOT), or leave it for the next run
   to GC.
```

A `#[cfg(windows)]` arm around the `fs::rename` site, plus a
small follow-up GC of leftover `.old` files at boot, covers it.
Add an integration test under a `#[cfg(windows)]` gate that
exercises a fake-archive install into a tempdir.

### 2. Defense-in-depth on archive download URL (cheap)

`crates/chan/src/update.rs:461-467`: the archive `client.get(...)`
does NOT run through `fetch_text`, so the `if !url.starts_with(
"https://") { bail }` guard does not apply to the archive
fetch. Today this is safe because `archive_url()` is built from
the hardcoded `https://chan.app/dl` constant. But if a future
refactor adds a configurable base URL (mirror support,
enterprise-internal releases, dev-only `http://localhost`
testing) and forgets to re-add the guard, the archive download
would silently accept `http://`.

Recommend lifting the HTTPS check to a small helper called by
both fetch paths, OR adding the same explicit guard immediately
before the archive `client.get(...)`. One-line fix.

### 3. No rollback if the new binary fails to launch (non-blocking)

The current sequence is "extract → chmod → atomic rename in".
If the new binary fails to launch (corrupt extract, missing
shared lib, signature failure on macOS Gatekeeper, etc.), there
is no automatic rollback: the old binary is gone, and the user
has to manually re-download a working version.

The Windows fix (#1) naturally provides a `.old` sibling that
could be promoted back. On unix, a `chan.bak` parallel rename
+ rename-back-on-startup-smoke-failure pattern would mirror it.
Lower priority than #1 because unix install failures are rare;
worth noting as a future-improvement.

### 4. No signed checksums (known tradeoff, document as such)

Trust today = chan.app's TLS cert + the integrity of whatever
publishes to that bucket. SHA256SUMS is fetched over HTTPS only,
not signed. If the release bucket is ever compromised, an
attacker can swap the binary + the matching SHA256SUMS and
TLS won't detect it.

For first canonical release, acceptable. For a binary that runs
arbitrary user content and can self-upgrade, signed checksums
(minisign, cosign, or sigstore) would be the next defense layer.
File as a "v1.x release-engineering follow-up", not phase-1
scope.

## Verification

```
cargo test -p chan update::                # 8 passed (pure fns)
cargo build -p chan                        # ok
cargo clippy --all-targets -- -D warnings  # clean
```

## Recommended dispatch

- Finding #1 (Windows rename): rustacean-N task, ~1 hour. Block
  Windows public-release sign-off until landed.
- Finding #2 (defense-in-depth HTTPS guard): rustacean-N or fold
  into #1's commit. ~5 minutes.
- Finding #3 (rollback on launch failure): defer to follow-up
  phase; pairs naturally with finding #1.
- Finding #4 (signed checksums): release-engineering follow-up,
  not Phase 1.

syseng audit complete.
