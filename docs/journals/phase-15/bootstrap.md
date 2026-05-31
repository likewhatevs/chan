# Phase 15 — process bootstrap

**Read order for every lane:** this file -> your `lane-<x>-tasks.md` ->
`plan-round-1.md` (the technical source of truth: root causes + file:line
refs) -> `coordination.md`. Do **not** read or edit `roadmap-round-1.md` —
it is @@Alex's source and is already fully decomposed into `plan-round-1.md`
and the lane task files.

## Goal

Deliver all of `roadmap-round-1.md` as release **v0.20.0**, tested and
validated. Timebox is soft; optimize for parallel progress, not speed.

## No backwards compatibility

This codebase is freshly new with a single user. Do **not** write migration
paths, graceful-degrade fallbacks, or legacy-format handling. Drop old
fields / formats / ids outright and write the new shape directly (session
hash, control-socket frames, preferences, terminal session fields, etc.). We
write the code, test it, document the result as a snapshot, and cut the
release at the end. Never escalate a back-compat question — there is none.

## Roles

- **@@Alex** — the hub. Pokes lanes to re-check tasks and delegate, aligns
  gates, merges periodically. Makes product/scope calls. The only one who
  pushes or tags.
- **Lanes (@@LaneA / @@LaneB / @@LaneC)** — own a scope and a set of files,
  coordinate directly with @@Alex, and cut tasks to each other at shared-file
  boundaries. Lanes may spawn their own subagents within their scope.

## How we work

1. **File ownership.** Each lane owns a set of files (listed in its task
   file). Do not edit a file another lane owns. If your work needs a change
   there, **cut a task** into that lane's task file (append-only) and
   coordinate at the relevant checkpoint.
2. **Append-only tasks.** Once a lane has started a task, new asks become new
   tasks, not rewrites. Amend by appending.
3. **Shared-file discipline.** A few central files are split by *region*
   between lanes (see `coordination.md`). When you commit anything in a
   shared file, collapse `git add <paths>` + `git diff --staged --stat`
   audit + `git commit` into a single chained invocation, and verify with
   `git show --stat HEAD` after. `git add <path>` does not unstage peers —
   always check the staged set.
4. **The gate (before any push).** `cargo fmt --check`,
   `cargo clippy --all-targets -- -D warnings`, `cargo test`,
   `cargo build --no-default-features`, and in `web/`: svelte-check +
   `npm run build`. CI breaks otherwise. The release gate additionally builds
   the **gateway** nested Cargo workspace.
5. **Browser-smoke the runtime-risky bits.** Static gates (svelte-check,
   vitest) miss Svelte-5 runtime reactivity. Anything touching flip
   animation, carousel reactivity, terminal key handling, or live
   server->SPA pushes must be smoked on a running test server. Record
   anything you could not empirically verify as "empirically-unverified" and
   tell @@Alex.
6. **Periodic merges.** Merge gated-green increments to `main` (locally)
   along the way — do not hoard a giant branch to the end. When you land a
   change in a shared file or hit a checkpoint, ping @@Alex + the dependent
   lane so they rebase.
7. **Status to @@Alex.** Curated: highlights / lowlights / contention, plus
   any decision you need. Details live in your task + journal file, not in
   chat.
8. **Self-document.** Write full context into your task file + a journal
   entry as you go; don't rely on @@Alex to relay context between lanes.

## Test server

When you need one, ask @@Alex: new throwaway drive (`/tmp/chan-test-<x>`) vs
reuse an existing registered drive, and what to seed. Build cycle for
frontend changes is full (rust-embed bakes the bundle): stop server ->
`npm run build` in `web/` -> `cargo build -p chan` -> restart, then hard
reload the tab. In the multi-agent run, serve from a renamed binary copy
(e.g. `/tmp/docsrv`) and scope any `pkill` to your own drive path / port so
you do not kill another lane's server.

## Round close

- Lanes that cannot finish a task to a tested state append it to a round-2
  backlog (do not rush it into the release).
- Coordination docs in `docs/journals/phase-15/` stay untracked during the
  round; @@Alex commits the whole tree as one `docs(phase-15)` commit at
  close.
- Release cut (version bump -> gate -> dry-run -> tag) happens only on
  @@Alex's explicit go. "Merge to main" means a local merge; never push or
  tag without an explicit ask.
