// Editor surface commands: available when a file tab is the active
// surface. Chorded reuse ids dispatch through the chan:command bridge so
// they share App.svelte's dispatch and its window-mode guard; net-new
// actions call their exported action function directly against the active
// file tab. See state/commands.ts for the Command shape and the
// onSurface / dispatchChanCommand / workspaceOnly helpers.

import {
  registerCommands,
  dispatchChanCommand,
  onSurface,
  workspaceOnly,
} from "../commands";
import {
  copyTextToClipboard,
  fileOps,
  openFsGraphForFile,
  revealPathInBrowser,
  searchPanel,
  setHybridSurfaceTheme,
} from "../store.svelte";
import { editorCommandsFor } from "../mountedEditors";
import {
  activeFileTab,
  openTerminalInActivePane,
  setTabHighlightTrailingWhitespace,
  setTabInspectorOpen,
  setTabOutlineOpen,
  setTabStyleToolbarOpen,
  type FileTab,
} from "../tabs.svelte";
import {
  DEFAULT_RATIO,
  PAGE_WIDTH_STEP_PCT,
  pageWidth,
  setPageWidth,
} from "../pageWidth.svelte";
import { notify } from "../notify.svelte";
import { terminalFromHereTarget } from "../../terminal/fromHere";
import { stripTrailingWhitespaceText } from "../../editor/tools";

/// Run an action against the active file tab, a no-op when none is
/// active. onSurface already hides these when no file tab is focused; the
/// guard keeps a stale invocation (tab closed between filter and run) safe.
function onFile(fn: (tab: FileTab) => void): () => void {
  return () => {
    const tab = activeFileTab();
    if (tab) fn(tab);
  };
}

/// Parent directory of a workspace-relative path (empty at the root),
/// matching the editor tab menu's "Copy path to $CWD".
function parentDir(path: string): string {
  const slash = path.lastIndexOf("/");
  return slash <= 0 ? "" : path.slice(0, slash);
}

const STEP = PAGE_WIDTH_STEP_PCT / 100;

