# new-team-1 round 1 — tidy-up: archaeology scrub + hygiene + docs

Opened 2026-06-12 by @@Lead from @@Alex's scope. Baseline: main @ 37a0f001
(v0.31.1, clean, in sync with origin).

## Scope (from @@Alex, verbatim intent)

1. **Archaeology scrub** — code comments and docs referencing phases
   ("phase-14", "phase 22") or agent handles ("fullstack-x:", "@@LaneB",
   "the architect's option (c)") are artifacts of how the system evolved
   and are meaningless to anyone else. Comments and docs must reflect the
   CURRENT snapshot of the code, not read like a changelog.
2. **Hygiene per lane** — non-idiomatic code, compilation warnings
   (svelte, frontend, backend), functions with >5-6 params get config
   structs, duplicated code deduplicated, ad-hoc fixups normalised (e.g.
   shortcuts declared outside the main shortcut store). End state:
   cleaner, simpler, consumable by humans AND agents.
3. **Docs dedup** — README duplicates manual content; README should
   point to the manual instead.
4. **Design docs up to date** — every design.md reflects the current
   system, grounded in source.

## Exemptions (Lead's call, flagged to @@Alex)

- `CHANGELOG.md` — explicitly exempt per the scope.
- `docs/phases/**` — the project's historical record by design,
  analogous to the changelog. Scrubbing phase names out of phase
  journals would destroy their purpose. EXEMPT.
- `dev/`, `new-team-1/` — untracked scratch / live coordination bus.
- Historical entries inside CHANGELOG-like sections elsewhere: none
  known; if found, route to @@Lead.

Test fixtures and CLI help/examples that use internal handles
(@@LaneB, @@LaneC/D in crates/chan-shell wire.rs tests and cli.rs help
text) are NOT exempt: help text is user-visible. Neutralize to generic
example handles (e.g. @@Alice, @@Bob) without changing test semantics.

## Lanes and file ownership (non-overlapping)

| lane          | owns                                                          |
|---------------|---------------------------------------------------------------|
| @@Chan        | crates/**, web/** (incl. their design.md files)               |
| @@ChanDesktop | desktop/** (incl. desktop/design.md)                          |
| @@ChanGateway | gateway/**, docs/manual/gateway.md                            |
| @@Lead        | README.md, design.md (root), docs/** (minus phases/ and manual/gateway.md), CLAUDE.md, Makefile/root files, integration gate, round close |

## Cross-cutting rules (every lane)

- **Refactor round = behavior-preserving.** If a cleanup would change
  behavior, flag it in your completion task instead of shipping it.
- **Shared worktree commit discipline:** commit with
  `git commit -F <msgfile> -- <explicit paths>` (pathspec; flags before
  `--`), check `git diff --staged --stat` before and
  `git show --stat HEAD` after. Never plain `git add` + `git commit`.
- **Commit to main locally. NEVER `git push`** — @@Alex owns push.
- **Cross-workspace ripples:** desktop/ and gateway/ are SEPARATE cargo
  workspaces that construct core-crate types. Any public signature /
  required-field change in crates/**: grep the WHOLE repo (both
  casings), fix ALL call sites incl. desktop/ + gateway/ in the SAME
  commit (mechanical cross-boundary edits allowed — announce them in
  your completion task), and re-verify `cargo check` green before
  pausing. Don't leave a non-compiling window for peers.
- **Gate AFTER the last edit** (a check that ran before a later edit is
  stale). Scoped own-gates per lane are in the task files; @@Lead runs
  the full `make pre-push` on committed state in an isolated worktree
  at integration points.
- **Web changes:** own-gate is `make web-check` (svelte-check + vitest
  + build), not svelte-check alone. If component reactivity logic
  changes, flag it — static gates miss Svelte 5 runtime errors.
- **Design docs:** rewrite to describe what IS, grounded in the source
  you just read — no phase numbers, no agent names, no "recently
  changed/now does X" framing. Delete changelog-style sections.
- Journals append-only in new-team-1/journals/journal-{you}.md.
  Completion = task file back to @@Lead in new-team-1/tasks/ + 1-line
  poke. Decisions for the host route through @@Lead.

## Recon numbers (starting points, not exhaustive)

- Phase mentions in code: crates/ 53, web/src 19, desktop/ 17, gateway/ 0.
  Pattern: `grep -rniE 'phase[- ]?[0-9]+'`
- Agent-handle artifacts in code: ~173 across comments, wire.rs test
  fixtures, cli.rs help examples.
  Pattern: `grep -rniE '(fullstack|lane[A-E]\b|webtest|desktect|architect|@@[A-Z])'`
  (filter legitimate product `@@` tab-handle syntax by hand).
- Docs archaeology outside phases/: 21 hits.
- design.md inventory: root; crates/{chan-tunnel-client,chan-report,
  chan-llm,chan-tunnel-proto,chan-workspace,chan-tunnel-server};
  web/src; web/src/editor; desktop; gateway/crates/{workspace-proxy,
  identity,admin,profile,gateway-common}.

## Sequencing

All three lanes run in parallel (disjoint files). @@Lead works the docs
lane concurrently. Integration: lanes commit coherent chunks as they
finish; @@Lead full-gates committed state, then round-close docs +
retrospective. No test servers expected this round; if a lane needs a
browser smoke, serve `--standalone` on a unique port and tear down.

## Addendum 2026-06-12: sweep tooling (ratified from @@Chan's finding)

The sandbox grep shim silently skips large files — a 130KB file
returns phantom "no match" (empirically confirmed: 221 real matches
rg finds, grep returns nothing). ALL sweeps — archaeology patterns,
chanwriter purge, call-site greps for signature changes — MUST use
`rg --text`, not grep. Also do not filter sweeps to *.md/*.rs only:
shell scripts and configs carry handles too (two docs/release/*.sh
files hid @@-handles from my md-filtered sweep). Re-run any sweep you
already did with the shim before reporting your surface clean.

## Addendum 2 (2026-06-12): own-gates must match the real gate's flags

The integrated gate caught a deprecation that a lane's bare
`cargo clippy` own-gate read as green: pre-push builds with
`RUSTFLAGS="-D warnings"` (clippy additionally `-- -D warnings`), so
warnings that look advisory locally are hard errors there. From now
on every Rust own-gate runs with `RUSTFLAGS="-D warnings"` on both
clippy and test. (Incident: NSFilenamesPboardType deprecation in
dropped_paths.rs, task-Lead-ChanDesktop-8.)
