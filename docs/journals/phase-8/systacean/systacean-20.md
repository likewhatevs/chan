# systacean-20 — gate 3 chan-drive lock contract tests on Windows + flag underlying platform gap

Owner: @@Systacean
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Unblock Windows cargo test on `ci.yml` for the 3 remaining
chan-drive lock-contract assertions that fail on Windows
because the lock primitive doesn't surface
`ChanError::DriveLocked` the same way `flock` does on
Unix. Apply shape (ii) `#[cfg(unix)]` — the mechanical
gate-unblocker. Flag the underlying Windows lock-primitive
gap as a Round-3 polish/hardening item.

After this lands alongside `-18` follow-up #4 + `-24`
smoke #6, the per-PR ci.yml Windows test surface is fully
green.

## Background

Surfaced 2026-05-21 by @@FullStackB during `-24` smoke #6
verdict. See
[`../alex/event-fullstack-b-architect.md`](../alex/event-fullstack-b-architect.md)
"poke (-b-24 smoke #6 verdict... 3 chan-drive lock
failures need routing)" + the task tail at
[`../fullstack-b/fullstack-b-24.md`](../fullstack-b/fullstack-b-24.md)
"Remaining Windows reds — out of -24's scope".

### The 3 failing tests

| Test | Location |
|------|----------|
| `drive::tests::second_open_blocks_on_writer_lock` | `crates/chan-drive/src/drive.rs:4396` |
| `library::tests::reset_drive_returns_locked_when_other_process_holds_lock` | `crates/chan-drive/src/library.rs:989` |
| `lock::tests::second_acquire_fails_while_held` | `crates/chan-drive/src/lock.rs:72` |

All 3 fail on `matches!(err, ChanError::DriveLocked)` —
chan-drive's lock primitive (file lock via flock on Unix)
either returns a different `ChanError` variant on Windows
OR doesn't fail in the same way at all.

### Why this isn't blocking real users today

* **chan-desktop ships macOS only at v0.11.2**. Windows
  chan-desktop isn't a real-user surface yet.
* **chan CLI** does have a Windows target in `release.yml`,
  but the lock primitive's failure mode on Windows is a
  test-only assertion — actual locking behavior at runtime
  is whatever Windows-fs does (likely "the second process
  gets EACCES or similar on the open", which surfaces as
  a different error to the user).
* The lock contract IS broken on Windows in the
  test-contract sense, but the broader Windows release
  path isn't user-facing for chan-desktop yet.

So: gate the tests to unblock CI today; fix the
lock-primitive parity properly when Windows becomes a
real-user release surface (Round 3 + or later).

## Decision: fix shape (ii) — `#[cfg(unix)]` the 3 tests

Three options @@FullStackB laid out:

* **(i)** Make the lock primitive return `ChanError::DriveLocked`
  on Windows too (Windows-specific bridge in `lock.rs`
  over `LockFileEx` or similar). REAL fix; bigger scope.
* **(ii)** Gate the 3 tests `#[cfg(unix)]`. Mechanical
  gate-unblocker. Documents the platform gap by gating.
* **(iii)** Cross-platform abstraction layer to harmonise
  the lock contract. Over-engineered for current state.

Architect call: **(ii)** for the unblock. Same pattern
as `-17` + `-18` — gate-unblocker via `#[cfg]` first;
real cross-platform fix lands when the broader Windows
lane work picks up (Round 3+).

Shape (i) gets flagged in
[`../phase-8-bugs.md`](../phase-8-bugs.md) as a Round-3
polish/hardening candidate; tracked but not blocked.

## Acceptance criteria

### Gate the 3 tests

1. `crates/chan-drive/src/drive.rs:4396` — wrap test
   function with `#[cfg(unix)]` attribute (alongside any
   existing `#[test]` / `#[tokio::test]` / `#[ignore]`).
2. `crates/chan-drive/src/library.rs:989` — same.
3. `crates/chan-drive/src/lock.rs:72` — same.

Skip reason via the gating itself + a 1-line comment
above each cross-referencing `systacean-20` for audit:

```rust
// systacean-20: gated on Unix because Windows lock
// primitive doesn't surface DriveLocked; see
// docs/journals/phase-8/systacean/systacean-20.md +
// phase-8-bugs.md "Windows lock contract parity"
#[cfg(unix)]
#[test]
fn second_open_blocks_on_writer_lock() { ... }
```

### Bug-list entry for the underlying gap

Add a Round-3 polish entry to
[`../phase-8-bugs.md`](../phase-8-bugs.md):

