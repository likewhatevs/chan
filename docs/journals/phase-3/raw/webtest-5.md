# webtest-5: Final service teardown

Owner: @@Webtest.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [webtest-1.md](./webtest-1.md)
- [webtest-4.md](./webtest-4.md)
- [architect-verify-1.md](./architect-verify-1.md)

## Goal

Tear down or explicitly preserve the phase-3 browser test service before
@@Architect writes the final phase summary.

## Current known service state

- Vite frontend: `http://127.0.0.1:5173/`.
- chan backend: `http://127.0.0.1:8787/`.
- Fixture drive: `/tmp/chan-phase3-drive`.
- Last recorded backend PID in [webtest-4.md](./webtest-4.md): `93853`.

## Acceptance criteria

- Stop any webtest-owned dev server, backend server, browser automation, or
  watcher that should not remain after the phase.
- If Alex should keep a service running for inspection, record that explicitly
  with the PID, URL, and reason instead of stopping it.
- Remove cleanup-safe scratch logs/temp outputs created by Webtest, or record
  why they are intentionally preserved.
- Do not remove source changes, task files, or the phase fixture unless Alex has
  explicitly asked for that cleanup.
- Update this task to REVIEW with final service state.

## Test expectations

- Record the commands used to inspect or stop processes.
- Confirm the final state of ports `5173` and `8787`.

## Progress notes

### 2026-05-17 — @@Webtest teardown

Stopped both services and reverted the assistant config side effects
from [webtest-4.md](./webtest-4.md).

```
pgrep -fl 'chan serve /tmp/chan-phase3|vite'
  # showed PID 40674 (vite) and PID 93853 (chan-server)
kill 93853
kill 40674
sleep 2
pgrep -fl 'chan serve /tmp/chan-phase3|vite'   # (no output)
lsof -nP -iTCP:8787 -sTCP:LISTEN                # (no output)
lsof -nP -iTCP:5173 -sTCP:LISTEN                # (no output)
```

Ports 5173 and 8787 are free.

Reverted global `chan config` side effects from webtest-4:

```
chan config set assistant.claude_cli.enabled false
chan config set assistant.default_backend none
```

`chan config get` no longer surfaces `enabled` or `default_backend`,
matching the pre-phase state.

Note on `cmd_override`: I could not set it to empty via CLI ("value
must not be empty"); `chan config get` now shows a fresh
`assistant.claude_cli.cmd` pointing at
`target/debug/deps/chan_server-f0efb6858a6fc3be`. That's a `cargo
test` artifact (likely from the `architect-verify-1` pre-push run)
overwriting whatever I had set. Same pollution mechanism that
produced the stale `cmd_override` I started the phase with; the right
fix is in the test setup, not in Webtest teardown — flagging as a
tiny follow-up.

Removed scratch logs:

```
ls /tmp/chan-phase3-logs/   # 13 files: baseline-*, check-after-*, test-after-*, server.log, vite.log
rm -rf /tmp/chan-phase3-logs
```

Preserved:

- `/tmp/chan-phase3-drive/` — phase fixture (per task: do not remove
  unless explicitly asked). Contents kept: README, contacts, inbox,
  media, notes, projects (incl. the `indent-bug.md` "whiLevelch"
  typo from the documented BUG-FE2-B web-build keystroke leak, and
  the re-seeded `list-image.md` + 200x150 `media/images/big.png`).
- All task files in this directory and any source-tree changes from
  other agents (none from me).
- Chrome tab(s) opened by claude-in-chrome — user browser state, not
  webtest-owned background processes.

## Completion notes

- Final port state: 5173 and 8787 unbound.
- No webtest-owned background processes remain.
- Phase fixture preserved; scratch logs removed.
- Two small follow-ups for Alex if of interest:
  - chan-server `cargo test --all-targets` writes into the live user
    config dir's `assistant.claude_cli.cmd`. Pre-existing dev-env
    quirk, not phase-3 work; flagging.
  - Settings panel AGENT section has no in-UI enable toggle for the
    assistant backend; CLI is the only path today to flip
    `assistant.claude_cli.enabled`. Possible UX polish item.
