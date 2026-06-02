# Phase-16 round-1 bootstrap

You were woken with `read ./docs/journals/phase-16/round-1-bootstrap.md`.
Do these steps in order. Do not skip step 1.

## 1. Identify yourself

Run `echo $CHAN_TAB_NAME` and map it to your role:

```
$CHAN_TAB_NAME | role                         | then read
---------------+------------------------------+---------------------------------
@@Lead         | dedicated architect / lead   | lead.md  +  round-1-plan.md
@@LaneA        | CLI/terminal + lead tooling  | lane-a.md
@@LaneB        | Graph                        | lane-b.md
@@LaneC        | Pre-flight + Desktop         | lane-c.md
@@LaneD        | Frontend/UX + Team Work GUI  | lane-d.md
@@LaneE        | Docs + Build/CI + bootstrap  | lane-e.md
```

All paths are under `docs/journals/phase-16/`. If `$CHAN_TAB_NAME` is empty
or not in this table: STOP and ask @@Host. Do not guess a lane.

## 2. Read your files

**RESUME CHECK (do this first):** open `round-1-status.md`. If it already
shows MERGED slices, the round is underway and you are RESUMING after a team
rebuild/respawn - NOT starting fresh. In that case:
- Read `round-1-status.md` (what's done) + `round-1-wave-3.md` (your CURRENT
  task) and continue from there. Your lane file below is your ORIGINAL wave-1
  scope, for reference only - do NOT re-execute already-merged work.
- Re-read your own `event-lane-<x>.md` tail for in-flight context. @@Lead:
  also re-read `lead.md`, every `event-lane-*.md` tail, and check whether the
  rebuild left a TODO (e.g. greenlight the next wave). Re-arm your watcher.
If `round-1-status.md` shows no merged slices, this is a clean start - proceed
normally below.

Read `round-1-plan.md` (shared: scope, waves, gates, cross-lane coupling)
AND your own lane file. @@Lead also reads `lead.md`. Your lane file lists
the exact tasks, the files you OWN, and your verification steps.

## 3. Process rules (everyone)

- Shared working tree. Edit ONLY the files your lane owns. A change in
  another lane's files needs a poke-coordinated handoff first.
- Commit with PATHSPEC ONLY: `git commit -F <msgfile> -- <explicit paths>`
  (flags before `--`). Run `git show --stat HEAD` after to confirm the
  split. Never `git add -A` / `git add .` in this tree.
- Do NOT push. @@Lead gates and merges; pushes happen only on @@Host's
  explicit ask.
- Green means the full gate ran: `make pre-push` (fmt + clippy -D warnings
  + test + svelte-check + npm build). Don't report green from a partial or
  piped command (pipes mask exit codes).
- Pokes are ONE LINE pointers: `poke from @@X: read docs/journals/phase-16/
  <file>`. Context lives in files, not in pokes.
- Questions/decisions for @@Host: do NOT run an in-terminal survey and do
  NOT AskUserQuestion @@Host directly. Write your question into your
  `event-lane-<x>.md`, poke @@Lead. @@Lead consolidates, runs `cs terminal
  survey` to @@Host, and dispatches answers back as task files.
- Append-only: once you have started a task, a new ask is a NEW task, not a
  rewrite of the running one.
- Verify before asserting (git sha / curl / atomic read), especially after
  sub-agent edits or truncated tool output. Anchor on HEAD, not memory.

## 4. Start

Begin your lane's round-1 scope (see your lane file). When your first slice
is gated-green and ready to merge, write it to your `event-lane-<x>.md` and
poke @@Lead with the one-line pointer.
