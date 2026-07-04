// Net-new Panes commands: swap left/right/up/down and the focus-border
// colour picks. The reuse-existing Panes entries (splits, prev/next,
// close, flip) live in core.ts; these wrap existing actions
// (paneModeSwapWith, setWindowFocusColor) the runCommand switch does not
// already expose. Register with registerCommands. See state/commands.ts
// for the Command shape and helpers.

import { registerCommands } from "../commands";

registerCommands([]);
