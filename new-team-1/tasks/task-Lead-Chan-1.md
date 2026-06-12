# task-Lead-Chan-1 — tidy-up: core workspace + frontend

From: @@Lead. To: @@Chan. Round plan: new-team-1/round-1-plan.md (READ
IT FIRST — exemptions, commit discipline, cross-workspace rules live
there).

## Your scope

You own `crates/**` and `web/**` — the root cargo workspace and the
Svelte frontend, including their design.md files.

## Work items

### 1. Archaeology scrub
- Remove/rewrite comments referencing phases (~53 hits in crates/, ~19
  in web/src; `grep -rniE 'phase[- ]?[0-9]+'`) and agent handles
  (`grep -rniE '(fullstack|lane[A-E]\b|webtest|desktect|architect)'`,
  e.g. crates/chan/src/main.rs:1672 "fullstack-b-28b slice ii",
  crates/chan-report/src/summary.rs:15 "the architect's option (c)").
  A comment that only narrates history gets deleted; a comment that
  carries a real constraint gets rewritten to state the constraint
  without the history.
- Neutralize internal handles in test fixtures and USER-VISIBLE help
  text: crates/chan-shell/src/wire.rs tests (@@LaneB/C/D) and
  crates/chan-shell/src/cli.rs help examples → generic handles
  (@@Alice, @@Bob). Keep test semantics identical.
- Legitimate product `@@` tab-handle syntax stays.

### 2. Hygiene
- Compilation warnings to zero on your surfaces: `cargo clippy
  --workspace --all-targets` and `make web-check` (svelte-check +
  vitest + vite build warnings).
- Functions/constructors with >5-6 parameters → config structs. For
  any public signature change: whole-repo grep for call sites — fix
  desktop/ + gateway/ call sites IN THE SAME COMMIT (announce in your
  completion task; they are separate workspaces, your -p check won't
  see them).
- Duplicated code: dedupe where it clearly simplifies; skip anything
  that needs a behavior decision (flag it instead).
- Non-idiomatic Rust/TS/Svelte: fix what's clear-cut.
- Shortcuts/keybindings declared or handled OUTSIDE the main shortcut
  store in web/src: normalise into the store. This is a named ask from
  @@Alex — enumerate the offenders in your completion task even if some
  are too risky to move this round.

### 3. Design docs (current-snapshot rewrite)
- crates/chan-tunnel-client, chan-report, chan-llm, chan-tunnel-proto,
  chan-workspace, chan-tunnel-server design.md; web/src/design.md;
  web/src/editor/design.md.
- Read the source first, then make each design.md describe the current
  system: no phase numbers, no agent names, no "now/recently" deltas.
  Delete stale sections outright (pre-release: no back-compat notes).

## Own-gate (run AFTER your last edit)
- `cargo fmt --check` (workspace)
- `cargo clippy --workspace --all-targets` — zero warnings
- `cargo test --workspace`
- `make web-check`
If a signature change touched desktop/ or gateway/: also
`cargo check` in desktop/src-tauri and gateway/.

## Coordination
- Behavior-preserving only; flag judgment calls in the completion task.
- Pathspec-atomic commits to main, no push (see round plan).
- desktop/ + gateway/ files: touch ONLY for mechanical call-site fixes
  riding your signature commits.
- Done = completion task new-team-1/tasks/task-Chan-Lead-N.md
  (warnings before/after, param-struct refactors made, shortcut
  offenders found/moved, design docs rewritten, judgment flags) + poke
  @@Lead. Journal as you go.
