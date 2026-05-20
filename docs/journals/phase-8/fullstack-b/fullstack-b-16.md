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
