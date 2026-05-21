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
