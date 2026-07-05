// Terminal surface commands: available when a terminal tab is the active
// surface, excluding the control-terminal singleton whose chrome and
// lifecycle belong to the connect flow. Theme, group broadcast, and set
// name / group act on the active terminal tab directly; the live-PTY
// actions dispatch a chan:command that the owning app/TerminalTab path
// handles, since live CWD, session lifecycle, and Rich Prompt visibility
// live outside this catalog. Register with registerCommands. See
// state/commands.ts for the Command shape and helpers.

import {
  registerCommands,
  dispatchChanCommand,
  onSurface,
  type CommandContext,
} from "../commands";
import { setHybridSurfaceTheme, setTransientStatus, uiPrompt } from "../store.svelte";
import {
  activeTerminalTab,
  renameTerminalTab,
  terminalTabName,
  setTerminalGroup,
} from "../tabs.svelte";

function onTerminal(ctx: CommandContext): boolean {
  return onSurface(ctx, "terminal") && !ctx.terminalControl;
}

function onWorkspaceTerminal(ctx: CommandContext): boolean {
  return onTerminal(ctx) && !ctx.terminalOnly;
}

async function renameActiveTerminal(): Promise<void> {
  const t = activeTerminalTab();
  if (!t) return;
  const name = await uiPrompt("Terminal name", terminalTabName(t));
  if (name !== null) renameTerminalTab(t, name);
}

/// Set the active terminal's group. The group reaches the shell only on
/// the next spawn, so flag that the change applies after a restart.
async function setActiveTerminalGroup(): Promise<void> {
  const t = activeTerminalTab();
  if (!t) return;
  const group = await uiPrompt("Terminal group");
  if (group === null) return;
  setTerminalGroup(t, group);
  setTransientStatus("Group set; applies on next restart");
}

registerCommands([
  {
    id: "app.terminal.surfaceTheme.light",
    title: "Terminal theme: light",
    category: "Terminal",
    keywords: ["theme", "appearance", "light"],
    available: onTerminal,
    run: () => setHybridSurfaceTheme("terminal", "light"),
  },
  {
    id: "app.terminal.surfaceTheme.dark",
    title: "Terminal theme: dark",
    category: "Terminal",
    keywords: ["theme", "appearance", "dark"],
    available: onTerminal,
    run: () => setHybridSurfaceTheme("terminal", "dark"),
  },
  {
    id: "app.terminal.broadcastToggle",
    title: "Toggle group broadcast",
    category: "Terminal",
    keywords: ["broadcast", "group", "select all", "input"],
    available: onTerminal,
    run: () => dispatchChanCommand("app.terminal.broadcastToggle"),
  },
  {
    id: "app.terminal.setName",
    title: "Set terminal name",
    category: "Terminal",
    keywords: ["rename", "name", "title"],
    available: onTerminal,
    run: () => void renameActiveTerminal(),
  },
  {
    id: "app.terminal.setGroup",
    title: "Set terminal group",
    category: "Terminal",
    keywords: ["group", "broadcast"],
    available: onTerminal,
    run: () => void setActiveTerminalGroup(),
  },
  {
    id: "terminal.richPrompt",
    title: "Show/Hide Rich Prompt",
    category: "Terminal",
    keywords: ["rich prompt", "prompt", "composer"],
    available: onWorkspaceTerminal,
    run: () => dispatchChanCommand("terminal.richPrompt"),
  },
  {
    id: "app.terminal.restart",
    title: "Restart terminal",
    category: "Terminal",
    keywords: ["restart", "respawn", "reload"],
    available: onTerminal,
    run: () => dispatchChanCommand("app.terminal.restart"),
  },
  {
    id: "app.terminal.copyCwd",
    title: "Copy path to $CWD",
    category: "Terminal",
    keywords: ["cwd", "path", "directory", "clipboard"],
    available: onTerminal,
    run: () => dispatchChanCommand("app.terminal.copyCwd"),
  },
  {
    id: "app.terminal.newFsEntry",
    title: "New file or directory ($CWD)",
    category: "Terminal",
    keywords: ["new", "file", "directory", "folder", "cwd"],
    available: onTerminal,
    run: () => dispatchChanCommand("app.terminal.newFsEntry"),
  },
]);
