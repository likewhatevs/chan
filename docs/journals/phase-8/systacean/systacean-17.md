# systacean-17 — chan-drive ConfigError boxing (Windows clippy result_large_err)

Owner: @@Systacean
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Unblock `ci.yml::test (windows-latest)` clippy by reducing
`chan-drive::index::config::ConfigError`'s size. Currently
the error variant carries an unboxed `toml::de::Error`, which
on the Windows target trips clippy's `result_large_err`
lint at every `Result<_, ConfigError>` return site.

Pre-existing: the lint has been red on Windows for the last
~15 commits' worth of unverified main runs; it was hidden
behind the GTK gap (ci-12) until @@CI's smoke validation
surfaced it.

## Background

Surfaced 2026-05-21 by @@CI during `ci-12` smoke validation.
See [`../alex/event-ci-architect.md`](../alex/event-ci-architect.md)
"Out-of-lane finding: Windows result_large_err" + ci-12's
post-mortem note for the empirical trace.

Failure mode (from `ci-12-smoke` workflow_dispatch):

```
error: the `Err`-variant returned from this function is very large
  --> crates/chan-drive/src/index/config.rs:130:34
   |
130 | pub fn load(index_dir: &Path) -> Result<IndexConfig, ConfigError> {
help: try reducing the size of `index::config::ConfigError`, for example
      by boxing large elements or replacing it with `Box<index::config::ConfigError>`
```

Same lint trips at:

* `crates/chan-drive/src/index/config.rs:140` (`save`).
* `crates/chan-drive/src/index/facade.rs:177` (`open`).
* (Run log likely lists more; audit at the start.)

