# systacean-44 — Round-3 Track-3 backend cleanup + hardening pass

Owner: @@Systacean
Phase: 8, Round 3
Date cut: 2026-05-23

## Goal

One time-boxed cleanup + hardening pass on the Rust workspace (chan-drive / chan-server / chan-llm / chan-tunnel-* / chan-report / chan). Produces a written report at task tail; fix P0/P1 release-blockers in-task, defer P2+ to v0.14+.

## Background

Round-3 Track 3 (cleanup / hardening) per [`../architect/round-3-plan.md`](../architect/round-3-plan.md). @@Alex's locked scope shape: **one wave per agent, time-boxed**. Round closes when no release-blockers remain; minor polish opportunities defer to v1.x.

Phase-8 shipped substantial backend work (Drafts saga end-to-end, Team feature backend, cross-platform CI, BM25 fallback, chan-report extensions, pre-flight feature toggles, mention endpoint, PTY soft-wrap flake killer, macOS updater verify). The result is a large + active Rust surface that benefits from a sweep before v0.13.0 ships.

## Scope (three sub-passes, in this order)

### 1. Rust dead-code + error-path sweep

* `cargo +nightly udeps` (or `cargo-machete` if you prefer; either is fine — pick whichever fits) across the workspace. Per-finding: confirm it's actually dead before deleting.
* Audit `.unwrap()` / `.expect()` usage in code paths that can be reached by user input (HTTP handlers, CLI argv parsing, drive ops). Look for any that could panic on adversarial input.
* Grep for `// TODO` / `// FIXME` / `// XXX` in `crates/`; triage each (still relevant / stale / P0+).
* `cargo +nightly clippy --all-targets -- -W clippy::pedantic -A clippy::module_name_repetitions -A clippy::missing_errors_doc -A clippy::missing_panics_doc` (or similar — selective pedantic). Don't fix everything; flag P0/P1 patterns + skim findings. Real fixes only.

### 2. CLI error-message audit + polish

@@Alex's standing theme: "we need to up our cmdline game by a lot." Seed example: `chan serve` bind-port error doesn't name the port that's in use.

* Walk every user-facing error path in `crates/chan/src/main.rs` + the chan-drive / chan-server errors that surface through it.
* Every user-facing error names the input that produced it (port, path, env var, secret name, etc.).
* Errors should be actionable — what to do next, not just what went wrong.
* Standard pattern: include the offending value in the error message (with redaction for tokens / secrets).

Examples of the target shape:

| Before | After |
|---|---|
| `Error: address in use` | `Error: address 127.0.0.1:8787 already in use (try a different --port; previous chan serve may still be running?)` |
| `Error: not found` | `Error: drive '/Users/foo/bar' not found in registry (use 'chan add <path>' first; 'chan list' shows registered drives)` |
| `Error: invalid path` | `Error: path '/Users/foo/../etc' rejected: traverses outside drive root` |

### 3. Input-validation pass at chan-server route boundaries

Use the `security-review` skill against chan-server's HTTP surface.

* Walk every route handler in `crates/chan-server/src/routes/*.rs`. For each: what input does it take, and what would happen on adversarial input (path traversal, oversized payload, malformed JSON, missing fields)?
* chan-drive's `Drive` already enforces the path sandbox + special-file refusal — that's the load-bearing safety net. This audit confirms chan-server handlers don't bypass it (e.g., direct `std::fs` access; unsafe canonicalization).
* Per-finding: log + fix in-task if cheap (input validation is usually small); file as follow-up if structural change required.

## Acceptance criteria

1. **Dead-code sweep**: tool ran + report (X items flagged, Y deleted, Z preserved with rationale). `unwrap()` audit findings categorized (acceptable static-init unwrap vs adversarial-input panic).
2. **CLI error-message audit**: error paths walked + report listing the changes (or commits if landed) + the ones deferred.
3. **Input-validation pass**: every chan-server route reviewed + report. P0/P1 fixed in-task; P2+ filed.
4. **Final report at task tail**:
   * What was found (counts + categorisation).
   * What was fixed in-task (commit references).
   * What was deferred (severity + follow-up task link if filed).
5. **All gate checks pass**: cargo fmt + clippy `-D warnings` + cargo test + `cargo build --no-default-features`.

## How to start

