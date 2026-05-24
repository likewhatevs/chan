import { describe, expect, test } from "vitest";
import {
  assignMemberToCell,
  closeTeamDialog,
  defaultGridForSize,
  defaultTeamConfig,
  emptySlotsForGrid,
  exportTeamDialogConfig,
  gridShapesForSize,
  importTeamDialogConfig,
  openTeamDialog,
  reshapeSplitGrid,
  resizeTeamMembers,
  switchRealEstate,
  TEAM_MIN_SIZE,
  teamDialogState,
  unassignMember,
  validateTeamConfig,
} from "./teamDialog.svelte";

// `fullstack-a-78` slice 1: TeamDialog state singleton + helpers.
//
// Tests pin the validation contract + the resize semantics
// (lead preservation + automatic Worker-N filling) + the
// open/close bus shape.

describe("fullstack-a-78: defaultTeamConfig", () => {
  test("default config has one lead agent (TEAM_MIN_SIZE)", () => {
    const cfg = defaultTeamConfig();
    expect(cfg.size).toBe(TEAM_MIN_SIZE);
    expect(cfg.members).toHaveLength(1);
    expect(cfg.members.filter((m) => m.isLead)).toHaveLength(1);
    expect(cfg.members[0].isLead).toBe(true);
    expect(cfg.autoPrefix).toBe(true);
    expect(cfg.realEstate).toEqual({ kind: "tabs" });
  });
});

describe("fullstack-a-78: validateTeamConfig", () => {
  test("requires non-empty Your name (per -a-80 slice 2 copy fix matching the field label)", () => {
    const cfg = { ...defaultTeamConfig(), hostName: "" };
    expect(validateTeamConfig(cfg)).toBe("Your name required");
  });

  test("requires non-empty Team name (per -a-80 slice 2 copy fix matching the field label)", () => {
    const cfg = { ...defaultTeamConfig(), hostName: "Alex", teamName: "" };
    expect(validateTeamConfig(cfg)).toBe("Team name required");
  });

  test("rejects size below TEAM_MIN_SIZE", () => {
    const cfg = {
      ...defaultTeamConfig(),
      hostName: "Alex",
      teamName: "a",
      size: 0,
    };
    expect(validateTeamConfig(cfg)).toContain("at least");
  });

  test("rejects size above TEAM_MAX_SIZE", () => {
    const cfg = {
      ...defaultTeamConfig(),
      hostName: "Alex",
      teamName: "a",
      size: 99,
    };
    expect(validateTeamConfig(cfg)).toContain("at most");
  });

  test("requires exactly one lead", () => {
    const noLead = defaultTeamConfig();
    noLead.hostName = "Alex";
    noLead.teamName = "a";
    noLead.members[0].isLead = false;
    expect(validateTeamConfig(noLead)).toBe("one member must be marked as lead");

    const twoLeads = defaultTeamConfig();
    twoLeads.hostName = "Alex";
    twoLeads.teamName = "a";
    twoLeads.size = 2;
    twoLeads.members.push({
      name: "Worker1",
      command: "claude",
      env: "",
      isLead: true,
    });
    expect(validateTeamConfig(twoLeads)).toBe(
      "exactly one member can be marked as lead",
    );
  });

  test("requires every member to have a name", () => {
    const cfg = defaultTeamConfig();
    cfg.hostName = "Alex";
    cfg.teamName = "a";
    cfg.members[0].name = "";
    expect(validateTeamConfig(cfg)).toBe("every member needs a name");
  });

  test("rejects duplicate team name", () => {
    const cfg = { ...defaultTeamConfig(), hostName: "Alex", teamName: "alpha" };
    const existing = new Set(["alpha"]);
    expect(validateTeamConfig(cfg, existing)).toContain("already exists");
  });

  test("returns null for valid config", () => {
    const cfg = { ...defaultTeamConfig(), hostName: "Alex", teamName: "alpha" };
    expect(validateTeamConfig(cfg)).toBeNull();
  });
});

describe("Phase 9 Spawn agents config copy/paste", () => {
  test("export/import round-trips the dialog config", () => {
    const cfg = resizeTeamMembers({
      ...defaultTeamConfig(),
      hostName: "Alex",
      teamName: "alpha",
      size: 2,
    });
    cfg.members[1].command = "codex";
    cfg.members[1].env = "DEBUG=1";

    const imported = importTeamDialogConfig(exportTeamDialogConfig(cfg));

    expect(imported).toEqual(cfg);
  });

  test("import clamps old oversized configs to the Phase 9 maximum", () => {
    const imported = importTeamDialogConfig(
      JSON.stringify({
        hostName: "Alex",
        teamName: "oversized",
        size: 16,
        members: Array.from({ length: 16 }, (_, idx) => ({
          name: idx === 0 ? "Lead" : `Worker${idx}`,
          command: "claude",
          env: "",
          isLead: idx === 0,
        })),
      }),
    );

    expect(imported.size).toBe(9);
    expect(imported.members).toHaveLength(9);
  });

  test("import rejects invalid JSON", () => {
    expect(() => importTeamDialogConfig("{nope")).toThrow(
      /not valid JSON/,
    );
  });
});

