// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import {
  NAMED_PANE_HEX,
  applyInitialPaneColor,
  applyLivePaneColor,
  namedForPaneHex,
  normalizeHexColor,
  seedInitialFocusColor,
  syncLiveFocusColorMenu,
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

describe("applyLivePaneColor colour-watch apply", () => {
  test("sets the CSS var from a valid hex frame", () => {
    applyLivePaneColor("#e58c4d");
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe("#e58c4d");
  });
  test("normalizes shorthand + case before applying", () => {
    applyLivePaneColor("#ABC");
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe("#aabbcc");
  });
  test("LEAVES a null frame as no-override (keeps the ?pane= seed — Bug A)", () => {
    document.documentElement.style.setProperty(CSS_VAR, "#e58c4d");
    applyLivePaneColor(null);
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe("#e58c4d");
  });
  test("LEAVES an invalid colour as no-override (no clobber, no injection)", () => {
    document.documentElement.style.setProperty(CSS_VAR, "#e58c4d");
    applyLivePaneColor("red; }");
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe("#e58c4d");
  });
  test("a null frame on an UNSEEDED window leaves the var unset (default accent)", () => {
    applyLivePaneColor(null);
    expect(document.documentElement.style.getPropertyValue(CSS_VAR)).toBe("");
  });
});

// Two or more windows of the same library (incl. split panes — the var is
// on the document root, so every pane in a window reads it) must converge on
// the library's non-blue focus border and never fall back to default blue. The
// per-call tests above lock each apply() in isolation; these lock the ORDERED
// sequences the real multi-window race produces (seed, then the watch's
// push-on-connect + reconnect storms), which is where the stuck-blue bug lives.
// Once a valid colour is present (from seed OR a live push), no later
// null/late push may revert it to blue.
describe("multi-window live-apply consistency", () => {
  const get = () => document.documentElement.style.getPropertyValue(CSS_VAR);

  test("a seeded window keeps its colour through a reconnect's null push", () => {
    // Window opened with ?pane=orange; the watch reconnects and re-pushes the
    // library's current colour. A library with no PERSISTED colour pushes null
    // on connect — that must not clobber the valid seed back to blue (Bug A).
    setSearch("?pane=%23f97316");
    applyInitialPaneColor();
    expect(get()).toBe("#f97316");
    applyLivePaneColor(null); // push-on-(re)connect for a no-persisted-colour lib
    expect(get()).toBe("#f97316");
  });

  test("a 2nd window with NO seed is coloured by the watch's push-on-connect", () => {
    // The desktop seed can lag on the 2nd same-library window; the
    // live watch's push-on-connect must still bring it to the library colour
    // (never leave it stuck on the default blue accent).
    setSearch("?t=token"); // no ?pane=
    applyInitialPaneColor();
    expect(get()).toBe(""); // unseeded → default accent for now
    applyLivePaneColor("#22c55e"); // watch delivers the library colour
    expect(get()).toBe("#22c55e");
  });

  test("a reconnect storm of null pushes never reverts a seeded colour", () => {
    setSearch("?pane=%23f97316");
    applyInitialPaneColor();
    for (let i = 0; i < 5; i += 1) applyLivePaneColor(null);
    expect(get()).toBe("#f97316");
  });

  test("a later live colour change recolours an already-seeded window", () => {
    setSearch("?pane=%23f97316");
    applyInitialPaneColor();
    applyLivePaneColor("#22c55e"); // another window picked green
    expect(get()).toBe("#22c55e");
  });

  test("an explicit blue preset push IS applied (blue is a chosen colour, not a clear)", () => {
    // Picking the blue preset persists `#388bfd` (a valid hex), so the watch
    // pushes the hex — distinct from the null "no colour set" frame. The window
    // must show the chosen blue, proving null-no-clobber doesn't swallow a real
    // blue choice.
    setSearch("?pane=%23f97316");
    applyInitialPaneColor();
    applyLivePaneColor(NAMED_PANE_HEX.blue);
    expect(get()).toBe("#388bfd");
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

// The live watch must sync the focus-colour menu (and thus
// new split panes' `data-focus-color`) to a pushed colour, not just the CSS var
// — else the checkmark + a fresh split disagree with the recoloured border.
describe("syncLiveFocusColorMenu (live watch → menu)", () => {
  test("selects the named preset for a pushed preset hex", () => {
    const setColor = vi.fn();
    syncLiveFocusColorMenu("#f97316", setColor);
    expect(setColor).toHaveBeenCalledWith("orange");
    expect(setColor).toHaveBeenCalledTimes(1);
  });
  test("normalizes case/shorthand before matching", () => {
    const setColor = vi.fn();
    syncLiveFocusColorMenu("#388BFD", setColor);
    expect(setColor).toHaveBeenCalledWith("blue");
  });
  test("leaves the menu as-is for a valid but non-preset (custom) hex", () => {
    const setColor = vi.fn();
    syncLiveFocusColorMenu("#abcdef", setColor);
    expect(setColor).not.toHaveBeenCalled();
  });
  test("leaves the menu as-is for a null / invalid push (no clobber)", () => {
    const setColor = vi.fn();
    syncLiveFocusColorMenu(null, setColor);
    syncLiveFocusColorMenu("red; }", setColor);
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
  test("setLocalColor PUTs via requestRoot (ROOT path, not the tenant prefix) — C8", () => {
    // MUST be `requestRoot`, NOT `req`/`request`: the local-color route lives only
    // on the root launcher router, so a window served under a tenant prefix would
    // 404 if the prefix were prepended (`apiPath`). See localColorRootPath.test.ts
    // for the behavioural proof. The window's `?t=` bearer still travels.
    expect(clientSource).toMatch(
      /setLocalColor: \(color: string\) =>[\s\S]*?requestRoot<void>\([\s\S]*?"PUT",[\s\S]*?"\/api\/library\/local-color",[\s\S]*?\{ color \}/,
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
