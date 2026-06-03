import { describe, it, expect } from "vitest";

import { autoAssignSlots } from "./teamDialog.svelte";

describe("autoAssignSlots (E1 team auto-assign)", () => {
  it("spreads all unassigned members one-per-cell when counts match", () => {
    expect(autoAssignSlots([[], [], [], []], 4)).toEqual([[0], [1], [2], [3]]);
  });

  it("fills empty cells first, then balances the overflow", () => {
    // 2 cells, 5 members -> 3/2, lowest index fills first on ties
    expect(autoAssignSlots([[], []], 5)).toEqual([
      [0, 2, 4],
      [1, 3],
    ]);
  });

  it("keeps already-placed members and fills the rest into emptier cells", () => {
    expect(autoAssignSlots([[0], [], []], 3)).toEqual([[0], [1], [2]]);
  });

  it("is pure - does not mutate the input slots", () => {
    const input = [[], []];
    const out = autoAssignSlots(input, 2);
    expect(input).toEqual([[], []]);
    expect(out).not.toBe(input);
  });

  it("returns empty when there are no cells", () => {
    expect(autoAssignSlots([], 3)).toEqual([]);
  });
});
