import { describe, expect, test } from "vitest";
import app from "./App.svelte?raw";

// Comma opens Settings. The focused-pane flip remains a separate command and
// must not be wired to the comma key path.

describe("Cmd+, opens Settings", () => {
  test("onWindowKey routes comma to app.settings.open", () => {
    expect(app).toMatch(
      /const settingsChord =[\s\S]*?os === "mac"[\s\S]*?e\.metaKey[\s\S]*?e\.code === "Comma"[\s\S]*?: e\.ctrlKey[\s\S]*?e\.code === "Comma"[\s\S]*?builtInChordSuperseded\("app\.settings\.open"\)[\s\S]*?openSettings\(\);/,
    );
  });

  test("the flip action stays off the comma key path", () => {
    expect(app).toMatch(/case "app\.settings\.toggle":/);
    expect(app).not.toMatch(
      /"Comma"[\s\S]{1,200}flipHybrid\(layout\.activePaneId\)/,
    );
  });
});
