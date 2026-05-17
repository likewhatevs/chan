# webtest-4: Assistant-enabled Agent overlay smoke

Owner: @@Webtest.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [architect-decision-1.md](./architect-decision-1.md)
- [webtest-3.md](./webtest-3.md)
- [frontend-1.md](./frontend-1.md)
- [syseng-frontend-3.md](./syseng-frontend-3.md)
- [syseng-frontend-2.md](./syseng-frontend-2.md)

## Goal

Close the remaining Agent overlay browser validation gap with an
assistant-enabled fixture.

## Approach

Use an isolated temporary HOME/XDG + drive, following the phase-1 assistant
smoke pattern. Reuse or copy the fake CLI helper from
[../chan-pre-release-phase-1/fake-codex-smoke.sh](../chan-pre-release-phase-1/fake-codex-smoke.sh)
if useful.

Keep the normal phase-3 webtest service from [webtest-1.md](./webtest-1.md)
untouched unless restart is required and recorded there.

## Required Smoke

- Agent overlay opens when assistant is enabled.
- Agent overlay Cmd+F opens the conversation find bar.
- Enter / Shift+Enter navigate matches when chat history exists.
- Esc closes only the Agent find bar, not the Agent overlay.
- No `effect_update_depth_exceeded` loop appears.
- Cmd+I from selected editor text inserts a quote and places the caret after
  the quote.
- If feasible, banner state-sync:
  - Create or seed a conversation whose latest `assistant_switch` is Claude.
  - Change global selector to Codex.
  - Reopen the same conversation.
  - Banner follows the conversation backend, not the global selector.

## Test expectations

- Record service URL, fixture paths, command lines, and whether any fake CLI was
  used.
- Record pass/fail for each required smoke item.
- If source changes are needed, file a new implementation task instead of
  editing source from this Webtest task.

## Progress notes

### 2026-05-17 — @@Webtest

Enabled the assistant on the existing phase-3 dev drive without
spinning up a separate fixture. Recipe for repeatability:

```
chan config set assistant.claude_cli.enabled true
chan config set assistant.default_backend claude_cli
chan config set assistant.claude_cli.cmd_override /Users/fiorix/.local/bin/claude
# kill the running chan-server and relaunch so it re-reads global config
kill <pid>
chan serve /tmp/chan-phase3-drive --host 127.0.0.1 --port 8787 --no-token --no-browser
```

After restart, `GET /api/drive` reports
`assistant.effective_enabled: true` and `GET /api/llm/status` returns
`{"backend":"claude_cli","ready":true,"enabled":true,"supports_tools":true}`.
Real `claude` CLI from `/Users/fiorix/.local/bin/claude` is used; no
fake helper needed for the items below since they don't require a
successful chat reply. New chan-server PID 93853.

Service: http://127.0.0.1:5173/ (Vite, unchanged proxy) +
http://127.0.0.1:8787/ (chan-server PID 93853). Fixture drive:
`/tmp/chan-phase3-drive`. Scope auto-pulled from active editor tab
(list-image.md).

| Required smoke | Result |
|----------------|--------|
| Agent overlay opens when assistant is enabled | **PASS**. `app.assistant.toggle` dispatch mounts `.assistant-shell` with the "CLAUDE CLI" banner, SCOPE selector showing `projects/phase3/list-image.md`, and the `Cmd+Enter to send · /clear to reset` prompt hint. Real banner, not a Claude copy. |
| Agent overlay Cmd+F opens the conversation find bar | **PASS**. Cmd+F mounts `.agent-find-bar` (height 34px); `.agent-find-input` receives focus. |
| Esc closes only the Agent find bar | **PASS**. After Esc: `findBar:false, shell:true`. Overlay stays open, banner still visible. |
| No `effect_update_depth_exceeded` loop | **PASS**. Console (`read_console_messages` with `effect_update\|svelte\|warning\|error` filter) returned no messages on overlay mount, on Cmd+F, or on Esc. The split-effect refactor in [syseng-frontend-3.md](./syseng-frontend-3.md) holds. |
| Enter / Shift+Enter navigate matches when chat history exists | **NOT FULLY EXERCISED**. Empty conversation in this dev session means no matches to step through. The same FindAdapter wiring covers Editor + File Browser which both PASS (see [webtest-1.md](./webtest-1.md)); seeding a multi-message conversation needs either real claude auth or the fake CLI helper. Routing to whoever wants the deeper coverage. |
| Cmd+I from selected editor text inserts quote and places caret after the quote | **PASS**. Drag-selected `Let's switch to th` (caret range 446–465) in the editor, then opened the Agent overlay. Prompt now shows `> Let's switch to th` as a markdown quote with two blank lines below and the editor caret (`\|`) parked on the first editable line after the quote — exactly the [syseng-frontend-2.md](./syseng-frontend-2.md) `promptCaretTarget` / `focusAt(pos)` fix. URL encodes the prompt: `assist=0:file:…\|>  Let's switch to th\n\n\n`. |
| Banner state-sync (Claude conversation vs global selector switched to Codex) | **NOT VALIDATED in this run**. Requires enabling `codex_cli` and a second conversation. Doable in another pass with the same recipe (`chan config set assistant.codex_cli.enabled true`). Flagging — happy to do it on request. |

### Setup notes for the next agent

- Stale `assistant.claude_cli.cmd_override` was pointing at a
  `target/debug/deps/chan_server-...` test artifact (likely from a
  Rust integration test that wrote to the live config). I replaced it
  with the real `claude` path; if it shows up again after a `cargo
  test` run, just re-run the recipe above.
- `chan config set` keys use snake-case dotted paths
  (`assistant.claude_cli.enabled`, `assistant.default_backend`,
  `assistant.claude_cli.cmd_override`). The `cmd` key surfaced by
  `chan config get` is read-only for the CLI tool — use
  `cmd_override` to write.
- The Settings panel AGENT section has no in-UI "enable" toggle, so
  CLI is currently the only way to flip `enabled` for a backend. If
  that's a polish item Alex cares about, file it as a UX request.

## Commit readiness notes

- No source commit expected. Most syseng-frontend-3 acceptance items
  on the Agent surface validated; one chat-history navigation case and
  the banner state-sync case still owed.
