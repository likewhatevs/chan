import { describe, it, expect } from "vitest";
import { capabilitiesFor, parseSurface } from "./capabilities";

describe("capabilitiesFor", () => {
  it("desktop can mutate and has a bridge, not self-managed", () => {
    expect(capabilitiesFor("desktop")).toEqual({
      canMutateRegistry: true,
      hasDesktopBridge: true,
      selfManagedWindows: false,
    });
  });

  it("devserver can mutate and self-manages, no bridge", () => {
    expect(capabilitiesFor("devserver")).toEqual({
      canMutateRegistry: true,
      hasDesktopBridge: false,
      selfManagedWindows: true,
    });
  });

  it("readonly has no capability", () => {
    expect(capabilitiesFor("readonly")).toEqual({
      canMutateRegistry: false,
      hasDesktopBridge: false,
      selfManagedWindows: false,
    });
  });
});

describe("parseSurface", () => {
  it("takes the descriptor value when valid", () => {
    expect(parseSurface("desktop", false)).toBe("desktop");
    expect(parseSurface("devserver", false)).toBe("devserver");
    expect(parseSurface("readonly", false)).toBe("readonly");
  });

  it("falls back to the legacy readonly meta when the descriptor is absent", () => {
    expect(parseSurface(null, true)).toBe("readonly");
  });

  it("defaults to desktop with no descriptor and no legacy meta", () => {
    expect(parseSurface(null, false)).toBe("desktop");
  });

  it("ignores an unrecognized descriptor value, falling back", () => {
    expect(parseSurface("bogus", true)).toBe("readonly");
    expect(parseSurface("bogus", false)).toBe("desktop");
  });
});
