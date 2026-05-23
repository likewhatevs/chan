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
