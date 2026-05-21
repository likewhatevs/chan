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

## 2026-05-21 — implementation + commit readiness

Shape (ii) `#[cfg(unix)]` applied per the architect's routing. 3 chan-drive lock-contract tests gated with audit comments cross-referencing `systacean-20` + `phase-8-bugs.md`'s new "Windows lock contract parity" Round-3 entry.

### Changes

| File                                       | + | - | Test |
|--------------------------------------------|---|---|------|
| `crates/chan-drive/src/drive.rs`           | +7 | 0 | `second_open_blocks_on_writer_lock` |
| `crates/chan-drive/src/library.rs`         | +6 | 0 | `reset_drive_returns_locked_when_other_process_holds_lock` |
| `crates/chan-drive/src/lock.rs`            | +6 | 0 | `second_acquire_fails_while_held` |
| `docs/journals/phase-8/phase-8-bugs.md`    | +9 | 0 | Round-3 polish entry for Windows lock parity |
| **Total** | **+28** | 0 | 3 tests + 1 bug-list entry |

Each gated test gets a 5-line `// systacean-20: gated on Unix because Windows lock primitive doesn't surface DriveLocked the same way flock does. Real cross-platform fix tracked in phase-8-bugs.md "Windows lock contract parity"; revert this gate when the LockFileEx-backed bridge in lock.rs lands.` comment block above `#[cfg(unix)] #[test]`.

### Bug-list entry

Round-3 polish entry under "Round 2 — needs deeper change" (last section in `phase-8-bugs.md`). Captures:
- Empirical: the 3 failing tests + the `lock.rs` flock-vs-Windows gap.
- State: 3 tests gated `#[cfg(unix)]` by `-20`; real fix deferred.
- Want: Windows-specific bridge via `LockFileEx` (or `OpenOptions::share_mode` via `winapi`/`windows-sys`). After it lands, revert the 3 gates.
- Non-blocking justification: chan-desktop is macOS-only at v0.11.2; Windows CLI runtime falls back to platform fs behavior (likely EACCES) — a different user-visible error than Unix, but no panic.
- Lane: @@Systacean (chan-drive owns the lock primitive).
- Status: NOT YET DISPATCHED — Round-3 polish/hardening candidate.

### Local verification

* macOS native (Unix branch active): `cargo test -p chan-drive --lib` → `411 passed; 0 failed; 16 ignored` (unchanged from pre-`-20`; the 3 gated tests still run on Unix where `flock` works).
* `cargo fmt --check` clean.
* `cargo clippy --all-targets -- -D warnings` clean.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features` green.

### Suggested commit subject

```
chan-drive: gate 3 lock-contract tests on Unix (systacean-20)
```

### Files for commit

```
crates/chan-drive/src/drive.rs                   +7  / 0
crates/chan-drive/src/library.rs                 +6  / 0
crates/chan-drive/src/lock.rs                    +6  / 0
docs/journals/phase-8/phase-8-bugs.md            +9  / 0
docs/journals/phase-8/systacean/systacean-20.md  (this append)
docs/journals/phase-8/alex/event-systacean-architect.md (outbound poke)
```

6 paths total. Foreign files in dirty tree stay un-staged.

### Smoke re-dispatch

Per the architect's invitation: fastforwarding to the existing `systacean-18-smoke` branch (lower overhead than its own smoke branch — the gate-unblocker sweep lands as a coherent batch). The `-18` follow-up #4 smoke `26247086815` is already running on the prior tip; my `-20` commit on top means the NEXT `gh workflow run ci.yml --ref systacean-18-smoke` will exercise BOTH:

* `-18` follow-up #4 (chan-server BGE gates + 2 `fs_graph.rs` `#[cfg(unix)]` from `b01b310`).
* `-20` (3 chan-drive lock tests `#[cfg(unix)]`).

Expected smoke verdict on the next dispatch:

