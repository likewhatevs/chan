// Editor surface commands: available when a file tab is the active
// surface. Register entries with registerCommands; gate visibility with
// onSurface(ctx, "file"). See state/commands.ts for the Command shape and
// the dispatchChanCommand / allowedInWindow / onSurface helpers. Chorded
// reuse ids dispatch through the bridge; net-new actions call their
// exported action function directly.

import { registerCommands } from "../commands";

registerCommands([]);
