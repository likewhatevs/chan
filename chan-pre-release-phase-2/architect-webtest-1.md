# architect-webtest-1: Tear down + commit + recycle

Owner: @@Architect (briefing) → @@Webtest (executes). Status:
DONE.

## Context

Alex is recycling the current @@Webtest agent (Chrome extension
crashes on the codex runtime). Replacement is an Opus 4.7 agent
(2x slots, one of them dual-roling as @@Backend). Before recycle,
the current @@Webtest needs to land a clean tear-down so the next
agents inherit a stable starting point.

The architect's phase-2 commit (bundle of all REVIEW-approved
implementation work) is **separate** and is not part of this
tear-down. Keep the scope of this task narrow.

## Scope

Three things, in order:

### 1. Record final state in the webtest task files

Append a dated closing section to
[[chan-pre-release-phase-2/webtest-1.md]] and
[[chan-pre-release-phase-2/webtest-2.md]] with:

* Time the tear-down ran.
* Final PID of the chan-serve listener on 8788 (from `lsof -i :8788`)
  and confirmation it was stopped.
* What landed green in the current cycle (lift from the existing
  smoke results — do not re-run; this is a record, not a rerun).
* What is **deferred to the next @@Webtest cycle**: the three
  remaining probes in [[chan-pre-release-phase-2/architect-9.md]]
  (G1a ghost-while-open, G4 live-add-while-open, G3 depth-cap)
  plus the workaround restart for the report-finding fixture.
* The fact that the type-gap fixes (`web/src/state/store.svelte.ts`
  session payload + `web/src/components/GraphCanvas.svelte`
  language/folder/edge typings) you made during the rebuild stay
  uncommitted intentionally; the architect's phase commit will
  bundle them with frontend-8.

### 2. Stop the running test service

```
# from this checkout, NOT from /tmp/chan-dev
lsof -i :8788                        # confirm the PID matches webtest-1.md
kill -TERM <pid>                     # SIGTERM, not SIGKILL
lsof -i :8788                        # confirm the port is free
```

Do **not** touch `/tmp/chan-dev` content (no `rm`, no
`.chan/report.jsonl` deletion). The next @@Webtest cycle picks up
the workaround from [[chan-pre-release-phase-2/architect-9.md]]
and will run the restart itself.

### 3. Commit the webtest-authored phase-2 files

Stage **only** these paths:

```
chan-pre-release-phase-2/webtest-1.md
chan-pre-release-phase-2/webtest-2.md
chan-pre-release-phase-2/webtest-smoke.mjs
chan-pre-release-phase-2/architect-webtest-1.md
```

Do **not** stage:

* Any `web/` files. They roll into the architect phase commit.
* Any `crates/` files. Same.
* Any other `chan-pre-release-phase-2/*.md` files. Those are
  authored by other agents and aren't yours to land.

Commit message (HEREDOC, follow the repo style — short subject,
factual body, no em dashes, no assistant attribution):

```
git add chan-pre-release-phase-2/webtest-1.md \
        chan-pre-release-phase-2/webtest-2.md \
        chan-pre-release-phase-2/webtest-smoke.mjs \
        chan-pre-release-phase-2/architect-webtest-1.md
git commit -m "$(cat <<'EOF'
phase-2: webtest smoke runner + review notes

Add the phase 2 headless-Chrome smoke runner and the test-service
+ review notes that drove the search overlay, search row collapse,
search-status Graph-this, and language-graph browser probes against
the shared 8788 service.

Verification:
- node --check chan-pre-release-phase-2/webtest-smoke.mjs
- node chan-pre-release-phase-2/webtest-smoke.mjs

Deferred to next webtest cycle: ghost-while-open, live-add-while-open,
and depth-cap browser probes against the scratch fixture path assigned
in chan-pre-release-phase-2/architect-9.md.
EOF
)"
git status                            # verify clean stage
```

After the commit, confirm the commit hash is recorded:

```
git log --oneline -1
```

### 4. Final journal entry

Append to [[chan-pre-release-phase-2/journal.md]] under the `## Log`
section:

```
* 2026-05-16 @@Webtest: cycle close-out. Recorded final smoke
  state in webtest-1.md and webtest-2.md, stopped the 8788
  listener (was PID <pid>), and committed the phase-2 smoke
  runner + review notes (commit <hash>). Deferred to the next
  @@Webtest cycle: architect-9 probe matrix + report-finding
  workaround restart. Type-gap fixes in web/src/state/store.svelte.ts
  and web/src/components/GraphCanvas.svelte stay uncommitted for
  the architect's phase commit to bundle with frontend-8.
```

### 5. Signal idle / ready for recycle

After steps 1-4 are done, leave a short note at the end of this
file under a `## Tear-down log` heading with:

* Timestamp.
* Commit hash from step 3.
* Confirmation that 8788 is free and the working tree's only
  remaining staged-style diffs are the architect's bundle.
* "Ready for recycle."

That note is the green light Alex needs to bring the agent down.

## Acceptance criteria

1. webtest-1.md + webtest-2.md carry closing sections that match
   the actual handoff state.
2. `lsof -i :8788` returns no rows.
3. Exactly one new commit on the current branch (no force-push,
   no amend). `git log --oneline -1` shows the new SHA.
4. `git status` is clean of webtest-authored phase-2 directory
   files (only the architect's pending phase bundle remains).
5. journal.md has the recycle log entry.

## Guardrails

* **Do not** run `git push`. The user pushes the phase bundle
  after the phase commit lands.
* **Do not** run `git commit --no-verify`, `--amend`, or any
  reset. New commit only.
* **Do not** touch any other agent's task files (frontend-*,
  backend-*, rustacean-*, syseng-*, architect-1..9.md). They
  belong to their owners or the architect's bundle.
* **Do not** delete the smoke fixture under
  `/tmp/chan-dev/Scratch/phase2-smoke/` even if it's empty; the
  next cycle uses that path.
* **Do not** restart the service after tearing it down. Leave
  8788 free so the next @@Webtest agent can decide which fixture
  to use.

## Tear-down log

- 2026-05-16 16:41 BST: @@Webtest picked up the close-out.
- Stopped the final 8788 listener, PID 15857, with SIGTERM.
- Confirmed port 8788 is free.
- Recorded closing state in [[chan-pre-release-phase-2/webtest-1.md]] and
  [[chan-pre-release-phase-2/webtest-2.md]].
- Commit hash: pending at time of note; see final `git log --oneline -1`.
- Remaining uncommitted work is the architect phase bundle, including the
  frontend type-gap fixes noted in [[chan-pre-release-phase-2/webtest-2.md]].
- Ready for recycle.
