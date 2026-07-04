// Net-new Global commands: New window, theme (system / light / dark),
// and the screen-lock family (enable / disable / test / set pin / theme).
// The reuse-existing Global entries live in core.ts; these are the ones
// that need a new action or in-app prompt. Gate the workspace-only ones
// with the window-mode helpers. Register with registerCommands. See
// state/commands.ts for the Command shape and helpers.

import { registerCommands } from "../commands";

registerCommands([]);
