# Phase 9 Rich Prompt Report

Date: 2026-05-24
Owners: Core Architect, Web Architect
Status: Integrated slice complete

## Summary

Phase 9 Rich Prompt is integrated across Core and Web. The feature now creates
server-owned Rich Prompt workspaces, wires watcher-backed status into the
terminal UI, archives submitted prompts through Core, and tears down workspace
state from terminal close.

The Web lane is complete from implementation and validation perspective. The
remaining follow-up is Core classification of path-less watcher rebuild
warnings observed during browser validation.

## Implemented

- Core Rich Prompt routes are wired:
  - `POST /api/rich-prompts`
  - `GET /api/rich-prompts/:name/status?session=<id>`
  - `POST /api/rich-prompts/:name/submit`
  - `POST /api/rich-prompts/:name/close`
- Web API client and types cover the Core contract.
- Cmd+P semantic helper now always creates a fresh terminal with Rich Prompt
  open. Browser/iab validation should use Cmd+Alt+P or Mod+. p; native can
  validate literal Cmd+P.
- Terminal Rich Prompt state persists workspace identity, phase, paths,
  submission sequence, submit mode, and agent target.
- Submit still sends to the terminal, then archives through Core and clears
  the prompt only if the user has not edited it while the archive request was
  in flight.
- Terminal close owns Rich Prompt teardown. If Core close fails or returns a
  broken phase, Web keeps the terminal visible.
- Prompt header now has plus actions, event count, agent picker, Send, and
  collapse. Prompt-local Close is removed.
- Plus menu now has Spawn agent, Spawn agents, Copy metadata dir, Copy Spawn
  agents config, Collapse/Expand, Bubble stack, and Bubble tray.
- Prompt-local New File and manual Watch/Stop watcher actions are removed.
- Spawn agents dialog supports min 1 and max 9, config copy/paste, and
  preflight confirmation before identity prompts are staged.

## Verification

- `npm run test -- --run src/components/TerminalRichPrompt.test.ts
  src/components/newTeamButton.test.ts
  src/components/teamBootstrapOrchestrator.test.ts src/state/tabs.test.ts
  src/components/toastAutoDismissSweep.test.ts
  src/components/PathPromptModal.test.ts` passed: 6 files, 181 tests.
- `npm run check` passed with 0 errors and 0 warnings.
- `npm run build` passed with existing Vite bundle-size and ineffective dynamic
  import warnings.
- `cargo build -p chan` passed. Core-side dead-code warnings remain in
  `crates/chan-drive/src/fd_budget.rs`.
- `git diff --check` passed.
- Live HTTP route walk against embedded `./target/debug/chan serve --no-browser`
  passed with isolated `HOME=/tmp/chan-rich-prompt-home`.
- In-app browser/iab validation passed for app load, prompt header, plus menu,
  removed menu items, agent picker persistence, and terminal-close teardown.

## Validation Notes

- Literal Cmd+P did not trigger in iab. This matches the current browser
  shortcut contract: web macOS uses Cmd+Alt+P, and all platforms support
  Mod+. p through Hybrid Nav. The staged Mod+. p path passed from editor and
  terminal.
- iab could not type or clipboard non-empty content into CodeMirror, so
  non-empty submit archive/clear and full Spawn agents preflight were only
  partially validated visually. Unit coverage and HTTP route walk cover the
  logic.
- Server stderr during visual validation showed repeated Core watcher warnings:
  `watcher event stream lost scope; requesting rebuild` for path-less event
  and path-less rename during Rich Prompt workspace activity.

## Follow-Up

Core should inspect and classify the path-less watcher rebuild warnings. See:

- `docs/journals/phase-9/architect/rich-prompt-core-follow-up-task.md`