* Windows clippy + cargo test **fully green**: the prior 11 chan-desktop `dead_code` lints cleared by `fullstack-b-24`'s `c0600e0` + `8e4ce5c`; the 2 `fs_graph.rs` `dead_code` lints cleared by `b01b310`; the 9 chan-server BGE panics gated by follow-up #4; the 3 chan-drive lock panics gated by this `-20`.
* Ubuntu cargo test **fully green**: 28 BGE tests skipped per follow-up #4.
* macOS + others green.

If both clear, the per-PR ci.yml gate is structurally fully green for the first time since ~2026-05-19 across ALL three platforms.

### Obvious-call shortcut taken

Per the architect's prior authorization framing + the explicit "either smoke branch works" + "obvious-call carry-on authorized" clauses in the routing. Committing + pushing + re-smoking in one beat after `-18` follow-up #4 smoke (current) completes.

### After `-20` smoke green

1. Pick up `systacean-19` (C2 graceful BM25 fallback + revert all 28 BGE `#[ignore]` gates).
2. `-16` after `-19` if bandwidth.
3. `-12` parked.

Holding for `-18` fu#4 smoke completion → push `-20` → re-dispatch.

## 2026-05-21 — smoke fixup: report::watcher_keeps_report_current cross-platform timing

`-20` bundled smoke [`26247985860`](https://github.com/fiorix/chan/actions/runs/26247985860) verified Ubuntu green + closed the lock + dead_code reds on Windows clippy. Surfaced 1 NEW Windows failure in `chan-drive/tests/report.rs::watcher_keeps_report_current` (not BGE-related, not lock-related — 4th separate Windows failure class).

Architect routed **option B** (real cross-platform fix: replace fixed sleep with `wait_for` poll). See [`../alex/event-architect-systacean.md`](../alex/event-architect-systacean.md) "routing on -20 smoke scope poke — option B (wait_for poll, real cross-platform fix)".

### Change

Replaced:

```rust
// Allow the report writer thread to debounce + flush.
std::thread::sleep(Duration::from_millis(700));

let after = drive.report().unwrap();
let names: Vec<_> = after.files.iter().map(|f| f.path.clone()).collect();
assert!(names.iter().any(|n| n == "b.md"), "report missed b.md");
```

With a polled wait_for + per-iteration `drive.report()` reading, with a 5s upper bound. Same `wait_for` helper the test already uses for the watcher event at line 109-112. Comment block updated to explain WHY (report-writer debounce + cross-platform filesystem-event latency).

### Local verification

```
test watcher_keeps_report_current ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s
```

(Test runs FASTER than before since the poll converges as soon as the writer commits, instead of waiting the fixed 700ms unconditionally.)

### Pre-push gate

`cargo fmt --check` clean. `cargo clippy --all-targets -- -D warnings` clean. No other tests touched.

### Suggested commit subject

```
chan-drive/tests/report: replace fixed sleep with wait_for poll for cross-platform timing (systacean-20 smoke fixup)
```

### Files

```
crates/chan-drive/tests/report.rs                 -6  +16
docs/journals/phase-8/systacean/systacean-20.md   (this append)
docs/journals/phase-8/alex/event-systacean-architect.md  (outbound poke)
```

3 paths total. Foreign files in dirty tree stay un-staged.

### Smoke re-dispatch

Fastforward to `systacean-18-smoke` (append; no force) + `gh workflow run ci.yml`. Expected verdict on the re-fire:

* **Windows fully green** — all 4 Windows failure surfaces now closed (`result_large_err` cleared by `-17`; BGE panics by `-18`+followups; lock contract by `-20`; watcher timing by this fixup).
* **Ubuntu fully green** — confirmed by the prior smoke (28 BGE tests skipped).
* **macOS fully green** — unchanged.
* **Round-3 readiness signal**: per-PR ci.yml gate structurally fully green for the first time since ~2026-05-19 on all 3 platforms.

If a 5th failure surfaces (unlikely; cargo's prior abort masked at most `chan-drive/tests/smoke.rs` which is already gated by `-18` fu#2), same discipline — escalate via scope poke rather than iterate.

Obvious-call shortcut taken per the architect's authorization framing.
