// Registration and availability gating for the Editor, File Browser,
// Graph, and Dashboard surface command modules. Importing the modules is the
// registration side effect; availableCommands then filters by a
// hand-built context, so these assert the surface gates without a live
// layout or the launcher UI.

import { describe, it, expect } from "vitest";
import { availableCommands, type CommandContext } from "../commands";

import "./editor";
import "./browser";
import "./graph";
import "./dashboard";
import "./diagram";
import "./terminal";

function ctx(partial: Partial<CommandContext>): CommandContext {
  return {
    terminalOnly: false,
    terminalControl: false,
    activeSurface: null,
    ...partial,
  };
}

function idsIn(c: CommandContext): Set<string> {
  return new Set(availableCommands(c).map((cmd) => cmd.id));
}

describe("editor surface commands", () => {
  it("appear only when a file tab is the active surface", () => {
    const onFile = idsIn(ctx({ activeSurface: "file" }));
    for (const id of [
      "app.editor.surfaceTheme.dark",
      "app.editor.toggleMode",
      "app.editor.outline",
      "app.editor.copyPath",
      "app.editor.copyParentPath",
      "app.file.duplicate",
      "app.file.rename",
      "app.editor.stripTrailingWs",
      "app.editor.toggleCollapse",
      "app.editor.searchSelection",
    ]) {
      expect(onFile.has(id)).toBe(true);
    }
    expect(idsIn(ctx({ activeSurface: "graph" })).has("app.editor.outline")).toBe(false);
  });

  it("New file follows the workspace gate, not the file surface", () => {
    expect(idsIn(ctx({ activeSurface: "graph" })).has("app.file.new")).toBe(true);
    expect(
      idsIn(ctx({ terminalOnly: true, activeSurface: "terminal" })).has("app.file.new"),
    ).toBe(false);
  });
});

describe("graph surface commands", () => {
  it("appear only on a graph surface", () => {
    const onGraph = idsIn(ctx({ activeSurface: "graph" }));
    for (const id of [
      "app.graph.surfaceTheme.light",
      "app.graph.copyLink",
      "app.graph.depth.increase",
      "app.graph.filter.contact",
      "app.graph.filter.media",
    ]) {
      expect(onGraph.has(id)).toBe(true);
    }
    expect(idsIn(ctx({ activeSurface: "file" })).has("app.graph.copyLink")).toBe(false);
  });
});

describe("file browser surface commands", () => {
  it("appear only on a file browser surface", () => {
    const onBrowser = idsIn(ctx({ activeSurface: "browser" }));
    for (const id of [
      "app.browser.surfaceTheme.dark",
      "app.browser.expandAll",
      "app.browser.importContacts",
      "app.browser.newFsEntry",
      "app.browser.newGraph",
      "app.browser.newTerminal",
      "app.browser.toggleLeftDock",
      "app.browser.uploadSelection",
    ]) {
      expect(onBrowser.has(id)).toBe(true);
    }
    expect(idsIn(ctx({ activeSurface: "file" })).has("app.browser.newGraph")).toBe(false);
  });
});

describe("terminal surface commands", () => {
  it("appear only on a workspace terminal surface", () => {
    const onTerminal = idsIn(ctx({ activeSurface: "terminal" }));
    for (const id of [
      "app.terminal.broadcastToggle",
      "app.terminal.copyCwd",
      "terminal.richPrompt",
    ]) {
      expect(onTerminal.has(id)).toBe(true);
    }
    expect(idsIn(ctx({ activeSurface: "file" })).has("terminal.richPrompt")).toBe(
      false,
    );
    expect(
      idsIn(ctx({ terminalOnly: true, activeSurface: "terminal" })).has(
        "terminal.richPrompt",
      ),
    ).toBe(false);
  });
});

describe("dashboard surface commands", () => {
  it("appear only on a dashboard surface", () => {
    const onDash = idsIn(ctx({ activeSurface: "dashboard" }));
    for (const id of [
      "app.dashboard.surfaceTheme.dark",
      "app.dashboard.nextSlide",
      "app.dashboard.prevSlide",
    ]) {
      expect(onDash.has(id)).toBe(true);
    }
    expect(idsIn(ctx({ activeSurface: "file" })).has("app.dashboard.nextSlide")).toBe(false);
  });
});

describe("new diagram command", () => {
  it("follows the workspace gate, independent of the active surface", () => {
    expect(idsIn(ctx({ activeSurface: "file" })).has("app.diagram.new")).toBe(true);
    expect(idsIn(ctx({ activeSurface: null })).has("app.diagram.new")).toBe(true);
    expect(
      idsIn(ctx({ terminalOnly: true, activeSurface: "terminal" })).has("app.diagram.new"),
    ).toBe(false);
  });
});

describe("surface commands in a standalone terminal window", () => {
  it("hide every editor, graph, and dashboard entry", () => {
    const inTerminal = idsIn(ctx({ terminalOnly: true, activeSurface: "terminal" }));
    for (const id of [
      "app.editor.outline",
      "app.browser.newGraph",
      "app.graph.copyLink",
      "app.dashboard.nextSlide",
      "terminal.richPrompt",
    ]) {
      expect(inTerminal.has(id)).toBe(false);
    }
  });
});
