# task-Lead-ChanDesktop-1 — tidy-up: desktop

From: @@Lead. To: @@ChanDesktop. Round plan:
new-team-1/round-1-plan.md (READ IT FIRST — exemptions, commit
discipline, cross-workspace rules live there).

## Your scope

You own `desktop/**` — the Tauri workspace (desktop/src-tauri) and
desktop/design.md.

## Work items

### 1. Archaeology scrub
- Remove/rewrite comments referencing phases (~17 hits;
  `grep -rniE 'phase[- ]?[0-9]+' desktop/`) and agent handles
  (`grep -rniE '(fullstack|lane[A-E]\b|webtest|desktect|architect)'`).
  A comment that only narrates history gets deleted; one that carries a
  real constraint gets rewritten to state the constraint without the
  history. Legitimate product `@@` tab-handle syntax stays.

### 2. Hygiene
- Compilation warnings to zero: `cargo clippy --all-targets` in
  desktop/src-tauri (local macOS cfg). The Linux/x86_64 cfg branch
  can't be compiled locally — eyeball it for the same patterns and note
  in your completion task that it's compile-unverified (CI dry-run is
  the only check; don't trigger one yourself).
- Functions/constructors with >5-6 parameters → config structs (your
  workspace only; if you find offenders in crates/** report them to
  @@Lead — @@Chan owns that surface).
- Duplicated code: dedupe where it clearly simplifies; flag anything
  needing a behavior decision.
- Non-idiomatic Rust: fix what's clear-cut. Window-management code
  just landed in v0.31.x — expect leftover scaffolding-style comments
  there.

### 3. Design doc (current-snapshot rewrite)
- desktop/design.md: read the source first, then make it describe the
  current system — bury-on-close, remote windows, launcher singleton,
  standalone-terminal control socket, etc. as they ARE. No phase
  numbers, no agent names, no "now/recently" deltas. Delete stale
  sections outright.

## Own-gate (run AFTER your last edit)
- `cargo fmt --check` + `cargo clippy --all-targets` (zero warnings) +
  `cargo test` in desktop/src-tauri.
- If web/dist is needed to build: `npm run build` in web/ is fine to
  RUN but web/** is @@Chan's surface — do not edit it.

## Coordination
- Behavior-preserving only; flag judgment calls in the completion task.
- Pathspec-atomic commits to main, no push (see round plan).
- @@Chan may land mechanical call-site fixes in desktop/** when core
  signatures change — expected; don't fight them, coordinate via tasks
  if you're mid-edit in the same file.
- WKWebView/desktop-runtime behavior is NOT verifiable from Chrome —
  this round should not need runtime verification (comments/refactors),
  but if a change is risky without it, flag instead of shipping.
- Done = completion task new-team-1/tasks/task-ChanDesktop-Lead-N.md
  (warnings before/after, refactors, design-doc state, flags) + poke
  @@Lead. Journal as you go.