registerCommands([
  {
    id: "app.editor.surfaceTheme.light",
    title: "Editor theme: light",
    category: "Editor",
    keywords: ["theme", "light", "appearance"],
    available: (ctx) => onSurface(ctx, "file"),
    run: () => setHybridSurfaceTheme("editor", "light"),
  },
  {
    id: "app.editor.surfaceTheme.dark",
    title: "Editor theme: dark",
    category: "Editor",
    keywords: ["theme", "dark", "appearance"],
    available: (ctx) => onSurface(ctx, "file"),
    run: () => setHybridSurfaceTheme("editor", "dark"),
  },
  {
    id: "app.editor.toggleMode",
    title: "Show source code",
    category: "Editor",
    keywords: ["source", "raw", "markdown", "wysiwyg", "toggle"],
    available: (ctx) => onSurface(ctx, "file"),
    run: () => dispatchChanCommand("app.editor.toggleMode"),
  },
  {
    id: "app.file.new",
    title: "New file",
    category: "Editor",
    keywords: ["create", "file"],
    // A workspace create action, not surface-bound; hidden only in a
    // standalone terminal window.
    available: (ctx) => workspaceOnly(ctx),
    run: () => dispatchChanCommand("app.file.new"),
  },
  {
    id: "app.file.duplicate",
    title: "Duplicate file",
    category: "Editor",
    keywords: ["copy", "clone"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => {
      void fileOps.duplicateFile(tab.path);
    }),
  },
  {
    id: "app.file.rename",
    title: "Rename file",
    category: "Editor",
    keywords: ["move", "rename"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => {
      void fileOps.rename(tab.path, false);
    }),
  },
  {
    id: "app.editor.styleToolbar",
    title: "Toggle style toolbar",
    category: "Editor",
    keywords: ["format", "markdown", "toolbar", "style"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => setTabStyleToolbarOpen(tab, !tab.styleToolbarOpen)),
  },
  {
    id: "app.editor.toggleCollapse",
    title: "Toggle collapse code blocks",
    category: "Editor",
    keywords: ["fold", "code", "collapse", "expand", "blocks"],
    available: (ctx) => onSurface(ctx, "file"),
    // The editor view owns fold ranges, so reach it through the mounted
    // editor registry. A no-op on files without foldable code blocks.
    run: onFile((tab) => editorCommandsFor(tab.id)?.toggleCodeBlocks()),
  },
  {
    id: "app.editor.copyPath",
    title: "Copy path to file",
    category: "Editor",
    keywords: ["clipboard", "path"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => {
      void copyTextToClipboard(tab.path, {
        onSuccess: () => notify("Copied file path"),
        onError: () => notify("Clipboard unavailable"),
      });
    }),
  },
  {
    id: "app.editor.copyParentPath",
    title: "Copy path to parent directory",
    category: "Editor",
    keywords: ["clipboard", "directory", "folder", "parent"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => {
      void copyTextToClipboard(parentDir(tab.path), {
        onSuccess: () => notify("Copied directory path"),
        onError: () => notify("Clipboard unavailable"),
      });
    }),
  },
  {
    id: "app.editor.terminalFromHere",
    title: "Terminal from here",
    category: "Editor",
    keywords: ["shell", "console", "cwd"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => {
      openTerminalInActivePane(terminalFromHereTarget(tab.path, false));
    }),
  },
  {
    id: "app.editor.graphFromHere",
    title: "Graph from here",
    category: "Editor",
    keywords: ["graph", "links", "scope"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => openFsGraphForFile(tab.path)),
  },
  {
    id: "app.editor.showInBrowser",
    title: "Show in file browser",
    category: "Editor",
    keywords: ["reveal", "files", "tree", "explorer"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => {
      revealPathInBrowser(tab.path, { inspectorOpen: true });
    }),
  },
  {
    id: "app.editor.searchSelection",
    title: "Search selection",
    category: "Editor",
    keywords: ["search", "find", "selection"],
    available: (ctx) => onSurface(ctx, "file"),
    // The selection lives in the editor view's state, so it survives the
    // launcher taking focus; seed the search overlay with it, capped at
    // 100 words. With no selection this opens plain search.
    run: onFile((tab) => {
      const selection = editorCommandsFor(tab.id)?.selectionText() ?? "";
      const words = selection.trim().split(/\s+/).filter(Boolean);
      if (words.length > 0) searchPanel.query = words.slice(0, 100).join(" ");
      searchPanel.open = true;
    }),
  },
  {
    id: "app.editor.toggleTrailingWs",
    title: "Toggle highlight trailing whitespace",
    category: "Editor",
    keywords: ["whitespace", "trailing", "highlight"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) =>
      setTabHighlightTrailingWhitespace(tab, !tab.highlightTrailingWhitespace),
    ),
  },
  {
    id: "app.editor.stripTrailingWs",
    title: "Remove trailing whitespace",
    category: "Editor",
    keywords: ["whitespace", "trailing", "strip", "clean"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => {
      tab.content = stripTrailingWhitespaceText(tab.content);
    }),
  },
  {
    id: "app.editor.outline",
    title: "Toggle outline",
    category: "Editor",
    keywords: ["outline", "headings", "toc"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => setTabOutlineOpen(tab, !tab.outlineOpen)),
  },
  {
    id: "app.editor.details",
    title: "Toggle details",
    category: "Editor",
    keywords: ["inspector", "details", "info", "metadata"],
    available: (ctx) => onSurface(ctx, "file"),
    run: onFile((tab) => setTabInspectorOpen(tab, !tab.inspectorOpen)),
  },
  {
    id: "app.editor.pageWidth.narrower",
    title: "Page width: narrower",
    category: "Editor",
    keywords: ["width", "reading", "column", "narrow"],
    available: (ctx) => onSurface(ctx, "file"),
    run: () => setPageWidth(pageWidth.ratio - STEP),
  },
  {
    id: "app.editor.pageWidth.wider",
    title: "Page width: wider",
    category: "Editor",
    keywords: ["width", "reading", "column", "wide"],
    available: (ctx) => onSurface(ctx, "file"),
    run: () => setPageWidth(pageWidth.ratio + STEP),
  },
  {
    id: "app.editor.pageWidth.reset",
    title: "Page width: reset",
    category: "Editor",
    keywords: ["width", "reading", "column", "default", "reset"],
    available: (ctx) => onSurface(ctx, "file"),
    run: () => setPageWidth(DEFAULT_RATIO),
  },
]);
