import { describe, expect, test } from "vitest";
import fileTree from "./FileTree.svelte?raw";
import app from "../App.svelte?raw";

// `fullstack-a-66b`: FB Drafts row. After `systacean-29`'s
// Drive::list unified-path extension, chan-server's
// `api_list_files` injects a synthetic "Drafts" entry at the
// top of the root listing. The FB tree renders it as the
// first row in a yellow tone (light + dark mode variants).
// Expansion into `Drafts/<name>/...` reuses the existing
// /api/files?dir=Drafts route + the unified Drive::list.

describe("fullstack-a-66b: FileTree Drafts row markup", () => {
  test("dir row gains drafts-row class when path === 'Drafts'", () => {
    expect(fileTree).toMatch(
      /class:drafts-row=\{node\.path === "Drafts"\}/,
    );
  });

  test("CSS rules tint .row.dir.drafts-row + its icon + name with --fb-drafts-fg", () => {
    expect(fileTree).toMatch(
      /\.row\.dir\.drafts-row \.dir-icon \{[\s\S]*?color: var\(--fb-drafts-fg\);/,
    );
    expect(fileTree).toMatch(
      /\.row\.dir\.drafts-row > \.name \{[\s\S]*?color: var\(--fb-drafts-fg\);/,
    );
    expect(fileTree).toMatch(
      /\.row\.dir\.drafts-row \{[\s\S]*?background: var\(--fb-drafts-bg\);/,
    );
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