1. `cargo +nightly udeps --all-targets` (install via `cargo install cargo-udeps --locked` if needed). Audit output.
2. `rg '\.(unwrap|expect)\(' crates/ | grep -v 'tests'` — survey + classify.
3. CLI errors: read `crates/chan/src/main.rs` top-down + identify every `eprintln!("Error: ...")` or returned error; refactor for input-naming.
4. Use the `security-review` skill on chan-server's `routes/` directory.
5. Fix in-task as you go (each fix is a separate atomic commit); end-of-task append produces the report.

## Coordination

* Time-boxed: ONE pass per sub-pass. Round closes when no release-blockers remain.
* P0 (data-loss / crash on common path / safety bypass) → fix in-task, flag for @@WebtestA walk.
* P1 (poor error UX, narrowly-reachable panic) → fix in-task if cheap, file as blocker if not.
* P2 (polish opportunities) → defer to v1.x; log in report.
* Subject convention: `crate: <area>: <fix> (systacean-44 sub-N)`.

If a fix touches @@FullStackA's chan-server lane (route handlers), poke first — they have `fullstack-a-97` (terminal renderer P0 release blocker) + `fullstack-a-96` sub-passes 1/2/3 active. Coordinate timing.

If a fix touches `desktop/` (unlikely for chan-core cleanup): poke @@Architect; cross-team bridge via @@Alex.

## Authorization

Yes for chan-core Rust edits (`crates/chan-drive/`, `crates/chan-server/`, `crates/chan-llm/`, `crates/chan-tunnel-*/`, `crates/chan-report/`, `crates/chan/`) + Rust tests + new dev-deps if needed (e.g., cargo-udeps) + task-tail report.

## Out of scope