describe("fullstack-a-78: resizeTeamMembers", () => {
  test("grow: appends fresh Worker-N entries", () => {
    const cfg = { ...defaultTeamConfig(), size: 4 };
    const out = resizeTeamMembers(cfg);
    expect(out.members).toHaveLength(4);
    expect(out.members[2].name).toBe("Worker2");
    expect(out.members[3].name).toBe("Worker3");
    expect(out.members.filter((m) => m.isLead)).toHaveLength(1);
  });

  test("shrink: truncates from the end", () => {
    const big = defaultTeamConfig();
    big.size = 4;
    const grown = resizeTeamMembers(big);
    // Now shrink back to 2.
    grown.size = 2;
    const shrunk = resizeTeamMembers(grown);
    expect(shrunk.members).toHaveLength(2);
    expect(shrunk.members[0].isLead).toBe(true);
  });

  test("shrink-past-lead: default lead to slot 0", () => {
    const cfg = defaultTeamConfig();
    cfg.size = 3;
    const grown = resizeTeamMembers(cfg);
    // Move lead to slot 2 then shrink to 2.
    grown.members[0].isLead = false;
    grown.members[2].isLead = true;
    grown.size = 2;
    const shrunk = resizeTeamMembers(grown);
    expect(shrunk.members).toHaveLength(2);
    expect(shrunk.members[0].isLead).toBe(true);
  });
});

describe("fullstack-a-78: openTeamDialog / closeTeamDialog bus", () => {
  test("open sets state.request; close clears it", () => {
    expect(teamDialogState.request).toBeNull();
    openTeamDialog({ onBootstrap: () => {} });
    expect(teamDialogState.request).not.toBeNull();
    closeTeamDialog();
    expect(teamDialogState.request).toBeNull();
  });
});

describe("fullstack-a-78 slice 2: gridShapesForSize", () => {
  test("size 2 yields 1×2 and 2×1", () => {
    const shapes = gridShapesForSize(2);
    expect(shapes).toContainEqual({ rows: 1, cols: 2 });
    expect(shapes).toContainEqual({ rows: 2, cols: 1 });
  });

  test("size 4 yields most-balanced 2×2 first", () => {
    const shapes = gridShapesForSize(4);
    expect(shapes[0]).toEqual({ rows: 2, cols: 2 });
    expect(shapes).toContainEqual({ rows: 1, cols: 4 });
    expect(shapes).toContainEqual({ rows: 4, cols: 1 });
  });

  test("size 6 yields 2×3, 3×2, 1×6, 6×1", () => {
    const shapes = gridShapesForSize(6);
    expect(shapes).toContainEqual({ rows: 2, cols: 3 });
    expect(shapes).toContainEqual({ rows: 3, cols: 2 });
    expect(shapes).toContainEqual({ rows: 1, cols: 6 });
    expect(shapes).toContainEqual({ rows: 6, cols: 1 });
  });

  test("prime size 5 still produces shapes that hold ≥5 cells", () => {
    const shapes = gridShapesForSize(5);
    expect(shapes.length).toBeGreaterThan(0);
    for (const s of shapes) {
      expect(s.rows * s.cols).toBeGreaterThanOrEqual(5);
    }
  });
});

describe("fullstack-a-78 slice 2: defaultGridForSize", () => {
  test("default for size 4 is 2×2 (most balanced)", () => {
    expect(defaultGridForSize(4)).toEqual({ rows: 2, cols: 2 });
  });

  test("default for size 6 is 2×3", () => {
    expect(defaultGridForSize(6)).toEqual({ rows: 2, cols: 3 });
  });

  test("default for size 5 falls back to nearest balanced shape", () => {
    const def = defaultGridForSize(5);
    expect(def.rows * def.cols).toBeGreaterThanOrEqual(5);
  });
});

describe("fullstack-a-78 slice 2: switchRealEstate", () => {
  test("tabs → split picks default grid + empty slots", () => {
    const cfg = defaultTeamConfig();
    const next = switchRealEstate(cfg, "split");
    expect(next.realEstate.kind).toBe("split");
    if (next.realEstate.kind === "split") {
      expect(next.realEstate.grid).toEqual(defaultGridForSize(cfg.size));
      expect(next.realEstate.slots).toEqual(emptySlotsForGrid(next.realEstate.grid));
    }
  });

  test("split → tabs drops the grid + slots", () => {
    const cfg = switchRealEstate(defaultTeamConfig(), "split");
    const next = switchRealEstate(cfg, "tabs");
    expect(next.realEstate).toEqual({ kind: "tabs" });
  });

  test("split → split is a no-op (preserves the grid)", () => {
    const cfg = switchRealEstate(defaultTeamConfig(), "split");
    const next = switchRealEstate(cfg, "split");
    expect(next).toBe(cfg);
  });
});

