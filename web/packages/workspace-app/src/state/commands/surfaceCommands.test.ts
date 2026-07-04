// Registration and availability gating for the Editor, Graph, and
// Dashboard surface command modules. Importing the three modules is the
// registration side effect; availableCommands then filters by a
// hand-built context, so these assert the surface gates without a live
// layout or the launcher UI.

import { describe, it, expect } from "vitest";
import { availableCommands, type CommandContext } from "../commands";

import "./editor";
import "./graph";
import "./dashboard";

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
      "app.editor.pageWidth.reset",
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

describe("surface commands in a standalone terminal window", () => {
  it("hide every editor, graph, and dashboard entry", () => {
    const inTerminal = idsIn(ctx({ terminalOnly: true, activeSurface: "terminal" }));
    for (const id of [
      "app.editor.outline",
      "app.graph.copyLink",
      "app.dashboard.nextSlide",
    ]) {
      expect(inTerminal.has(id)).toBe(false);
    }
  });
});
