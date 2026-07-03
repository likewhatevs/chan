// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import {
  closeConfirmState,
  resolveCloseConfirm,
  uiCloseConfirm,
} from "./closeConfirm.svelte";

afterEach(() => {
  // Drop any resolver a test left pending so it can't leak into the next one.
  resolveCloseConfirm("cancel");
});

describe("closeConfirm", () => {
  it("opens and resolves the chosen action", async () => {
    for (const choice of ["hide", "close", "cancel"] as const) {
      const p = uiCloseConfirm();
      expect(closeConfirmState.open).toBe(true);
      resolveCloseConfirm(choice);
      expect(closeConfirmState.open).toBe(false);
      expect(closeConfirmState.resolve).toBeNull();
      await expect(p).resolves.toBe(choice);
    }
  });

  it("a second open resolves the prior prompt as a cancel", async () => {
    const first = uiCloseConfirm();
    const second = uiCloseConfirm();
    // The superseded prompt resolves cancel (its window stayed open); the second
    // is now the live one.
    await expect(first).resolves.toBe("cancel");
    expect(closeConfirmState.open).toBe(true);
    resolveCloseConfirm("hide");
    await expect(second).resolves.toBe("hide");
  });

  it("resolving with no pending prompt is a no-op", () => {
    expect(() => resolveCloseConfirm("cancel")).not.toThrow();
    expect(closeConfirmState.open).toBe(false);
  });
});