describe("fullstack-a-78 slice 2: reshapeSplitGrid", () => {
  test("re-picks grid + resets slots when reshaping", () => {
    const cfg = switchRealEstate({ ...defaultTeamConfig(), size: 4 }, "split");
    const reshaped = reshapeSplitGrid(cfg, { rows: 1, cols: 4 });
    expect(reshaped.realEstate.kind).toBe("split");
    if (reshaped.realEstate.kind === "split") {
      expect(reshaped.realEstate.grid).toEqual({ rows: 1, cols: 4 });
      expect(reshaped.realEstate.slots).toHaveLength(4);
      expect(reshaped.realEstate.slots.every((c) => c.length === 0)).toBe(true);
    }
  });

  test("no-op when realEstate.kind === 'tabs'", () => {
    const cfg = defaultTeamConfig();
    const reshaped = reshapeSplitGrid(cfg, { rows: 1, cols: 2 });
    expect(reshaped).toBe(cfg);
  });
});

describe("fullstack-a-78 slice 2: assignMemberToCell + unassignMember", () => {
  test("assign places the member in the target cell", () => {
    const cfg = switchRealEstate(defaultTeamConfig(), "split");
    const next = assignMemberToCell(cfg, 0, 0);
    if (next.realEstate.kind === "split") {
      expect(next.realEstate.slots[0]).toEqual([0]);
    }
  });

  test("re-assigning to a different cell removes from the prior cell", () => {
    let cfg = switchRealEstate(
      resizeTeamMembers({ ...defaultTeamConfig(), size: 2 }),
      "split",
    );
    cfg = assignMemberToCell(cfg, 0, 0);
    cfg = assignMemberToCell(cfg, 0, 1);
    if (cfg.realEstate.kind === "split") {
      expect(cfg.realEstate.slots[0]).toEqual([]);
      expect(cfg.realEstate.slots[1]).toEqual([0]);
    }
  });

  test("multiple members in same cell stack as tabs (no replacement)", () => {
    let cfg = switchRealEstate(
      resizeTeamMembers({ ...defaultTeamConfig(), size: 2 }),
      "split",
    );
    cfg = assignMemberToCell(cfg, 0, 0);
    cfg = assignMemberToCell(cfg, 1, 0);
    if (cfg.realEstate.kind === "split") {
      expect(cfg.realEstate.slots[0]).toEqual([0, 1]);
    }
  });

  test("same-member same-cell drop is idempotent", () => {
    let cfg = switchRealEstate(defaultTeamConfig(), "split");
    cfg = assignMemberToCell(cfg, 0, 0);
    cfg = assignMemberToCell(cfg, 0, 0);
    if (cfg.realEstate.kind === "split") {
      expect(cfg.realEstate.slots[0]).toEqual([0]);
    }
  });

  test("unassignMember removes from every cell", () => {
    let cfg = switchRealEstate(defaultTeamConfig(), "split");
    cfg = assignMemberToCell(cfg, 0, 0);
    cfg = unassignMember(cfg, 0);
    if (cfg.realEstate.kind === "split") {
      expect(cfg.realEstate.slots.every((c) => !c.includes(0))).toBe(true);
    }
  });
});

describe("fullstack-a-78 slice 2: resize preserves the split mode", () => {
  test("resize from 2 → 4 keeps split mode + picks new default grid", () => {
    const cfg = switchRealEstate(
      resizeTeamMembers({ ...defaultTeamConfig(), size: 2 }),
      "split",
    );
    const grown = resizeTeamMembers({ ...cfg, size: 4 });
    expect(grown.realEstate.kind).toBe("split");
    if (grown.realEstate.kind === "split") {
      expect(grown.realEstate.grid).toEqual(defaultGridForSize(4));
    }
  });

  test("resize shrink drops slot assignments referencing removed members", () => {
    let cfg = switchRealEstate({ ...defaultTeamConfig(), size: 4 }, "split");
    cfg = resizeTeamMembers(cfg);
    cfg = assignMemberToCell(cfg, 3, 0);
    cfg = resizeTeamMembers({ ...cfg, size: 2 });
    if (cfg.realEstate.kind === "split") {
      for (const cell of cfg.realEstate.slots) {
        expect(cell.includes(3)).toBe(false);
      }
    }
  });
});
