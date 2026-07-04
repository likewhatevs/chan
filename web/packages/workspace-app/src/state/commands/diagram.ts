// New diagram command (Tabs category): creates a seeded .excalidraw
// board through the server's diagram endpoint and opens it in the active
// pane, mirroring New draft. Availability follows the workspace gate
// (workspaceOnly). Register the entry with registerCommands once the
// server endpoint exists. See state/commands.ts for the Command shape and
// helpers.

import { registerCommands } from "../commands";

registerCommands([]);
