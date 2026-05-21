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
