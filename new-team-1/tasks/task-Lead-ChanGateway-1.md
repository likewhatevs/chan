# task-Lead-ChanGateway-1 — tidy-up: gateway

From: @@Lead. To: @@ChanGateway. Round plan:
new-team-1/round-1-plan.md (READ IT FIRST — exemptions, commit
discipline, cross-workspace rules live there).

## Your scope

You own `gateway/**` (a SEPARATE cargo workspace from the root) plus
one manual page: `docs/manual/gateway.md`.

## Work items

### 1. Archaeology scrub
- Code recon found 0 phase mentions in gateway/ — verify with your own
  sweep (`grep -rniE 'phase[- ]?[0-9]+' gateway/` and
  `grep -rniE '(fullstack|lane[A-E]\b|webtest|desktect|architect)'`)
  including design.md files and any READMEs in the tree, which my
  code-only grep missed.

### 2. Hygiene
- Compilation warnings to zero: `cargo fmt --check`, `cargo clippy
  --workspace --all-targets`, `cargo test --workspace` — all run INSIDE
  gateway/ (it is not part of the root workspace).
- Functions/constructors with >5-6 parameters → config structs. If a
  refactor would touch types defined in crates/** report it to @@Lead
  instead — @@Chan owns that surface.
- Duplicated code: the gateway crates (workspace-proxy, identity,
  admin, profile, gateway-common) grew fast — look for repeated
  handler/config/error boilerplate that belongs in gateway-common.
- Non-idiomatic Rust: fix what's clear-cut.

### 3. Docs (current-snapshot rewrite)
- gateway/crates/{workspace-proxy,identity,admin,profile,
  gateway-common}/design.md: read the source first, then make each
  describe the current system. No phase numbers, no agent names, no
  "now/recently" deltas. Delete stale sections outright.
- docs/manual/gateway.md: verify every claim against the current
  gateway behavior; fix what's stale. This is user-facing — ground
  every capability description in source, don't infer from names.

## Own-gate (run AFTER your last edit)
- In gateway/: `cargo fmt --check` + `cargo clippy --workspace
  --all-targets` (zero warnings) + `cargo test --workspace`.

## Coordination
- Behavior-preserving only; flag judgment calls in the completion task.
- Pathspec-atomic commits to main, no push (see round plan).
- @@Chan may land mechanical call-site fixes in gateway/** when core
  signatures change — expected; coordinate via tasks if you're mid-edit
  in the same file.
- Yours is the smallest lane — when done, poke @@Lead; I may route a
  second-pass review of another lane's diff to you.
- Done = completion task new-team-1/tasks/task-ChanGateway-Lead-N.md
  (warnings before/after, refactors, design-doc + manual state, flags)
  + poke @@Lead. Journal as you go.
