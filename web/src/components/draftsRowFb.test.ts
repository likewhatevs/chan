import { describe, expect, test } from "vitest";
import fileTree from "./FileTree.svelte?raw";
import app from "../App.svelte?raw";

describe("FileTree Drafts visibility", () => {
  test("FileTree no longer has Drafts-row special casing", () => {
    expect(fileTree).not.toMatch(/drafts-row/);
    expect(fileTree).not.toMatch(/node\.path === "Drafts"/);
  });
});

describe("fullstack-a-66b: App.svelte CSS variables", () => {
  test("dark mode declares --fb-drafts-fg + --fb-drafts-bg", () => {
    expect(app).toMatch(/--fb-drafts-fg: #e3b341;/);
    expect(app).toMatch(/--fb-drafts-bg: rgba\(227, 179, 65, 0\.10\);/);
  });

  test("light mode declares deeper-hue counterparts", () => {
    expect(app).toMatch(/--fb-drafts-fg: #9a6700;/);
    expect(app).toMatch(/--fb-drafts-bg: rgba\(154, 103, 0, 0\.08\);/);
  });
});
