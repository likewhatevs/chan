// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import {
  NAMED_PANE_HEX,
  applyInitialPaneColor,
  namedForPaneHex,
  normalizeHexColor,
  seedInitialFocusColor,
} from "./paneColor";
import paneSource from "../components/Pane.svelte?raw";
import clientSource from "../api/client.ts?raw";

const CSS_VAR = "--pane-highlight-color";

function setSearch(search: string): void {
  window.history.replaceState(null, "", `/${search}`);
}

afterEach(() => {
  document.documentElement.style.removeProperty(CSS_VAR);
  setSearch("");
});

describe("normalizeHexColor", () => {
  test("accepts #rrggbb and lowercases", () => {
    expect(normalizeHexColor("#E58C4D")).toBe("#e58c4d");
    expect(normalizeHexColor("#aabbcc")).toBe("#aabbcc");
  });
  test("accepts a bare 6-digit hex (no leading #)", () => {
    expect(normalizeHexColor("aabbcc")).toBe("#aabbcc");
  });
  test("expands #rgb shorthand to #rrggbb", () => {
    expect(normalizeHexColor("#abc")).toBe("#aabbcc");
    expect(normalizeHexColor("abc")).toBe("#aabbcc");
  });
  test("trims surrounding whitespace", () => {
    expect(normalizeHexColor("  #abc  ")).toBe("#aabbcc");
  });
  test("rejects named colours", () => {
    expect(normalizeHexColor("red")).toBeNull();
  });
  test("rejects CSS / injection payloads", () => {
    expect(normalizeHexColor("javascript:alert(1)")).toBeNull();
    expect(normalizeHexColor("#abc; color: red")).toBeNull();
    expect(normalizeHexColor("red; }")).toBeNull();
  });
  test("rejects non-hex digits and bad lengths", () => {
    expect(normalizeHexColor("#xyz")).toBeNull();
    expect(normalizeHexColor("#ggg")).toBeNull();
    expect(normalizeHexColor("#abcd")).toBeNull();
    expect(normalizeHexColor("#abcde")).toBeNull();
    expect(normalizeHexColor("#abcdefg")).toBeNull();
  });
  test("rejects empty / nullish", () => {
    expect(normalizeHexColor("")).toBeNull();
    expect(normalizeHexColor("#")).toBeNull();
    expect(normalizeHexColor(null)).toBeNull();
    expect(normalizeHexColor(undefined)).toBeNull();
  });
});

describe("applyInitialPaneColor", () => {
  test("sets the CSS var from a valid ?pane= (URL-encoded #)", () => {
    setSearch("?pane=%23e58c4d");
    applyInitialPaneColor();
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe(
      "#e58c4d",
    );
  });
  test("sets the CSS var from a bare hex ?pane=", () => {
    setSearch("?pane=aabbcc");
    applyInitialPaneColor();
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe(
      "#aabbcc",
    );
  });
  test("leaves the var unset when ?pane= is absent", () => {
    setSearch("?t=token");
    applyInitialPaneColor();
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe("");
  });
  test("leaves the var unset when ?pane= is invalid", () => {
    setSearch("?pane=red");
    applyInitialPaneColor();
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe("");
  });
});

describe("NAMED_PANE_HEX named <-> hex map", () => {
  test("blue is the --pane-focus literal, the other three mirror the presets", () => {
    expect(NAMED_PANE_HEX).toEqual({
      blue: "#388bfd",
      orange: "#f97316",
      green: "#22c55e",
      pink: "#ff5fb7",
    });
  });
});

describe("namedForPaneHex reverse lookup", () => {
  test("maps each preset hex back to its name", () => {
    expect(namedForPaneHex("#388bfd")).toBe("blue");
    expect(namedForPaneHex("#f97316")).toBe("orange");
    expect(namedForPaneHex("#22c55e")).toBe("green");
    expect(namedForPaneHex("#ff5fb7")).toBe("pink");
  });
  test("normalizes case + shorthand before matching", () => {
    expect(namedForPaneHex("#388BFD")).toBe("blue");
  });
  test("returns null for a valid but non-preset hex", () => {
    expect(namedForPaneHex("#abcdef")).toBeNull();
  });
  test("returns null for invalid / nullish input", () => {
    expect(namedForPaneHex("red")).toBeNull();
    expect(namedForPaneHex(null)).toBeNull();
    expect(namedForPaneHex(undefined)).toBeNull();
  });
});

describe("seedInitialFocusColor boot-seed", () => {
  test("selects the named preset when ?pane= is a preset hex", () => {
    setSearch("?pane=%23388bfd");
    const setColor = vi.fn();
    seedInitialFocusColor(setColor);
    expect(setColor).toHaveBeenCalledWith("blue");
    expect(setColor).toHaveBeenCalledTimes(1);
  });
  test("ignores a valid but non-preset ?pane= hex", () => {
    setSearch("?pane=%23abcdef");
    const setColor = vi.fn();
    seedInitialFocusColor(setColor);
    expect(setColor).not.toHaveBeenCalled();
  });
  test("is a no-op when ?pane= is absent", () => {
    setSearch("?t=token");
    const setColor = vi.fn();
    seedInitialFocusColor(setColor);
    expect(setColor).not.toHaveBeenCalled();
  });
});

describe("Pane.svelte doSetFocusColor persists + recolours per library", () => {
  test("keeps the existing per-window preset call", () => {
    expect(paneSource).toMatch(/setWindowFocusColor\(color\);/);
  });
  test("sets --pane-highlight-color to the mapped hex live", () => {
    expect(paneSource).toMatch(/const hex = NAMED_PANE_HEX\[color\];/);
    expect(paneSource).toMatch(
      /setProperty\("--pane-highlight-color", hex\)/,
    );
  });
  test("fires api.setLocalColor(hex) best-effort, swallowing failure", () => {
    expect(paneSource).toMatch(
      /void api\.setLocalColor\(hex\)\.catch\([\s\S]*?console\.warn/,
    );
  });
});

describe("api.setLocalColor PUTs the library local-color route", () => {
  test("setLocalColor issues PUT /api/library/local-color with { color }", () => {
    expect(clientSource).toMatch(
      /setLocalColor: \(color: string\) =>[\s\S]*?req<void>\([\s\S]*?"PUT",[\s\S]*?"\/api\/library\/local-color",[\s\S]*?\{ color \}/,
    );
  });
});

describe("Pane.svelte active-pane highlight prefers --pane-highlight-color", () => {
  test(".pane.focused border falls back through the highlight var", () => {
    expect(paneSource).toMatch(
      /\.pane\.focused \{[\s\S]*?border-color: var\(--pane-highlight-color, var\(--pane-active-focus\)\);/,
    );
  });
  test("the focus halo box-shadow uses the highlight var inside color-mix", () => {
    expect(paneSource).toMatch(
      /color-mix\(in srgb, var\(--pane-highlight-color, var\(--pane-active-focus\)\) 55%, transparent\)/,
    );
  });
  test("the data-focus-color presets are unchanged", () => {
    expect(paneSource).toMatch(
      /\.pane\[data-focus-color="blue"\] \{ --pane-active-focus: var\(--pane-focus\); \}/,
    );
    expect(paneSource).toMatch(
      /\.pane\[data-focus-color="orange"\] \{ --pane-active-focus: #f97316; \}/,
    );
  });
});