* Wholesale refactoring of any single crate's module structure. The hardening cap is one pass; structural rework is post-v0.13.0.
* chan-tunnel-proto protocol changes — protocol seam co-evolves with chan-desktop; cross-team via @@Alex.
* SPA / web/ changes (@@FullStackA's lane).
* CI / workflow changes (@@CI's lane).
* desktop/ changes (chan-desktop's lane).

## 2026-05-23 15:52 BST — report

Track-3 one-wave pass complete.

### Fixed

* `crates/chan-tunnel-client/Cargo.toml`: removed confirmed unused normal deps flagged by `cargo machete` (`anyhow`, `async-trait`, `bytes`, `http-body`, `http-body-util`, `pin-project-lite`).
* `crates/chan-tunnel-server/Cargo.toml`: removed confirmed unused normal deps (`anyhow`, `http-body`, `pin-project-lite`, `serde`, `serde_json`, `tower`).
* `crates/chan-tunnel-server/Cargo.toml`: made the integration-test-only `reqwest` `stream` feature explicit after dependency cleanup exposed the prior feature-unification assumption.
* `crates/chan/src/main.rs`: local `chan serve` bind failures now carry the requested listen address via `running server on {addr}`.
* `crates/chan-server/src/routes/terminal.rs`: watcher event listing now skips matching event files larger than 1 MiB before `read_to_string`, with a regression test.

### Dead-code / error-path sweep

* `cargo machete`: initially found 12 unused dependency edges across `chan-tunnel-client` and `chan-tunnel-server`; after the patch it reports no unused dependencies.
* `cargo clippy --all-targets -- -W clippy::pedantic -A clippy::module_name_repetitions -A clippy::missing_errors_doc -A clippy::missing_panics_doc`: warning-only. Mostly doc markdown, must-use candidates, cast/format/style lints. No P0/P1 pattern worth broadening the pass.
* `unwrap` / `expect` audit: user-input-adjacent production candidates were inspected. Static invariant unwraps/expectations in tunnel setup and serializer shape were left alone. Lock poisoning unwraps in route/state code are still present, classified P2 structural cleanup rather than adversarial input panic.
* TODO/FIXME/XXX scan: no release blocker. Noted existing P2s: sub-second mtime race comment in `routes/files.rs`; terminal slash-command automation TODO in `terminal_sessions.rs`.

### Route-boundary validation

Manual route-boundary review covered `crates/chan-server/src/routes/*.rs`; `security-review` skill was requested by dispatch but is not installed in this session.

* Drive content paths mostly route through chan-drive `Drive` / `resolve_safe*`, preserving the sandbox and special-file refusal.
* Direct filesystem access reviewed:
  * `terminal.rs`: watcher dirs intentionally bypass drive sandbox after settings-gated attach; hardened oversized event reads in-task.
  * `fonts.rs`: hardcoded Adobe GitHub URLs + config-dir writes; no user-controlled URL/path.
  * `files.rs`, `graph.rs`, `inspector.rs`, `fs_graph.rs`: direct metadata/read paths are behind drive-root resolution or graph-derived drive-relative paths; no cheap P0/P1 bypass found.

### Verification

* `cargo fmt`: pass.
* `cargo machete`: pass, no findings.
* `cargo check -p chan-tunnel-client -p chan-tunnel-server`: pass.
* `cargo test -p chan-server routes::terminal::tests`: pass, 28/0.
* `cargo test -p chan-server routes::terminal::tests::list_watcher_events`: pass, 2/0.
* `cargo test -p chan-tunnel-server`: pass, 42/0 across unit + e2e.
* `cargo test -p chan --bin chan parse_idle_timeout`: pass, 2/0.
* `cargo clippy --all-targets -- -D warnings`: pass.
* `cargo build --no-default-features`: pass.
* `cargo test`: failed under parallel resource pressure with three `chan-drive` reindex tests reporting macOS `Too many open files`; targeted rerun `cargo test -p chan-drive --lib drive::tests::reindex -- --test-threads=1` passed, 8/0. Classified as environment/resource pressure, not this patch.

### Deferred

* P2: convert route/state `Mutex::lock().unwrap()` / `RwLock` unwraps to explicit 500s where it improves crash containment. Broad, mechanical, and not adversarial-input reachable in this pass.
* P2: CLI error copy still has many polish opportunities beyond the bind-address seed; no release blocker found.

## 2026-05-23 — @@Architect: approved + commit clearance

Report reviewed. Clean Track-3 pass. All five concrete fixes are well-scoped:

* Tunnel-* unused-deps cleanup (12 edges) — `cargo machete` clean post-patch is the right verification.
* `reqwest` `stream` feature explicit — good catch; feature-unification leaks happen quietly.
* `chan serve` bind-failure now names the address — exactly the CLI-error-polish seed @@Alex flagged. Good first cut of the standing theme.
* `routes/terminal.rs` watcher event 1 MiB cap — pre-`read_to_string` is the right placement. Regression-test pin confirms it.

### Deferred items acknowledged

* P2 `Mutex/RwLock` poisoned-unwrap → explicit 500: agreed P2; not adversarial-input reachable; structural cleanup for v0.14+ or later.
* P2 broader CLI error-message polish: agreed; the bind-address seed lands the pattern; rest is polish territory.

### Security-review skill availability

Noted that the skill wasn't installed in your session. Manual route-boundary review covered the same surface and produced clear findings. Suggest @@Alex / config — `security-review` is on the user-invocable list per the bootstrap reminder. Not blocking; flagging for future polishing.

### Suggested commit subject

```
crates: Round-3 Track-3 cleanup + hardening (systacean-44: tunnel-* unused-deps removed; chan serve bind-address in error; chan-server watcher event 1 MiB cap; reqwest stream feature explicit)
```

### Commit instructions

Per the standing pre-authorization for your lane:

* Per-path `git add` only (multi-agent worktree). Stage:
  * `crates/chan-tunnel-client/Cargo.toml`
  * `crates/chan-tunnel-server/Cargo.toml`
  * `crates/chan/src/main.rs`
  * `crates/chan-server/src/routes/terminal.rs`
  * any test-file additions
  * `Cargo.lock` if dep removal touched it
  * `docs/journals/phase-8/systacean/systacean-44.md`
  * `docs/journals/phase-8/alex/event-systacean-architect.md` (if appended)
* Pre-commit `git diff --staged --stat` + post-commit `git show --stat HEAD`.

### macOS open-files test pressure

The `Too many open files` cargo test failure is a known macOS test-environment ceiling, not your patch. Classification is correct. Targeted rerun at `--test-threads=1` passing is sufficient.

### Lane state post-`-44`

Queue-empty. Round-3 Track-3 cleanup done for the chan-core backend. Standing by until v0.13.0 cut beat or any follow-up.

Thank you for the careful + tight pass.
