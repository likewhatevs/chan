# @@Webtest A task 3: test service for Alex's post-commit click-around

Owner: @@Webtest A
Status: DONE — service torn down at phase close per Alex.
Pairs with [systacean-7](./systacean-7.md) (Chan.app build + install).

## Goal

After the commits land, give Alex a running test service against
the same `chan-test-phase5` drive so he can click around in a
plain browser tab alongside the installed Chan.app, hunt for any
last bugs, and cross-check what he reads in
[summary.md](./summary.md).

## Pre-requisites

* HEAD has the Phase 5 cleanup + enhancement + bug-fix + terminal
  persistence commits landed locally.
* @@Webtest A's existing service from [webtest-1](./webtest-1.md)
  may still be running on the pre-commit binary (PID 59434 at
  last record); plan a clean restart so the running binary
  matches the committed source.

## Steps

1. **Stop the previous service.**

   `kill <pid>` if still running; verify `lsof -iTCP:8787 -sTCP:LISTEN`
   is empty before relaunch.

2. **Rebuild the debug binary against the committed HEAD.**

   ```
   npm --prefix web run build   # idempotent if web/dist is current
   cargo build -p chan
   ```

   This produces `target/debug/chan` with the post-Phase-5 web
   bundle embedded.

3. **Launch on the same drive Alex's been using.**

   ```
   ./target/debug/chan serve /tmp/chan-test-phase5 \
     --host 127.0.0.1 --port 8787 --no-browser
   ```

   Capture the per-launch bearer token from stderr (the token
   rotates on each launch). Run in background; log to
   `/tmp/chan-phase5-logs/server-post-commit.log`.

4. **Confirm baseline reachability.**

   * `curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:8787/api/health` → `{"status":"ok"}`.
   * `/api/build-info` reports the post-Phase-5 version + features.
   * `/api/index/status` reaches `state:idle` (the seeded
     `chan-test-phase5` drive is small; settle is sub-second).

5. **Post the URL + token in this task file** under
   `### Service` below so Alex (and Chan.app, which uses the
   embedded binary, not this debug service) can both be exercised
   side-by-side.

## Reporting

* URL, PID, token, drive path, log path, time started.
* If Alex flags a bug while clicking around, capture the repro
  here and route to the right owner via a new task file.
* Leave the service up until @@Architect signals phase close.

## Teardown

At phase close:
* Stop the service.
* Keep `/tmp/chan-test-phase5` since it's a long-lived registered
  drive Alex has been using all phase; @@Architect decides if
  it gets cleaned up.
* Record final state here.

## Service

| Service     | URL                                                         | PID    | Log                                       |
|-------------|-------------------------------------------------------------|--------|-------------------------------------------|
| chan-server | http://127.0.0.1:8787/?t=qag7t48iruaBs88YycrJ7etcikDeEcdi   | 26997  | /tmp/chan-phase5-logs/server-post-commit.log |

* Drive: `/private/tmp/chan-test-phase5` (the long-lived
  registered drive Alex has been clicking around all phase).
* Bearer token: `qag7t48iruaBs88YycrJ7etcikDeEcdi`.
* Command: `./target/debug/chan serve /tmp/chan-test-phase5
  --host 127.0.0.1 --port 8787 --no-browser`.
* Bind: 127.0.0.1 loopback only, no tunnel.
* Launched: 2026-05-17 ~22:30 BST.

### Baseline reachability

* `GET /api/health` → 200.
* `GET /api/build-info` → `{"version":"0.8.1","features":{"embeddings":true}}`.
* `GET /api/index/status` → `{"state":"idle","indexed_docs":95,"indexed_vectors":95,"model":"BAAI/bge-small-en-v1.5"}` (the cumulative drive content from rounds 1-10 smoke).

### HEAD at launch

```
bccdb18 release: phase 5 final journal + summary
7da49f6 release: close phase 5 tasks
9ecb27d docs: refresh phase-5 boundary
790fd02 web: phase-5 frontend (overlay removal + persistent terminals + ux)
9e121d5 chan-server: prune agent surface + persistent terminal sessions
58fe80a chan-drive: drop assistant blobs + vcs-aware indexing
c748484 chan-llm: pare to MCP-only surface
02be09c web: add terminal tab controls
455c5df web: move terminal into workspace tabs
9c1ea91 web: add terminal overlay
```

(`origin/main..HEAD` shows 10 ahead; push held for Alex's
explicit go per the journal round-19 entry.)

## Progress

* 2026-05-17 ~22:30 BST: rebuilt `web/dist/` and the chan binary
  against committed `bccdb18`, killed PID 18015 (round-10 debug
  service), launched fresh on the same drive. Service alive,
  health + build-info + index status all green. URL + token
  posted above; Alex can click around alongside the running
  [systacean-7](./systacean-7.md) Chan.app build (still in
  flight per the journal).
* No bugs surfaced from this lane between launch and this
  entry; will append any repros Alex flags during the click-
  around to the progress section below as they come in.

## Completion notes

* 2026-05-17 ~22:40 BST: Alex called "done". Killed PID 26997
  (`SIGTERM` clean shutdown, no orphan processes — `pgrep -f
  'target/debug/chan serve'` returns nothing).
* Fixture drive `/private/tmp/chan-test-phase5` left in place
  per the task spec ("long-lived registered drive Alex has been
  using all phase; @@Architect decides if it gets cleaned up").
  Current contents: the markdown + .txt files seeded across
  rounds 1-10 (`welcome.md`, `notes/`, `projects/phase5/`,
  `longfile.md`, `confcheck-*`, `r4-create.*`, `deep/nested/`),
  plus the `.git/` directory from the systacean-4 git/hg smoke.
* Log directory `/tmp/chan-phase5-logs/` left in place with 31
  files (`server-*.log`, baseline check/test logs, fmt/clippy/
  build retry logs, etc.). Useful as evidence if any of the
  acceptance findings need replaying; cleanup-safe per the
  teardown brief.
* Service ports 5173 + 8787 are unbound. No webtest-owned
  background processes remain.
* My lane's tracking: webtest-1 round-1 through round-10 all
  PASS, webtest-3 service spun up + torn down. Coordination
  with @@Webtest B: clean (their webtest-2 closed REVIEW
  earlier).
* Phase-5 acceptance from this lane: green. Final summary lives
  in [summary.md](./summary.md).
