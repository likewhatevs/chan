import { describe, expect, test } from "vitest";
import { parentDir, scopeKey } from "./scope.svelte";

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
