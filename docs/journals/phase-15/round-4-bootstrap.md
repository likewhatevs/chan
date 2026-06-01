# Phase-15 round-4 bootstrap (shared process)

Read order for every lane (INCLUDING @@LaneA / @@Architect): this file ->
`round-4-status.md` (the active wave + live cross-lane notes) -> your
`round-4-lane-<x>.md` -> `round-4-plan.md` (the technical source of truth:
grounded scopes with file:line anchors).

## Goal

Deliver the round-4 backlog (`round-4-backlog.md`, with @@Host's decisions
baked in) as release v0.23.0, tested and validated. Two waves, four agents,
all agents recycled (refreshed) at the wave barrier.

## Roster

- @@Host (Alex): product/scope calls; the only one who pushes, tags, cuts the
  release, AND recycles the agents. Talks to @@Architect, not the workers.
- @@LaneA = @@Architect: hub / gate / coordinator + the release cut. The only
  agent that talks to @@Host. Sets the wave, sequences merges, arbitrates the
  (minimal) shared seam, runs the refresh handshake. Coding-light: the 2
  carryover editor browser-smokes + lends subagents to @@LaneB.
- @@LaneB: Linux build tooling (the long pole; chan-desktop + gateway from
  macOS via sdme, AppImage, cs verify). MAY spawn subagents per distro.
- @@LaneC: `cs terminal team` CLI (new | load + `--script`).
- @@LaneD: semantic-search wiring + phase-8 docs cleanup (two smalls).

## Waves + the recycle model

Work runs in 2 waves (the architect may add a 3rd if build tooling slips).
Within a wave all four agents work in parallel on non-overlapping files. At a
wave barrier:

1. Each lane drives its wave items to GATED-GREEN + locally merged, writes its
   journal, and pokes @@Architect "wave N done".
2. @@Architect verifies all four, sequences the merges to main (local),
   updates `round-4-status.md` to "wave N complete / wave N+1 active" with any
   carryover notes, then tells @@Host "refresh all into wave N+1".
3. @@Host RECYCLES every agent (restart + the 1-liner bootstrap
   "you are @@LaneX, read round-4-lane-x.md"). On restart an agent re-reads
   this file -> `round-4-status.md` (active wave) -> its lane doc's wave
   section -> resumes. A refresh is a clean-context reset: ALL durable state
   lives in the docs + on-disk code/journal, never in an agent's head.

## The gate (before any push or wave-complete claim)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo
test`, `cargo build --no-default-features`, and in `web/`: svelte-check +
`npm run build`. The release gate additionally builds the gateway nested
workspace. Do not pipe the command whose exit you are verifying (set pipefail
or capture $?).

GATED-PUSH SIGPIPE (round-3 incident, standing): `git push` runs the pre-push
hook = the FULL `make pre-push` (~3 min, ~90KB output) on EVERY push including
tags. A BACKGROUNDED gated push SIGPIPEs (exit 141) when the hook log overruns
the capture pipe, and silently fails to update the remote while sometimes
reporting exit 0. Always push in the FOREGROUND with output redirected to a
file (`git push origin main > /tmp/push.log 2>&1`) and verify the remote with
`git ls-remote origin refs/heads/main` (and `--tags`) BEFORE the next step.
`--no-verify` is classifier-blocked, so budget the hook re-running on the tag
push too.

## Browser-smoke the runtime-risky bits

Static gates miss Svelte-5 reactivity, CodeMirror/xterm timing, and the
preflight/boot gate. The carryover editor smokes (click-caret, [[ stuck
bubble) need a running server + browser `navigate` (denied to all lanes in
round-3; rides on @@Host re-allowing it). Serve a throwaway drive from a
renamed binary copy and scope any `pkill` to your own drive path / port.
Record anything you could not empirically verify as "empirically-unverified"
and tell @@Architect.

## File ownership + touch points

Each lane owns a DISJOINT set of files (its lane doc lists them). Round-4
coupling is LOW: build=infra (Makefiles/CI/sdme), cs-team=chan-shell +
control_socket/team_config, semantic=routes/search.rs + main.rs cmd_search,
docs=docs/. The ONE cross-lane seam is B<->A: @@LaneB edits
`.github/workflows/release.yml` (the multi-distro matrix); the architect's
release cut USES release.yml, so B's release.yml change lands + gates BEFORE
@@LaneA cuts v0.23.0. Do not edit a file another lane owns. Any shared-file
commit collapses `git add <paths>` + `git diff --staged --stat` audit + `git
commit` into ONE chained invocation; verify `git show --stat HEAD` after. The
only race-proof commit in the shared worktree is `git commit -F msg --
<explicit paths>` (pathspec; flags BEFORE `--`).

## Pokes (lean bus)

Context lives in on-disk task/journal/plan files. Pokes are 1-line POINTERS,
not fat context:

```
cs terminal write --tab-name=<target> $'poke from <me>: <1-line>; read <path>\x1b[27;9;13~'
```

The trailing `\x1b[27;9;13~` is the Meta+Enter submit chord (a bare `\n` / `\r`
parks the poke unsubmitted). `cs terminal` is the v0.21.0+ name. Per-agent
submit encodings: claude=`\x1b[27;9;13~`, codex/gemini=`\r`; all lanes run
claude unless a lane doc says otherwise.

## No backwards compatibility

Fresh codebase, single user. Drop old shapes outright; no migration or
graceful-degrade paths. Never escalate a back-compat question. (The graph DB
already proved this in round-3: schema changed at the v1 CREATE, fresh
`~/.chan` adopts it.)

## Round close

- Items not finished to a tested state append to a round-5 (or phase-16)
  backlog, not the release. Metal is already a phase-16 item.
- These coordination docs stay UNTRACKED during the round; @@Architect commits
  the whole `docs/journals/phase-15/` tree as one `docs(phase-15)` commit at
  close.
- Push / tag / release only on @@Host's explicit go. "Merge to main" means a
  local merge. Round-4 ships as **v0.23.0**.
