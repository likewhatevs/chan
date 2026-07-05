// The launcher category split: "Apps" holds the spawn commands, "Tabs"
// holds the tab operations (close / next / prev / reopen). Importing the
// modules is the registration side effect; allCommands() then exposes the
// catalog so these assert category membership without the launcher UI.

import { describe, it, expect } from "vitest";
import { allCommands } from "../commands";

import "./core";
import "./diagram";

function categoryOf(id: string): string | undefined {
  return allCommands().find((c) => c.id === id)?.category;
}

describe("launcher Apps / Tabs split", () => {
  it("keeps the spawn commands under Apps", () => {
    for (const id of [
      "app.terminal.toggle",
      "app.terminal.teamWork",
      "app.draft.new",
      "app.graph.toggle",
      "app.files.toggle",
      "app.dashboard.open",
      "app.diagram.new",
    ]) {
      expect(categoryOf(id)).toBe("Apps");
    }
  });

  it("groups the tab operations under Tabs", () => {
    for (const id of [
      "app.tab.close",
      "app.tab.next",
      "app.tab.prev",
      "app.tab.reopenClosed",
    ]) {
      expect(categoryOf(id)).toBe("Tabs");
    }
  });

  it("does not surface Jump to tab in the launcher", () => {
    expect(categoryOf("app.tab.jump")).toBeUndefined();
  });
});
