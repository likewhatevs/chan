// The per-window orphan flash: a window in the feed with no live handle here
// slow-flashes for a (re)open click. These cover the mark/clear/has/reset API
// keyed directly by window_id.

import { describe, it, expect, beforeEach } from "vitest";
import {
  markWindowAttention,
  clearWindowAttention,
  hasWindowAttention,
  clearAllWindowAttention,
} from "./windowAttention.svelte";

beforeEach(() => {
  clearAllWindowAttention();
});

describe("windowAttention", () => {
  it("starts clear for any window", () => {
    expect(hasWindowAttention("w-1")).toBe(false);
  });

  it("flags a window and reports it", () => {
    markWindowAttention("w-1");
    expect(hasWindowAttention("w-1")).toBe(true);
    expect(hasWindowAttention("w-2")).toBe(false);
  });

  it("clears one window without touching others", () => {
    markWindowAttention("w-1");
    markWindowAttention("w-2");
    clearWindowAttention("w-1");
    expect(hasWindowAttention("w-1")).toBe(false);
    expect(hasWindowAttention("w-2")).toBe(true);
  });

  it("clearing an unflagged window is a no-op", () => {
    expect(() => clearWindowAttention("nope")).not.toThrow();
    expect(hasWindowAttention("nope")).toBe(false);
  });

  it("marking twice stays flagged (idempotent)", () => {
    markWindowAttention("w-1");
    markWindowAttention("w-1");
    expect(hasWindowAttention("w-1")).toBe(true);
    clearWindowAttention("w-1");
    expect(hasWindowAttention("w-1")).toBe(false);
  });

  it("clearAll resets every flag", () => {
    markWindowAttention("w-1");
    markWindowAttention("w-2");
    clearAllWindowAttention();
    expect(hasWindowAttention("w-1")).toBe(false);
    expect(hasWindowAttention("w-2")).toBe(false);
  });
});
