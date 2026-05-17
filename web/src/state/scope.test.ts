import { describe, expect, test } from "vitest";
import { commonAncestor, parentDir, scopeKey } from "./scope.svelte";

describe("scopeKey", () => {
  test("sorts and joins with pipe", () => {
    expect(scopeKey(["b", "a"])).toBe("a|b");
  });

  test("empty input → empty string", () => {
    expect(scopeKey([])).toBe("");
  });

  test("single entry → that entry", () => {
    expect(scopeKey(["only"])).toBe("only");
  });
});

describe("parentDir", () => {
  test("top-level file → empty string", () => {
    expect(parentDir("file.md")).toBe("");
  });

  test("nested file → directory portion", () => {
    expect(parentDir("notes/today.md")).toBe("notes");
  });

  test("deeply nested file", () => {
    expect(parentDir("a/b/c/d/e.md")).toBe("a/b/c/d");
  });

  test("empty input → empty string", () => {
    expect(parentDir("")).toBe("");
  });

  test("folder with trailing path component", () => {
    expect(parentDir("notes/2024")).toBe("notes");
  });
});

describe("commonAncestor", () => {
  test("empty input → empty string", () => {
    expect(commonAncestor([])).toBe("");
  });

  test("single file → that file's parent dir", () => {
    expect(commonAncestor(["notes/today.md"])).toBe("notes");
  });

  test("two siblings → their parent", () => {
    expect(commonAncestor(["notes/a.md", "notes/b.md"])).toBe("notes");
  });

  test("two cousins → grandparent", () => {
    expect(commonAncestor(["a/x/1.md", "a/y/2.md"])).toBe("a");
  });

  test("disjoint top-level files → empty string", () => {
    expect(commonAncestor(["one.md", "two.md"])).toBe("");
  });

  test("mixed depth shares deepest common dir", () => {
    expect(commonAncestor(["a/b/c.md", "a/b/d/e.md"])).toBe("a/b");
  });

  test("paths under different top-level dirs → empty", () => {
    expect(commonAncestor(["docs/intro.md", "notes/today.md"])).toBe("");
  });
});
