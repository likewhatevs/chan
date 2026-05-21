# fullstack-b-16: Launch-time PATH-first probe + binary selection

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Implement the launch-time logic in chan-desktop that picks
between a PATH-installed chan binary and the bundled fallback
from `fullstack-b-15`. Per the LOCKED decision (round-2-plan
decisions table, decision 3): PATH-first with bundled fallback
+ version match.

## Background

* **Round-2 plan decision 3 LOCKED**: PATH-first w/ bundled
  fallback + version match. Reasoning captured in
  [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"Decisions (all locked 2026-05-20)" item 3.
* **Why this shape**:
  * Power users who run their own chan build from this
    checkout get to use it via chan-desktop without
    rebuilding chan-desktop. (`cargo build -p chan` → `cargo
    install --path crates/chan` → chan-desktop picks it up
    on next launch.)
  * Broken / stale PATH installs don't brick the app —
    fallback to bundled keeps the app launchable even when
    `which chan` returns garbage.
  * Version match guard: if the PATH-installed chan's
    version doesn't match the bundled version, fall back to
    bundled. Prevents weird-state crashes from running
    chan-desktop v0.12.0 against `chan` v0.11.0 (or worse,
    v0.13.0-prerelease).
* **Resolution algorithm**:
  1. Try `which chan` (using `std::process::Command::new`
     + `--version` to verify it actually executes).
  2. Parse the version string.
  3. Compare against the bundled binary's version (which is
     the same as chan-desktop's own version since they're
     built from the same checkout).
  4. If match: use the PATH chan.
  5. If no match (or no `chan` on PATH, or PATH chan errors
     on `--version`): use the bundled chan from
     `bundled_chan_path()`.
* **Today's state**: chan-desktop likely invokes `chan` via
  some hardcoded path or a simple `Command::new("chan")` call
  somewhere in `desktop/src-tauri/src/serve.rs` or adjacent.
  This task replaces that with a `resolve_chan_binary()`
  helper that implements the algorithm above.

## Authorization

**Authorization: yes**, this task covers edits to
`desktop/src-tauri/src/serve.rs` (resolution logic + helper),
possibly `desktop/src-tauri/src/main.rs` (call-site wiring),
and `desktop/CLAUDE.md` (documentation). @@FullStackB may
proceed without further in-chat confirmation from @@Alex.

## Acceptance criteria

* New helper `resolve_chan_binary() -> PathBuf` (or
  `Result<PathBuf, _>` if the no-binary case needs error
  surfacing — implementer picks) that implements the
  PATH-first + version-match algorithm.
* Unit tests cover:
  * PATH chan exists + version matches → returns PATH path.
  * PATH chan exists + version mismatch → returns bundled
    path.
  * PATH chan exists + errors on `--version` → returns
    bundled path.
  * No chan on PATH → returns bundled path.
  * Bundled missing AND PATH missing → returns sensible
    error (this case shouldn't happen in a properly-bundled
    chan-desktop but the helper handles it gracefully).
* All chan-desktop chan-invocation call sites updated to use
  the resolved path from `resolve_chan_binary()`.
* `desktop/CLAUDE.md` documents the resolution algorithm +
  the user-facing behavior (power users can override by
  installing chan v0.12.0 to PATH; fresh installs just
  work).
* No regression in chan-desktop boot time (the version-probe
  spawns one subprocess per launch — measure + document if
  it shows up in cold-start latency).
* Pre-push gate: clean.

## How to start

1. Wait for `fullstack-b-15` to land — `bundled_chan_path()`
   is the fallback branch in this algorithm.
2. Grep `desktop/src-tauri/src/` for current chan-invocation
   patterns (`Command::new("chan")`, hardcoded `target/...`
   paths, etc.). Catalog every call site.
3. Implement `resolve_chan_binary()`.
4. Replace call sites.
5. Write the unit tests.
6. Test locally:
   * `cargo install --path crates/chan` (PATH path lights
     up) → launch chan-desktop → confirm via debug logs
     it picked PATH.
   * `cargo uninstall chan` (PATH path goes away) → launch
     chan-desktop → confirm it picked bundled.
   * Version mismatch: install an old chan v0.11.0 to PATH
     → launch chan-desktop → confirm it skipped PATH +
     used bundled.
7. Append commit-readiness to the task tail.

## Coordination

* **Depends on `fullstack-b-15`**: the bundled-path helper
  must be in HEAD before this task can land its fallback
  branch.
* **Independent of @@CI / @@Systacean Wave-1**: this is
  pure chan-desktop / Tauri-side work; no CI or chan-drive
  touch.

## Open questions

(populated as you investigate)

## 2026-05-21 — implementation note

`fullstack-b-15` (`6f4f697` on HEAD) ships `bundled_chan_path()`
and `probe_chan_version()` as the public seam. `-16` builds the
PATH-first resolver on that surface.

### Changes landed

* **`desktop/src-tauri/src/serve.rs`**
  * New `pub fn resolve_chan_binary() -> Result<PathBuf,
    String>`. PATH-first picker per locked decision 3: walks
    `PATH` for a `chan` (or `chan.exe`) entry; probes its
    `--version` and accepts only an EXACT semver match against
    `env!("CARGO_PKG_VERSION")`; falls through to
    `bundled_chan_path()` on any failure (no chan on PATH,
    spawn error, --version error, version mismatch).
  * New `fn resolve_chan_binary_with(...)` — testable core
    factored over its three dependencies (PATH candidate +
    version probe + bundled fallback) so the five acceptance
    branches don't need real subprocesses.
  * New `fn which_chan()` + `fn which_chan_in(path_var, name)`
    — `which`-style lookup factored over the `PATH` value
    string so the test fixture can drive synthetic PATH dirs
    without mutating live `std::env`.
  * New `fn is_executable_file(&Path)` — Unix branch checks
    the `0o111` bits; non-Unix branch accepts any matching
    `.is_file()` (Windows PATHEXT-based: we only look for
    `chan.exe`, so existence is sufficient).
  * `probe_chan_version`'s doc generalized to "any chan
    binary" since `-16` now uses it for the PATH candidate
    too, not just bundled.