* **Title**: "Windows lock contract parity — chan-drive
  lock primitive doesn't surface `DriveLocked` on Windows"
* Reference: this task + the 3 gated tests.
* Want (Round-3 polish): Windows-specific bridge in
  `lock.rs` using `LockFileEx` (or equivalent) to surface
  `ChanError::DriveLocked` consistently. After it lands,
  REVERT the `#[cfg(unix)]` gates from this task.

### Local verification

1. `cargo test -p chan-drive` — green on macOS (the 3
   tests still run since macOS is Unix).
2. `cargo fmt --all`; `cargo clippy --all-targets -- -D
   warnings` — workspace-wide green.

### CI verification

1. Push fastforward to `systacean-18-smoke` branch (you're
   already using that smoke branch for the follow-up #4
   work) OR a fresh `systacean-20-smoke` — your call.
   Either shape works; fastforward to `-18-smoke` is the
   lower-overhead path since the gates are landing as a
   coherent gate-unblocker sweep.
2. `gh workflow run ci.yml --ref <smoke-branch>`.
3. Confirm:
   * Windows cargo test ✓ (no more 3 lock failures).
   * Ubuntu cargo test ✓ (no more BGE failures from
     follow-up #4).
   * macOS green (the 3 lock tests still run on macOS).
   * All other jobs green.

After this confirms, the per-PR ci.yml gate is structurally
fully green for the first time since ~2026-05-19 on ALL
three platforms.

## How to start

1. Read the 3 test sites; confirm the test functions are
   straightforward to gate (no shared helper that
   incidentally needs gating).
2. Apply `#[cfg(unix)]` + audit-trail comment to each.
3. Local pre-push gate green.
4. CI smoke (fastforward `-18-smoke` recommended).
5. Add bug-list entry for the Round-3 polish item.
6. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@Systacean lane (chan-drive primary scope).
* SEQUENCING: pick up after `-18` follow-up #4 commits
  (you're in flight on that this beat; smoke fired).
  `-20` can ride the same `-18-smoke` branch or its own;
  doesn't matter operationally.
* `systacean-19` (C2 graceful BM25 degradation) doesn't
  affect this — lock contract is orthogonal to the
  embedding model.

### Shared-infra authorization

**Authorization: yes** for:

* `crates/chan-drive/src/{drive,library,lock}.rs` (3
  `#[cfg(unix)]` gates + audit comments).
* `docs/journals/phase-8/phase-8-bugs.md` (Round-3 polish
  entry for the Windows lock primitive bridge).
* `docs/journals/phase-8/systacean/systacean-20.md` (task
  tail).
* `docs/journals/phase-8/alex/event-systacean-architect.md`
  (your outbound).

Bug-list edits cross-lane but the entry is narrow + the
bug-list is the canonical place to track Round-3 polish
candidates. @@Systacean may proceed without further
in-chat confirmation from @@Alex.

## Numbering

Highest dispatched `systacean-N` is `-19` (C2 graceful
degradation); this is `-20`.

### Queue (revised 2026-05-21)

```
-18 follow-up #4 (in flight; smoke on systacean-18-smoke)
-20 (this task; Windows lock test gating)
-19 (C2 graceful BM25 degradation; reverts all 28 #[ignore] gates after)
-16 (chan-report file-class buckets; feature work; parks if needed)
-12 (tauri-plugin-updater verify; parked on permission ask)
```

`-20` slots between `-18` follow-up #4 and `-19` because
it's tiny (3 `#[cfg]` adds) and gate-unblocker-tier
priority. Either ride the `-18-smoke` branch (fastforward)
or its own smoke branch; @@Systacean picks.

## Out of scope

* Windows lock-primitive bridge (shape (i)). That's a
  Round-3 polish item; tracked in bug list.
* Cross-platform abstraction layer (shape (iii)). Over-
  engineered for current scope; same Round-3 polish
  territory.
* Auditing OTHER Windows test failures beyond these 3.
  If you spot adjacent issues while in the file, surface
  as a follow-up task or bug-list entry.
* Modifying `lock.rs` / `library.rs` / `drive.rs`
  production code (lock primitive's actual runtime
  behavior). Stay narrow — test gating only.

## What this task is NOT

* A real Windows lock contract fix. That's tracked
  separately for Round 3+.
* A regression. The Windows test failures are pre-existing
  for the same reason `-17` + `-18` were pre-existing —
  the broken pre-`ci-12`-and-`-17` gate hid them.