Root cause: `ConfigError` carries `toml::de::Error` (or a
variant containing it). On the Windows target the type's
stack layout exceeds clippy's `result-large-err` default
threshold (currently 128 bytes per clippy's default; chan
doesn't override). Linux doesn't flag it because the type
size is below the threshold there (different stack
alignment / type size).

## Decision: fix shape

Box the large variant(s). Two reasonable shapes:

* **(a)** `Box<toml::de::Error>` inside the relevant
  `ConfigError` variant(s). Smallest diff; preserves the
  current error API exactly. Recommended.
* **(b)** `Box<ConfigError>` at every `Result` site.
  Heavier; touches every call site; only worth it if (a)
  doesn't bring the type under the threshold (unlikely).

Default to (a); only escalate to (b) if (a) leaves the
type over the threshold on Windows.

## Acceptance criteria

### Audit the offending sites

1. `cargo clippy -p chan-drive --all-targets -- -D warnings`
   on macOS — likely passes (Linux/macOS don't flag it).
   Doesn't reproduce the Windows trip locally; flagged for
   awareness.
2. Optional empirical confirmation via `limactl shell
   default sudo sdme chan-build-ubuntu -r ubuntu` + cross-
   compile to x86_64-pc-windows-gnu, OR rely on CI
   verification after the YAML patch lands. The Windows
   shape is hard to repro locally (no Windows host); CI is
   the canonical gate. Recommend skipping local repro
   attempt + relying on `ci-12-smoke`-style smoke dispatch
   for confirmation.
3. Read `crates/chan-drive/src/index/config.rs` to identify
   every `ConfigError` variant carrying a large type.
   `toml::de::Error` is the named offender; audit for
   others.

### Apply boxing

1. Mutate the `ConfigError` enum to box the large variant(s)
   (likely the `Toml` variant or similar). Adjust
   `From<toml::de::Error>` impls + match arms accordingly.
2. Verify all `Result<_, ConfigError>` consumers compile
   without API change (the public surface should stay
   identical; only the internal layout shrinks).
3. `cargo fmt --all`; `cargo clippy --all-targets -- -D
   warnings`; `cargo test -p chan-drive`. All green locally.

### CI verification

1. Push the patch to a branch (similar to `ci-12-smoke`
   shape OR fold into the regular PR / branch flow).
2. `gh workflow run ci.yml --ref <branch>`. Confirm:
   * `test (windows-latest)` reaches the clippy step
     and either passes OR reds on something OTHER than
     `result_large_err` (any remaining reds belong to a
     separate task).
   * No regression on `test (ubuntu-latest)` or
     `test (macos-latest)`.

If `result_large_err` STILL trips on Windows after (a),
escalate to (b) (`Box<ConfigError>` at call sites).
Document the escalation in the task tail with the
re-confirmed empirical Windows error.

## How to start

1. Read `crates/chan-drive/src/index/config.rs` end-to-end;
   note every `ConfigError` variant + its payload size.
2. Pick the boxing shape (a) per the recommendation.
3. Apply the patch.
4. Local pre-push gate green (fmt + clippy + cargo test).
5. CI smoke via `gh workflow run ci.yml` on the patch
   branch.
6. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@Systacean lane (chan-drive primary scope).
* No interaction with @@CI's ci-12 fix needed; this
  layers on top once ci-12 lands.
* Pre-push gate green workspace-wide before commit
  clearance.

### Shared-infra authorization

This task touches `crates/chan-drive/src/index/config.rs`
+ possibly `crates/chan-drive/src/index/facade.rs` (Rust
source, not shared infra). No `.github/workflows/` edits
expected. If the fix needs a clippy-config override (e.g.
raising `result-large-err-threshold`), prefer the boxing
fix over the threshold bump — the lint is correct + the
boxing is a real improvement (smaller stack copies of the
ok variant).

## Numbering

Highest committed `systacean-N` is `-14`; `-15` is cut +
in-flight (cleared but uncommitted as of this cut); `-16`
is queued (file-classification buckets); this is `-17`.

### Queue order (revised 2026-05-21)

`-15` (committable now) → `-17` (this task; gate-unblocker)
→ `-18` (model-dep tests; gate-unblocker) → `-16` (file-class
buckets; feature work). `-17` + `-18` ahead of `-16` because
they unblock the per-PR CI gate, which is load-bearing for
every future commit.

## Out of scope

* Raising the clippy `result-large-err-threshold` config.
  The lint is correct as-is; box the variant instead.
* Refactoring `ConfigError` to a separate error crate or
  `thiserror`-style derive. Stay narrow.
* Auditing the OTHER lints that may trip on Windows runs
  beyond `result_large_err`. If you spot adjacent issues
  while in the file, surface them as bug-list entries or
  follow-up tasks; don't fold into this commit.

## 2026-05-21 — implementation + commit readiness

Implemented shape (a) per the architect's recommendation: box
`toml::de::Error` in `ConfigError::Decode` + box
`toml::ser::Error` in `ConfigError::Encode`. Single-file
diff in `crates/chan-drive/src/index/config.rs`: +26 / -3.

### Changes

* `ConfigError::Decode { path, source: toml::de::Error }` →
  `Decode { path, source: Box<toml::de::Error> }`.
* `ConfigError::Encode(#[from] toml::ser::Error)` →
  `Encode(Box<toml::ser::Error>)`. Dropped `#[from]` because
  it would generate `From<Box<toml::ser::Error>>` and break
  `?` at `toml::to_string_pretty(cfg)?`. Added a manual
  `impl From<toml::ser::Error> for ConfigError` that wraps in
  `Box::new(e)`, so call-site `?` continues to compile
  unchanged.
* `config::load` construction site at line 136: wrapped
  `source` in `Box::new(source)`.

### Audit

`Result<_, ConfigError>` return sites benefit transparently
because the size shrink is at the type definition. Every
`Result<IndexConfig, ConfigError>` return at
`config::load`, `config::save`, and every `facade.rs`
caller (lines 179, 181, 191, 210, 270, 294, 637) sees the
smaller Err variant for free. No call-site edits needed.

`IndexError::Config(#[from] ConfigError)` at
`facade.rs:34` propagates the shrink up — `Result<_,
IndexError>` sites benefit too.

Adjacent `ChanError` impls in `crates/chan-drive/src/error.rs:77-90`
already string-render the toml errors at the From boundary
(`e.to_string()`), so `ChanError` carries strings, not the
underlying toml types. No boxing needed there.

### Why box both (Decode + Encode), not just Decode

The architect's task body named `toml::de::Error` as the
empirical offender from `ci-12-smoke`. `toml::ser::Error`
wasn't in the trace, but it's the same crate's error type
with the same intrinsic size class. If only `Decode` is
boxed, the variant ordering puts `Encode` in the
discriminant slot for the worst-case sizeof(enum) on
Windows — defensive to box both at the same time so the
shrink holds against future toml-crate version bumps that
might change the ser-side payload.

### `Box<T>` + thiserror

* `#[error(transparent)]` on `Encode(Box<toml::ser::Error>)`
  works because std provides `impl<T: Display> Display
  for Box<T>` and `impl<T: Error> Error for Box<T>`. Box
  itself is the field; thiserror's transparent forwards
  through it.
* `#[source]` on `Decode { source: Box<toml::de::Error> }`
  works for the same reason — `.source()` returns
  `&dyn Error` and `Box<T: Error>` deref-coerces.

### Local pre-push gate

All green at HEAD `f4a197d` (post-systacean-15):

* `cargo fmt --check` — clean.
* `cargo clippy --all-targets -- -D warnings` (macOS native)
  — clean. Linux + macOS already passed pre-fix; this
  confirms the boxing didn't introduce new lints.
* `cargo test -p chan-drive` — 425 + 4 + 8 + 1 + 2 + 8 + 3
  + 4 + 1 = all chan-drive tests green (including
  `malformed_is_error` which pins
  `matches!(err, ConfigError::Decode { .. })` against the
  boxed-source variant; pattern still matches).
* `cargo test` (workspace) — every crate's tests green,
  no drift in any consumer.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`
  — green.

### Windows verification — pending

Attempted `cargo clippy -p chan-drive --target x86_64-pc-windows-msvc
--all-targets -- -D warnings` on the macOS host. The
target is installed via rustup, but the `onig_sys` C dep
(oniguruma) fails to build because Windows MSVC C
headers aren't available on macOS (`fatal error:
'stdlib.h' file not found`). This is the same hard
local-host limitation the task body anticipates
("Recommend skipping local repro attempt + relying on
`ci-12-smoke`-style smoke dispatch for confirmation").

The structural change is correct (smaller variant +
smaller stack copy of every `Result<_, ConfigError>`
return value); Windows clippy should clear
`result_large_err` after this lands. Empirical Windows
confirmation = the next per-PR CI gate against main,
OR an architect-/Alex-initiated smoke dispatch against
a branch carrying this commit + the existing local
queue (the patch + `f4a197d` + the architect's earlier
`6b8bf38` / `f4c4dca` / etc. + @@CI's `6abac58`).

### Files

| File                                       | +     | -    |
|--------------------------------------------|-------|------|
| crates/chan-drive/src/index/config.rs      | +26   | -3   |

Plus this task tail append. Single-purpose commit shape.

### Suggested commit subject

```
chan-drive: box toml::Error variants in ConfigError (systacean-17)
```

Per the project's commit-subject pattern (matches `-6`'s
"Gate BGE-small model behind embed-model cargo feature +
runtime resolver (systacean-6)" + `-2`'s "Graph: ..."
shape).

### Shared-worktree discipline

Working tree dirty with foreign in-flight files (.github
workflows from @@CI, multiple webtest task tails, the
architect's event channels). Pre-stage explicit `git add`
of exactly:

```
crates/chan-drive/src/index/config.rs
docs/journals/phase-8/systacean/systacean-17.md
docs/journals/phase-8/alex/event-systacean-architect.md
```

Pre-commit `git diff --staged --stat` audit + post-commit
`git show --stat HEAD` audit will confirm exactly 3 paths.

### Coordination

* `-18` (model-dep tests gate-unblocker) lands next; the
  per-PR CI gate goes fully green only after both `-17` +
  `-18` are in HEAD.
* No conflicts with @@CI's `ci-12` ground work (`6abac58`)
  or `-15` (`f4a197d`); single-file edit on a separate
  source path.
* The Windows smoke verification is architect / @@Alex's
  call to schedule — I'll flag if I see anything change
  on subsequent CI runs.

Holding for @@Architect commit clearance.

## 2026-05-21 — committed as 663ab26 + smoke verification: PASS

Cleared by @@Architect on [`../alex/event-architect-systacean.md`](../alex/event-architect-systacean.md) "## 2026-05-21 — @@Architect: approved + commit clearance (systacean-17) + smoke-dispatch pick". Suggested subject accepted verbatim. Smoke-dispatch via `systacean-17-smoke` branch authorized.

### Sequence executed

1. Staged exactly 3 cleared paths (`crates/chan-drive/src/index/config.rs` + `systacean-17.md` + `event-systacean-architect.md`). Pre-stage audit clean (no foreign files from the 19-file dirty working tree).
2. Committed as `663ab26`: `chan-drive: box toml::Error variants in ConfigError (systacean-17)`. Post-commit audit clean.
3. `git push origin HEAD:refs/heads/systacean-17-smoke` — new branch on origin.
4. `gh workflow run ci.yml --ref systacean-17-smoke` — run [`26235956637`](https://github.com/fiorix/chan/actions/runs/26235956637) fired.

### Smoke result (10m wall-clock total)

| Job                                  | Outcome | Notes                                                              |
|--------------------------------------|---------|--------------------------------------------------------------------|
| rustfmt                              | ✓ 22s   | Clean.                                                             |
| web (check + test + build)           | ✓ 2m19s | Clean.                                                             |
| build (no default features)          | ✓ 7m37s | Clean. `RUSTFLAGS="-D warnings"` path stays green.                  |
| clippy + test (ubuntu-latest)        | X 9m55s | **Clippy step PASSED**; `cargo test` step failed on 14 BGE-model tests (exactly the `-18` failure list — pre-existing, not regression). |
| clippy + test (windows-latest)       | X 9m24s | Clippy reds on `dead_code` lints in chan-desktop IPC; **NO `result_large_err`** — `-17` fix delivered. |

### `-17` verification: PASS

Per the task body's acceptance criterion: "`test (windows-latest)` reaches the clippy step and either passes OR reds on something OTHER than `result_large_err`". The Windows clippy step DID reach + DID red on something other than `result_large_err`:

```
error: function `path_to_posix` is never used
error: function `abs_to_drive_rel` is never used
error: function `parent_rel` is never used
error: function `open_path` is never used
error: function `handle_request` is never used
error: struct `WindowCommandFrame` is never constructed
error: function `is_false` is never used
error: enum `WindowCommand` is never used
error: enum `ControlResponse` is never used
error: enum `ControlRequest` is never used
error: unused variable: `exit_signal`
note: `-D dead-code` implied by `-D warnings`
```

All 11 lints are `dead_code` (one `unused_variable`). All from chan-desktop's IPC layer (`desktop/src-tauri/src/`). All Windows-platform-only because the macOS / Linux `#[cfg(target_os = "...")]` paths declare these items at module scope where they're only consumed on those targets; the Windows target compiles the declarations but doesn't reference them in the inactive `#[cfg]` branches. That's a chan-desktop platform-conditional cleanup, NOT a chan-drive lint problem and NOT under `-17`'s scope.

`result_large_err` does not appear in the Windows clippy output. The boxing fix neutralized the lint as predicted. The variant size shrunk from "`toml::de::Error` payload + `PathBuf`" (Windows: over 128 bytes) to "pointer + `PathBuf`" (Windows: 16+24=40 bytes, well under threshold). Every `Result<_, ConfigError>` site benefits transparently.

### Ubuntu test failures: pre-existing `-18` scope

`cargo test --all-targets` on ubuntu-latest panicked on 14 tests:

drive.rs (12):
1. `link_targets_finds_file_after_index`
2. `index_file_stamps_pre_read_stat_so_concurrent_writes_stay_visible`
3. `pending_writes_journal_handles_forget_op`
4. `pending_writes_journal_is_empty_on_a_clean_path`
5. `pending_writes_journal_replay_converges_after_simulated_crash`
6. `pending_writes_replay_degrades_index_op_to_forget_when_file_is_gone`
7. `reconcile_catches_same_mtime_different_size_rewrite`
8. `reconcile_on_empty_graph_indexes_everything_like_a_fresh_reindex`
9. `reconcile_picks_up_files_added_offline`
10. `reconcile_picks_up_modified_files`
11. `resolve_link_returns_contact_kind_for_contact_node`
12. `resolve_link_returns_file_kind_for_plain_note`

indexer.rs (2):
1. `debounce_coalesces_rapid_writes_into_one_index`
2. `writes_to_disk_get_indexed_after_debounce`

All panic on `Search("embedding model 'BAAI/bge-small-en-v1.5' not downloaded; ...")`. Exactly the failure mode the architect documented in `-18`'s background. Pre-existing — not regression from `-17`. `-18` is the targeted fix; picking that up next per the queue.

### Out-of-scope finding: Windows dead_code lints

The 11 chan-desktop dead_code lints on Windows are a separate platform-cleanup item, NOT in `-17`'s scope. Flagging here so the architect can decide whether to cut a `desktop-N` or `fullstack-b-N` task (depends on lane assignment for desktop IPC) or fold into a broader Windows cleanup pass. The lints will re-surface on the next `ci-12-smoke`-style Windows verification once the `-18` test fix lands and unmasks them in normal CI.

### Branch handling

Per the architect's "audit-trail-keep set" note: `systacean-17-smoke` joins `ci-12-smoke` on the keep list; both prune on the same beat as the `chan-v0.11.99-dryrun.{1..4}` tag cleanup.

### Acceptance criteria recap

* ✓ Local pre-push gate green at commit time.
* ✓ Windows clippy reaches the clippy step + reds on something OTHER than `result_large_err`.
* ✓ Ubuntu clippy passes (test step failure is the pre-existing `-18` scope, not regression).
* ✓ macOS clippy + tests green (verified locally pre-commit).

`-17` acceptance criteria met. Standing by for the architect's audit ack + clearance to proceed with `-18`.
