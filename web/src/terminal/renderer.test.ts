import { describe, expect, test } from "vitest";
import { refreshTerminalRows, shouldUseWebglRenderer } from "./renderer";

describe("terminal renderer helpers", () => {
  test("repaints only visible rows through xterm refresh", () => {
    const calls: Array<[number, number]> = [];
    refreshTerminalRows({ rows: 24, refresh: (start, end) => calls.push([start, end]) });
    expect(calls).toEqual([[0, 23]]);
  });

  test("does not require refresh support", () => {
    expect(() => refreshTerminalRows({ rows: 1 })).not.toThrow();
  });

  test("keeps Linux desktop on the DOM renderer", () => {
    expect(shouldUseWebglRenderer(true, "linux")).toBe(false);
    expect(shouldUseWebglRenderer(true, "mac")).toBe(true);
    expect(shouldUseWebglRenderer(false, "linux")).toBe(true);
  });
});
