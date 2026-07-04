// Reuse-existing Global, Tabs, and Panes commands: each maps a launcher
// entry to an action that already runs through a chord. Chorded ids
// dispatch a chan:command so they share App.svelte's dispatch and its
// window-mode guard; the two chordless entries (Hybrid Nav, Reopen closed
// tab) have no runCommand case, so they call their exported action
// directly and gate visibility themselves.

import {
  registerCommands,
  dispatchChanCommand,
  allowedInWindow,
  type Command,
  type CommandCategory,
} from "../commands";
import {
  enterPaneMode,
  reopenClosedTab,
  canReopenClosedTab,
} from "../tabs.svelte";

/// A reuse-existing chorded command: run() dispatches the id, available()
/// mirrors runCommand's window-mode guard so the launcher hides it in the
/// same windows runCommand would drop it.
function reuse(
  id: string,
  title: string,
  category: CommandCategory,
  keywords: string[],
): Command {
  return {
    id,
    title,
    category,
    keywords,
    available: (ctx) => allowedInWindow(id, ctx),
    run: () => dispatchChanCommand(id),
  };
}

registerCommands([
  // Global
  reuse("app.search.toggle", "Search", "Global", ["find", "grep"]),
  {
    id: "app.pane.mode",
    title: "Enter hybrid navigation",
    category: "Global",
    keywords: ["nav", "pane", "keyboard", "wasd"],
    // Hybrid Nav has no terminal-only guard on its chord, so it stays
    // available in every window, standalone terminals included.
    available: () => true,
    run: () => enterPaneMode(),
  },
  {
    id: "app.tab.reopenClosed",
    title: "Reopen last closed tab",
    category: "Global",
    keywords: ["undo", "restore", "tab"],
    // Only offer it when the closed-tab stack has something to restore.
    available: () => canReopenClosedTab(),
    run: () => {
      reopenClosedTab();
    },
  },
  reuse("app.screensaver.lock", "Lock screen now", "Global", [
    "screensaver",
    "lock",
  ]),

  // Tabs
  reuse("app.terminal.toggle", "New terminal", "Tabs", ["shell", "console"]),
  reuse("app.terminal.teamWork", "New team", "Tabs", ["team work", "agents"]),
  reuse("app.draft.new", "New draft", "Tabs", ["markdown", "note"]),
  reuse("app.graph.toggle", "New graph", "Tabs", ["links", "network"]),
  reuse("app.files.toggle", "New file browser", "Tabs", [
    "files",
    "tree",
    "explorer",
  ]),
  reuse("app.dashboard.open", "New dashboard", "Tabs", ["slides", "present"]),
  reuse("app.tab.close", "Close tab", "Tabs", ["close"]),

  // Panes
  reuse("app.pane.splitRight", "Split right", "Panes", ["split", "vertical"]),
  reuse("app.pane.splitDown", "Split bottom", "Panes", [
    "split",
    "horizontal",
    "down",
  ]),
  reuse("app.pane.prev", "Previous pane", "Panes", ["focus", "pane"]),
  reuse("app.pane.next", "Next pane", "Panes", ["focus", "pane"]),
  reuse("app.pane.closeTabs", "Close all tabs in pane", "Panes", ["close"]),
  reuse("app.pane.kill", "Close pane", "Panes", ["close", "kill"]),
  reuse("app.settings.toggle", "Flip focused pane", "Panes", [
    "flip",
    "settings",
    "config",
    "back",
  ]),
]);
