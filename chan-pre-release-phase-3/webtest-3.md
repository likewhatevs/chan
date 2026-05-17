# webtest-3: Remaining browser validation pass

Owner: @@Webtest.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [webtest-1.md](./webtest-1.md)
- [webtest-2.md](./webtest-2.md)
- [frontend-1.md](./frontend-1.md)
- [syseng-frontend-3.md](./syseng-frontend-3.md)
- [syseng-frontend-4.md](./syseng-frontend-4.md)

## Goal

Close the remaining browser validation gaps after the latest REVIEW tasks.

## Required Smoke

- Settings / Layout frontend wiring from [syseng-frontend-4.md](./syseng-frontend-4.md):
  - Settings shows `Standard` / `Compact`, not `Tight`.
  - Default selection is `Standard`.
  - Selecting `Compact` persists and visibly tightens editor line height.
  - Reload preserves the selected value.
- Agent overlay find behavior from [syseng-frontend-3.md](./syseng-frontend-3.md):
  - Agent overlay Cmd+F opens the Agent find bar.
  - Enter / Shift+Enter navigate matches when chat history exists.
  - Esc closes only the Agent find bar, not the Agent overlay.
  - No `effect_update_depth_exceeded` loop appears.
- Agent banner state-sync from [frontend-1.md](./frontend-1.md), if an
  assistant-enabled fixture/backend is available:
  - Reopen a Claude conversation after switching selector to Codex.
  - Banner should follow the conversation's latest `assistant_switch`, not the
    global selector.
- Narrow viewport pass for the phase-3 surfaces already smoked on desktop.

## Notes

[webtest-1.md](./webtest-1.md) validated File Browser find next/previous and
Esc behavior after [syseng-frontend-3.md](./syseng-frontend-3.md), but could not
validate Agent overlay behavior because the phase-3 fixture has no configured
LLM backend.

If an assistant-enabled fixture is needed, record the setup here and keep
service ownership in Webtest. Do not modify source files.

## Progress notes

### 2026-05-17 — @@Webtest pass

Re-baseline: `npm run check` 0/0 (3918), `npm test -- --run` 14/168 pass.

- **Settings / Layout (syseng-frontend-4) — PASS.** Shows Standard /
  Compact, not Tight. Default Standard. Compact tightens `.cm-line` from
  28.8px (standard) to 26.4px (compact). Reload preserves Compact.
  Legacy `tight` config via `chan config set editor.line_spacing tight`
  reloads as Compact in the UI (26.4px). Restored to Standard.
- **Narrow viewport (414x800) — PASS** for editor (numbered-list hang
  indent holds, image fits the column, status bar visible), File
  Browser overlay (tree + DETAILS scaled into narrow), File Browser
  Cmd+F (counter, ▲/▼ controls, BINARY-blue DETAILS pill all fit),
  and Esc staged close (find bar first, overlay second).
- **Agent overlay (syseng-frontend-3) and Agent banner state-sync
  (frontend-1) — NOT VALIDATED.** Phase-3 fixture has no enabled LLM
  backend; `drive.info.preferences.assistant.effective_enabled` stays
  `false`, so `app.assistant.toggle` is a no-op and `.assistant-shell`
  never mounts. Setup notes for whoever enables it:
  - `claude`, `gemini`, `codex` are all on PATH; the dev drive's
    `claude_cli.cmd_override` points at a stale dev-test artifact
    (`target/debug/deps/chan_server-...`) and `claude_cli.enabled` is
    `false`.
  - Settings panel AGENT section has no visible "Enable" toggle —
    only Agent CLI dropdown + cmd_override input — so dropdown
    selection alone does not flip `enabled` to `true`.
  - `PATCH /api/drive` with the obvious
    `{"preferences":{"assistant":{...}}}` payload returns 200 but
    leaves prefs unchanged (likely wrong PATCH shape; SettingsPanel
    actually uses `/api/config` for global prefs per
    [SettingsPanel.svelte:225](../web/src/components/SettingsPanel.svelte)).
  - Once `effective_enabled=true`, the syseng-frontend-3 Agent items
    and frontend-1 banner state-sync become testable.

Service ownership stays with @@Webtest; happy to dig into the
prefs API shape if Alex wants Agent-overlay coverage from this lane,
otherwise routing to @@WebtestB.

## Commit readiness notes

- No source commit expected. REVIEW with one unresolved validation decision:
  Agent overlay smoke requires an assistant-enabled fixture.
