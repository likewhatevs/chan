# @@Webtest A task 3: test service for Alex's post-commit click-around

Owner: @@Webtest A
Status: BLOCKED — fires after [architect-2](./architect-2.md) lands
the wave-1 + wave-2 commits. Pairs with
[systacean-7](./systacean-7.md) (Chan.app build + install).

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

(populated by @@Webtest A on launch)

## Progress

(populated by @@Webtest A once the commit gate opens)

## Completion notes

(populated by @@Webtest A at task close)
