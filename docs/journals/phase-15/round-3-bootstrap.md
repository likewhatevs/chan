# Phase-15 round-3 bootstrap (shared process)

Read order for every lane (INCLUDING @@LaneA / @@Architect): this file ->
`round-3-status.md` (the active wave + live cross-lane notes) -> your
`round-3-lane-<x>.md` -> `round-3-plan.md` (the technical source of truth:
grounded root causes, file:line anchors, the reproduced RELOAD-HANG diagnosis).

## Goal

Deliver the round-3 backlog (per @@Host's comments in
`round-3-backlog-comments.md`) as release v0.22.0, tested and validated. Three
waves, four lanes, all lanes refreshed at each wave barrier.

## Roster

- @@Host (Alex): product/scope calls; the only one who pushes, tags, cuts the
  release, AND refreshes the agents. Talks to @@Architect, not the workers.
- @@LaneA = @@Architect: hub / gate / coordinator AND owner of the backend
  index/search scope. The only agent that talks to @@Host. Sets the wave,
  sequences merges, arbitrates shared files, runs the refresh handshake.
- @@LaneB: editor + search frontend.
- @@LaneC: Team Work + Survey (the round's biggest scope).
- @@LaneD: desktop + CLI (cs-shell, submit map, chan open, desktop shell).

## Waves + the refresh model

Work runs in 3 waves. Within a wave all four lanes work in parallel on
non-overlapping files. At a wave barrier:

1. Each lane drives its wave items to GATED-GREEN + locally merged, writes its
   journal, and pokes @@Architect "wave N done".
2. @@Architect verifies all four, sequences the merges to main (local), updates
   `round-3-status.md` to "wave N complete / wave N+1 active" with any carryover
   notes, then tells @@Host "refresh all into wave N+1".
3. @@Host REFRESHES every agent (restart + the 1-liner bootstrap
   "you are @@LaneX, read round-3-lane-x.md"). On restart an agent re-reads this
   file -> `round-3-status.md` (active wave) -> its lane doc's wave section ->
   resumes. A refresh is a clean-context reset: ALL durable state lives in the
   docs + on-disk task/journal/code, never in an agent's head.

## The gate (before any push or wave-complete claim)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`cargo build --no-default-features`, and in `web/`: svelte-check + `npm run
build`. The release gate additionally builds the gateway nested workspace. Do
not pipe the command whose exit you are verifying (set pipefail or capture $?).

## Browser-smoke the runtime-risky bits

Static gates miss Svelte-5 reactivity, CodeMirror/xterm timing, and the
preflight/boot gate. Anything touching editor decorations, survey overlay
reactivity, terminal key handling / agent submit, the preflight/boot path, or
live server->SPA pushes must be smoked on a running test server. Serve a
throwaway drive from a renamed binary copy and scope any `pkill` to your own
drive path / port so you do not kill another lane's server. Record anything you
could not empirically verify as "empirically-unverified" and tell @@Architect.

## File ownership + touch points

Each lane owns a DISJOINT set of files (its lane doc lists them). Do not edit a
file another lane owns. When your work needs a change in another lane's file,
cut a task into that lane's section plus a touch-point note and coordinate
through @@Architect at the wave barrier. The only cross-lane shared seams this
round are C<->D (survey transport) and A<->B (search API + graph); both are
arbitrated by @@Architect. Any shared-file commit collapses `git add <paths>` +
`git diff --staged --stat` audit + `git commit` into ONE chained invocation;
verify `git show --stat HEAD` after.

## Pokes (lean bus)

Context lives in on-disk task/journal/plan files. Pokes are 1-line POINTERS,
not fat context:

```
cs terminal write --tab-name=<target> $'poke from <me>: <1-line>; read <path>\x1b[27;9;13~'
```

The trailing `\x1b[27;9;13~` is the Meta+Enter submit chord (a bare `\n` / `\r`
parks the poke unsubmitted). `cs terminal` is the v0.21.0 name; if the installed
app is still v0.20.0 use `cs term`. Per-agent submit encodings (codex / gemini)
arrive in @@LaneD Wave-1; until then the chord is claude-correct and all lanes
run claude.

## No backwards compatibility

Fresh codebase, single user. Drop old shapes outright; no migration or
graceful-degrade paths. Never escalate a back-compat question.

## Round close

- Items not finished to a tested state append to a round-4 backlog, not the
  release.
- These coordination docs stay UNTRACKED during the round; @@Architect commits
  the whole `docs/journals/phase-15/` tree as one `docs(phase-15)` commit at
  close.
- Push / tag / release only on @@Host's explicit go. "Merge to main" means a
  local merge.
