import { describe, expect, test } from "vitest";
import fileInfo from "./FileInfoBody.svelte?raw";

// Contact + language chip wiring in FileInfoBody. Source-pattern checks
// complement the runtime KindChip test; FileInfoBody has no jsdom mount
// harness so the exact wiring strings are pinned here.

describe("FileInfoBody language + contact wiring", () => {
  test("imports openGraphForContact and openGraphForLanguage", () => {
    expect(fileInfo).toMatch(/openGraphForContact,/);
    expect(fileInfo).toMatch(/openGraphForLanguage,/);
  });

  test("contact pill fallback now opens the contact lens (not openGraphForFile)", () => {
    expect(fileInfo).toMatch(
      /onContactNavigate[\s\S]*?:\s*\(p: string\) => openGraphForContact\(p\)/,
    );
  });

  test("directory-branch per-language row routes name click to openGraphForLanguage", () => {
    expect(fileInfo).toMatch(
      /<button[\s\S]*?class="lang-name"[\s\S]*?onclick=\{\(\) => openGraphForLanguage\(lang\.name\)\}/,
    );
  });

  test("file-branch language label routes click to openGraphForLanguage", () => {
    expect(fileInfo).toMatch(
      /<button[\s\S]*?class="lang-link"[\s\S]*?onclick=\{\(\) => openGraphForLanguage\(fileLang\)\}/,
    );
  });

  test("clickable language elements carry the graph-scope title hint", () => {
    const matches = fileInfo.match(
      /title="open in graph \(scoped to this language\)"/g,
    );
    expect(matches).not.toBeNull();
    expect(matches!.length).toBeGreaterThanOrEqual(2);
  });

  test("CSS rules for .lang-name button + .lang-link present", () => {
    expect(fileInfo).toMatch(
      /\.lang-name \{[\s\S]*?background: none;[\s\S]*?cursor: pointer;/,
    );
    expect(fileInfo).toMatch(
      /\.lang-link \{[\s\S]*?background: none;[\s\S]*?cursor: pointer;/,
    );
    expect(fileInfo).toMatch(/\.lang-name:focus-visible \{/);
    expect(fileInfo).toMatch(/\.lang-link:focus-visible \{/);
  });
});