* **`desktop/src-tauri/src/main.rs`**
  * Three IPC handlers (`add_drive`, `remove_drive`,
    `set_drive_on`) now resolve via
    `serve::resolve_chan_binary()` instead of
    `serve::bundled_chan_path()`. The `require_bin` gate is
    unchanged.
  * `compute_bin_status()` resolves the binary via
    `resolve_chan_binary()` and runs the existence check +
    version probe on the resolved path. Translocation check
    stays first (PATH chan doesn't rescue a translocated
    install — the broader runtime environment is hostile
    regardless of which `chan` binary is picked).

* **`desktop/CLAUDE.md`**
  * "Resolution helpers" subsection expanded to include
    `resolve_chan_binary()` as the user-facing entry point;
    a new "Resolution algorithm" subsection with the
    state-to-picked-binary table.

### Tests landed

Five new unit tests in `serve.rs::tests` (chan-desktop 21 → 26),
all dependency-injected so no real subprocess spawns:

| Test                                                              | Branch covered                                              |
|-------------------------------------------------------------------|-------------------------------------------------------------|
| `resolve_chan_binary_picks_path_when_version_matches`             | PATH chan exists + version matches → returns PATH path.     |
| `resolve_chan_binary_falls_back_when_path_version_mismatches`     | PATH chan exists + version mismatch (or --version errors) → returns bundled path. |
| `resolve_chan_binary_falls_back_when_no_chan_on_path`             | No chan on PATH → returns bundled path.                     |
| `resolve_chan_binary_surfaces_error_when_bundled_also_missing`    | Bundled missing AND PATH missing → propagates bundled error. |
| `which_chan_in_finds_chan_in_first_matching_path_entry` (Unix)    | Real PATH-walk: first-match-wins, empty PATH returns None, non-executable file is rejected. |

The `which_chan_in` test uses temp-directory fixtures with
executable + non-executable stubs. Cleanup happens at test end.

### Acceptance criteria — verification

| Criterion                                                                            | State                                                                                |
|--------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------|
| New helper `resolve_chan_binary() -> Result<PathBuf, _>` with the PATH-first + version-match algorithm | Landed in `serve.rs`.                                                                |
| Unit test: PATH chan exists + version matches → PATH path                            | `resolve_chan_binary_picks_path_when_version_matches`.                               |
| Unit test: PATH chan exists + version mismatch → bundled                             | `resolve_chan_binary_falls_back_when_path_version_mismatches`.                       |
| Unit test: PATH chan exists + errors on `--version` → bundled                        | Same probe-error branch as version-mismatch (probe returns `Err` either way).        |
| Unit test: no chan on PATH → bundled                                                 | `resolve_chan_binary_falls_back_when_no_chan_on_path`.                               |
| Unit test: bundled missing AND PATH missing → sensible error                         | `resolve_chan_binary_surfaces_error_when_bundled_also_missing`.                      |
| All chan-invocation call sites updated                                               | Three IPC handlers + `compute_bin_status` route via `resolve_chan_binary()`.         |
| `desktop/CLAUDE.md` documents the algorithm + user-facing behavior                   | Expanded "Resolution helpers" + new "Resolution algorithm" subsection.               |
| No regression in chan-desktop boot time                                              | One extra `chan --version` subprocess at boot (only when PATH has a chan). Measure deferred to runtime verification by @@WebtestB; one-time per boot, run on a thread before AppState binds. |
| Pre-push gate                                                                        | Workspace fmt + clippy `--workspace -D warnings` + test (chan-desktop 21 → 26) + no-default-features build + svelte-check (3978 files / 0 errors) all green. |

### Coordination footprint

* No overlap with @@CI's ci-7 territory (`.github/workflows/`,
  `docs/journals/phase-8/ci/`).
* No overlap with @@Systacean's systacean-11 (signing keys in
  `tauri.conf.json`) or systacean-13 (notarytool keychain
  profile in `desktop/Makefile`). My `-16` only touches
  `serve.rs` + `main.rs` + `CLAUDE.md`'s Resolution-helpers
  subsection (separate region of CLAUDE.md from the
  notarization section others may still be staging).

### Suggested commit subject

```
chan-desktop: resolve_chan_binary() PATH-first with bundled fallback (fullstack-b-16)
```

Touches:
* `desktop/src-tauri/src/serve.rs`
* `desktop/src-tauri/src/main.rs`
* `desktop/CLAUDE.md` (Resolution-helpers subsection only)

Holding for @@Architect commit clearance. Push held for the
Round-2 release tag (end of Round 2). After this commit,
queue empty for Wave-1.

### Runtime verification (deferred)

`-16`'s algorithm is unit-tested via dependency injection; the
real end-to-end (PATH chan vs bundled chan, version-mismatch
fallback, etc.) is @@WebtestB's lane-B walkthrough. Standing
chan-desktop runtime permission covers a debug build with
chan installed/uninstalled at the matching version if @@WebtestB
needs me to set up the fixture; otherwise their lane drives the
walkthrough directly.
