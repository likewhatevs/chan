import { describe, expect, test } from "vitest";
import {
  closeTeamDialog,
  defaultTeamConfig,
  openTeamDialog,
  resizeTeamMembers,
  TEAM_MAX_SIZE,
  TEAM_MIN_SIZE,
  teamDialogState,
  validateTeamConfig,
} from "./teamDialog.svelte";

// `fullstack-a-78` slice 1: TeamDialog state singleton + helpers.
//
// Tests pin the validation contract + the resize semantics
// (lead preservation + automatic Worker-N filling) + the
// open/close bus shape.

describe("fullstack-a-78: defaultTeamConfig", () => {
  test("default config has lead + 1 worker (TEAM_MIN_SIZE)", () => {
    const cfg = defaultTeamConfig();
    expect(cfg.size).toBe(TEAM_MIN_SIZE);
    expect(cfg.members).toHaveLength(2);
    expect(cfg.members.filter((m) => m.isLead)).toHaveLength(1);
    expect(cfg.members[0].isLead).toBe(true);
    expect(cfg.autoPrefix).toBe(true);
    expect(cfg.realEstate).toEqual({ kind: "tabs" });
  });
});

describe("fullstack-a-78: validateTeamConfig", () => {
  test("requires non-empty host name", () => {
    const cfg = { ...defaultTeamConfig(), hostName: "" };
    expect(validateTeamConfig(cfg)).toBe("host name required");
  });

  test("requires non-empty team name", () => {
    const cfg = { ...defaultTeamConfig(), hostName: "Alex", teamName: "" };
    expect(validateTeamConfig(cfg)).toBe("team name required");
  });

  test("rejects size below TEAM_MIN_SIZE", () => {
    const cfg = {
      ...defaultTeamConfig(),
      hostName: "Alex",
      teamName: "a",
      size: 1,
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
    twoLeads.members[1].isLead = true;
    expect(validateTeamConfig(twoLeads)).toBe(
      "exactly one member can be marked as lead",
    );
  });

  test("requires every member to have a name", () => {
    const cfg = defaultTeamConfig();
    cfg.hostName = "Alex";
    cfg.teamName = "a";
    cfg.members[1].name = "";
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
