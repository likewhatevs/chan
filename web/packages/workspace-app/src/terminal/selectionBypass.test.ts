import { describe, expect, test } from "vitest";
import { Terminal } from "@xterm/xterm";
import {
  forceSelectionForShift,
  installShiftSelectionBypass,
} from "./selectionBypass";

describe("forceSelectionForShift", () => {
  test("forces a selection whenever Shift is held", () => {
    expect(forceSelectionForShift({ shiftKey: true })).toBe(true);
    expect(forceSelectionForShift({ shiftKey: false })).toBe(false);
  });
});

describe("installShiftSelectionBypass", () => {
  // Mock shaped like the internal path the bypass wraps, with a macOS-style
  // default that ignores Shift -- the exact case the A1 fix repairs.
  test("adds Shift while preserving the platform default (macOS Option)", () => {
    const macDefault = (e: MouseEvent) => e.altKey; // Shift ignored, Option forces
    const selection = { shouldForceSelection: macDefault };
    const term = { _core: { _selectionService: selection } } as unknown as Terminal;
    installShiftSelectionBypass(term);

    expect(selection.shouldForceSelection({ shiftKey: true, altKey: false } as MouseEvent)).toBe(true);
    expect(selection.shouldForceSelection({ shiftKey: false, altKey: true } as MouseEvent)).toBe(true);
    expect(selection.shouldForceSelection({ shiftKey: false, altKey: false } as MouseEvent)).toBe(false);
  });

  test("no-op when the internal selection service is absent", () => {
    const term = {} as unknown as Terminal;
    expect(() => installShiftSelectionBypass(term)).not.toThrow();
  });

  // Real-Terminal test: proves the bypass resolves the ACTUAL internal path
  // (`_core._selectionService.shouldForceSelection`) on an opened xterm.js v6
  // and flips its behavior -- closing the mock/reality gap. jsdom lacks
  // `matchMedia` (xterm's renderer reads it during open), so we stub it.
  test("wires the real opened xterm.js SelectionService", () => {
    (window as { matchMedia?: unknown }).matchMedia ??= (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener() {},
      removeEventListener() {},
      addListener() {},
      removeListener() {},
      dispatchEvent: () => false,
    });
    const host = document.createElement("div");
    document.body.appendChild(host);
    const term = new Terminal();
    term.open(host);
    const selection = (term as unknown as {
      _core: { _selectionService: { shouldForceSelection: (e: MouseEvent) => boolean } };
    })._core._selectionService;

    // jsdom is detected as non-mac, so the default is `e.shiftKey` already; to
    // prove the wrap actually replaced the method we install over a stub that
    // ignores Shift and confirm Shift now forces selection.
    selection.shouldForceSelection = (e: MouseEvent) => e.altKey; // pretend macOS
    installShiftSelectionBypass(term);
    expect(selection.shouldForceSelection({ shiftKey: true, altKey: false } as MouseEvent)).toBe(true);
    expect(selection.shouldForceSelection({ shiftKey: false, altKey: false } as MouseEvent)).toBe(false);

    term.dispose();
    host.remove();
  });
});
