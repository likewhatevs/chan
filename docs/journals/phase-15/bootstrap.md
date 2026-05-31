# Phase 15 round 2 - process bootstrap

**Read order for every lane:** this file -> your `lane-<x>-tasks.md` ->
`round-2-part-1.md` + `round-2-part-2.md` (the technical source of truth: bugs,
features, root causes + file:line refs) -> `coordination.md`. @@Architect
decomposes the round-2 part docs into the lane task files; workers read their
own task file plus the part docs it points at.

## Goal

Deliver all of `round-2-part-1.md` + `round-2-part-2.md` as release
**v0.21.0**, tested and validated. Timebox is soft; optimize for parallel
progress, not speed.

## No backwards compatibility

This codebase is freshly new with a single user. Do **not** write migration
paths, graceful-degrade fallbacks, or legacy-format handling. Drop old
fields / formats / ids outright and write the new shape directly (session
hash, control-socket frames, preferences, terminal session fields, survey /
event wire shapes, etc.). We write the code, test it, document the result as a
snapshot, and cut the release at the end. Never escalate a back-compat
question - there is none.

## Roles

- **@@Architect** - the hub, gate, and coordinator; the only agent that talks
  to @@Host. Decomposes round-2 into lane task files; organizes the waves and
  the testing coordination; reviews and validates every outcome; owns the
  general documentation. Delegates and enforces the same standard of code
  quality and human-facing comment quality (the writing rules below) on every
  worker. Aligns the gate, sequences merges, makes the obvious calls, and
  consolidates to @@Host only when a product / scope / risk decision is needed.
- **@@Host** - makes product / scope calls. The only one who pushes or tags,
  and the only one who cuts the release. Talks to @@Architect, not the workers.
- **@@Lane-1 / @@Lane-2 / @@Lane-3** - workers. Each owns a scope and a set of
  files (listed in its task file), owns its own append-only journal + task
  file, and coordinates **through @@Architect** (not @@Host directly). Cut
  tasks to a peer at shared-file boundaries. May spawn subagents within scope.

## How we work

1. **File ownership.** Each lane owns a set of files (listed in its task
   file). Do not edit a file another lane owns. If your work needs a change
   there, **cut a task** into that lane's task file (append-only) and
   coordinate at the relevant checkpoint via @@Architect.
2. **Append-only tasks.** Once a lane has started a task, new asks become new
   tasks, not rewrites. Amend by appending.
3. **Shared-file discipline.** A few central files are split by *region*
   between lanes (see `coordination.md`). When you commit anything in a
   shared file, collapse `git add <paths>` + `git diff --staged --stat`
   audit + `git commit` into a single chained invocation, and verify with
   `git show --stat HEAD` after. `git add <path>` does not unstage peers -
   always check the staged set.
4. **The gate (before any push).** `cargo fmt --check`,
   `cargo clippy --all-targets -- -D warnings`, `cargo test`,
   `cargo build --no-default-features`, and in `web/`: svelte-check +
   `npm run build`. CI breaks otherwise. The release gate additionally builds
   the **gateway** nested Cargo workspace.
5. **Browser-smoke the runtime-risky bits.** Static gates (svelte-check,
   vitest) miss Svelte-5 runtime reactivity and CodeMirror / xterm timing.
   Anything touching editor decorations, carousel reactivity, terminal key
   handling, agent submit, or live server->SPA pushes (the survey bubbles)
   must be smoked on a running test server. Terminal-key / agent-submit work
   additionally needs a **real-agent** smoke (a running claude/codex in the
   terminal), not just a shell. Record anything you could not empirically
   verify as "empirically-unverified" and tell @@Architect.
6. **Periodic merges.** Merge gated-green increments to `main` (locally) along
   the way - do not hoard a giant branch to the end. When you land a change in
   a shared file or hit a checkpoint, tell @@Architect so the dependent lane
   rebases.
7. **Status to @@Architect.** Curated: highlights / lowlights / contention,
   plus any decision you need. Details live in your task + journal file, not in
   chat. @@Architect consolidates to @@Host.
8. **Self-document.** Write full context into your task file + a journal entry
   as you go; don't rely on @@Architect to relay context between lanes.
9. **Completion protocol.** When you finish a task, do **both**:
   (a) append the completion to your event file, and
   (b) poke the target so it picks the event up immediately. CK-SUBMIT resolved
   how a poke submits into a *running agent*: Shift+Enter now inserts a newline
   (`\n`) by design, so a poke must append the agent submit chord to actually
   submit. Use:
   `cs term write --tab-name=<target> $'poke from <agent>: check <path>\x1b[27;9;13~'`
   (the trailing `\x1b[27;9;13~` is the Meta+Enter submit chord; a bare `\n` only
   inserts a newline, a bare `\r` is unreliable). NOTE: the live session runs the
   installed v0.20.0 app, so the running CLI is still `cs term`; the CK-RENAME
   `cs terminal` name ships in v0.21.0 and only applies once the app rebuilds.
   When the target is **@@Host**, the survey-bubble surface (feature 2.3) is
   deferred to round-3, so route Host-targeted pokes through the event file +
   @@Architect for now.

## Test server

When you need one, ask @@Architect: new throwaway drive (`/tmp/chan-test-<x>`)
vs reuse an existing registered drive, and what to seed. Build cycle for
frontend changes is full (rust-embed bakes the bundle): stop server ->
`npm run build` in `web/` -> `cargo build -p chan` -> restart, then hard
reload the tab. In the multi-agent run, serve from a renamed binary copy
(e.g. `/tmp/docsrv`) and scope any `pkill` to your own drive path / port so
you do not kill another lane's server.

## Round close

- Lanes that cannot finish a task to a tested state append it to a round-3
  backlog (do not rush it into the release).
- Coordination docs in `docs/journals/phase-15/` stay untracked during the
  round; @@Architect commits the whole tree as one `docs(phase-15)` commit at
  close.
- Release cut (version bump -> gate -> dry-run -> tag) happens only on
  @@Host's explicit go. "Merge to main" means a local merge; never push or
  tag without an explicit ask from @@Host.
