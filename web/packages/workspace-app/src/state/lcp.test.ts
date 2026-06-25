import { describe, expect, test } from "vitest";
import { longestCommonPrefix } from "./lcp";

describe("longestCommonPrefix", () => {
  test("empty input → empty string", () => {
    expect(longestCommonPrefix([])).toBe("");
  });

  test("single entry → that entry", () => {
    expect(longestCommonPrefix(["notes"])).toBe("notes");
  });

  test("shared prefix shorter than any entry", () => {
    expect(longestCommonPrefix(["notes/2024", "notes/2025"])).toBe("notes/202");
  });

  test("one entry is a prefix of the other", () => {
    expect(longestCommonPrefix(["notes", "notes/today"])).toBe("notes");
  });

  test("no shared prefix → empty", () => {
    expect(longestCommonPrefix(["alpha", "beta"])).toBe("");
  });

  test("identical entries → that entry", () => {
    expect(longestCommonPrefix(["foo/bar", "foo/bar"])).toBe("foo/bar");
  });

  test("case-sensitive comparison", () => {
    expect(longestCommonPrefix(["Notes", "notes"])).toBe("");
  });

  test("respects path separators (does not split on /)", () => {
    expect(longestCommonPrefix(["a/b/c", "a/b/d", "a/b/e"])).toBe("a/b/");
  });
});
