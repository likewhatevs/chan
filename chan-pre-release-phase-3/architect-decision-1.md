# architect-decision-1: Agent-overlay smoke coverage decision

Owner: @@Architect.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [webtest-3.md](./webtest-3.md)
- [frontend-1.md](./frontend-1.md)
- [syseng-frontend-3.md](./syseng-frontend-3.md)

## Decision Needed

[webtest-3.md](./webtest-3.md) could not validate Agent overlay Cmd+F / Esc
behavior or banner state-sync because the phase-3 fixture has no enabled LLM
backend and `.assistant-shell` never mounts.

Decide whether to:

1. Spend time enabling an assistant/agent fixture and run the missing browser
   smoke, or
2. Record this as a known validation gap in [summary.md](./summary.md), relying
   on type/tests plus code review for this phase.

## Context

Webtest notes:

- CLI binaries are on PATH.
- The dev drive has stale `claude_cli.cmd_override`.
- `claude_cli.enabled` is false.
- Settings UI has no obvious enable toggle.
- A guessed `PATCH /api/drive` payload returned 200 but did not change prefs.
- SettingsPanel appears to use `/api/config` for global preferences.

## Progress notes

- 2026-05-17 @@Architect: chose option 1, bounded validation attempt. Prior phase
  already has a fake Codex helper, so the remaining Agent overlay smoke should
  be attempted with an isolated assistant-enabled fixture before accepting a
  validation gap.

## Outcome

- Created [webtest-4.md](./webtest-4.md). If Webtest cannot enable the fixture
  with reasonable effort, record the blocker there and carry the remaining
  Agent overlay browser coverage as a known gap in [summary.md](./summary.md).
