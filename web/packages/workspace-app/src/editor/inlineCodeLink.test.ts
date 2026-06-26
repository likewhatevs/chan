import { describe, expect, test } from "vitest";
import { codeSpanInternalTarget } from "./widgets/wikilink";
import wikilink from "./widgets/wikilink.ts?raw";
import wysiwyg from "./Wysiwyg.svelte?raw";

// An inline `code` span whose text resolves to a real workspace file renders
// as a Cmd/Ctrl-clickable internal link (detect + open, single match).

describe("codeSpanInternalTarget (the detect decision)", () => {
  test("skips code containing whitespace (a snippet, not a path)", () => {
    expect(codeSpanInternalTarget("npm install", "notes/a.md")).toBeNull();
    expect(codeSpanInternalTarget("const x = 5", "notes/a.md")).toBeNull();
  });

  test("skips external / anchor-only / empty strings", () => {
    expect(codeSpanInternalTarget("http://example.com", "notes/a.md")).toBeNull();
    expect(codeSpanInternalTarget("#section", "notes/a.md")).toBeNull();
    expect(codeSpanInternalTarget("", "notes/a.md")).toBeNull();
  });

  test("resolves a bare stem against the editing file's directory", () => {
    expect(codeSpanInternalTarget("pasta", "notes/a.md")).toBe("notes/pasta");
  });

  test("resolves a workspace-rooted path (leading slash)", () => {
    expect(codeSpanInternalTarget("/guide", "notes/a.md")).toBe("guide");
  });

  test("resolves with no editing file (workspace-relative)", () => {
    expect(codeSpanInternalTarget("pasta", null)).toBe("pasta");
  });

  test("skips the current file by its stem (no self link)", () => {
    // `a` in notes/a.md normalizes to notes/a == the stem of the current file.
    expect(codeSpanInternalTarget("a", "notes/a.md")).toBeNull();
  });
});

describe("inline-code link decoration + open wiring", () => {
  test("decorates only a resolved real file, as a non-atomic data-carrying mark", () => {
    expect(wikilink).toMatch(/if \(getKind\(target\) !== "file"\) return;/);
    expect(wikilink).toMatch(
      /class: "cm-md-code-link",\s*attributes: \{ "data-code-link-target": target \},/,
    );
    // Non-atomic: a Decoration.mark, never the atomic wiki-pill replace widget.
    expect(wikilink).toMatch(/function codeLinkMark\(target: string\): Decoration \{\s*return Decoration\.mark\(/);
  });

  test("the ViewPlugin re-runs on the shared kind-resolve broadcast", () => {
    expect(wikilink).toMatch(/export function inlineCodeLinkDecorations\(/);
    expect(wikilink).toMatch(
      /update\(u: ViewUpdate\): void \{[\s\S]*?e\.is\(kindResolvedEffect\)[\s\S]*?scanInlineCodeLinks\(u\.view/,
    );
  });

  test("Cmd/Ctrl-click opens via onWikiClick using the mark's data attribute", () => {
    expect(wikilink).toMatch(
      /export function inlineCodeLinkClickHandler\([\s\S]*?if \(!\(event\.metaKey \|\| event\.ctrlKey\)\) return false;[\s\S]*?closest\(\s*"\.cm-md-code-link",\s*\)[\s\S]*?dataset\.codeLinkTarget;[\s\S]*?opts\.onWikiClick\(\{/,
    );
  });

  test("Wysiwyg wires both the decorator and the Cmd/Ctrl-click opener", () => {
    expect(wysiwyg).toMatch(/inlineCodeLinkClickHandler\(\{\s*onWikiClick,/);
    expect(wysiwyg).toMatch(/inlineCodeLinkDecorations\(\{\s*onWikiClick,/);
  });
});
