// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { applyInitialPaneColor, normalizeHexColor } from "./paneColor";
import paneSource from "../components/Pane.svelte?raw";

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
