# frontend-1: Agent overlay removal pass

## Merge plan check

- `../chan-term` and this checkout already point at the same commit:
  `963bade web: add terminal tab controls`.
- `../chan-term` is detached at that commit, but has no diff against
  local `main`.
- Local `main` is ahead of `origin/main` by the same three terminal
  commits, so there is no additional chan-term merge work for the
  frontend before cleanup.

## Frontend cleanup done

- Removed Agent overlay and Agent history overlay mounts from
  `web/src/App.svelte`.
- Removed Agent keyboard routing from browser and desktop command
  bridges.
- Removed Agent from the central shortcut registry and regenerated the
  `chan serve` keybinding text.
- Removed Agent entry points from file-tab and empty-pane menus.
- Removed the Agent section from Settings.
- Deleted the Agent overlay component files that were no longer mounted.

## Preserved / not owned in this pass

- MCP and chan-llm/backend surfaces are intentionally left for the
  backend cleanup owner; request.md explicitly calls out preserving the
  MCP server and agent access to the DRIVE.
- The web store still contains assistant/LLM state and tests because
  backend removal needs to decide which persisted-history and MCP-facing
  APIs survive.

## Verification

- `npm run check` in `web/`: clean.
- `npm run test` in `web/`: 14 files, 168 tests passed.
