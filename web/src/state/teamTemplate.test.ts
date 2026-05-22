import { describe, expect, test } from "vitest";
import {
  CHAN_INTERNAL_TEAM_VARS,
  substituteTeamTemplate,
} from "./teamTemplate";

// `fullstack-a-81` slice 1: template substitution helper for
// `-a-79`'s bootstrap orchestrator. Replaces {host-handle} /
// {lead-handle} / {worker-N-handle} / {team-name} tokens.

describe("fullstack-a-81: substituteTeamTemplate", () => {
  test("substitutes {host-handle} + {lead-handle} + {team-name}", () => {
    const out = substituteTeamTemplate(
      "Host is {host-handle}, lead is {lead-handle}, team is {team-name}.",
      {
        hostHandle: "@@Host",
        leadHandle: "@@Lead",
        workerHandles: [],
        teamName: "team-alpha",
      },
    );
    expect(out).toBe("Host is @@Host, lead is @@Lead, team is team-alpha.");
  });

  test("substitutes {worker-N-handle} for the Nth (1-indexed) worker", () => {
    const out = substituteTeamTemplate(
      "W1={worker-1-handle} W2={worker-2-handle} W3={worker-3-handle}",
      {
        hostHandle: "@@H",
        leadHandle: "@@L",
        workerHandles: ["@@A", "@@B", "@@C"],
      },
    );
    expect(out).toBe("W1=@@A W2=@@B W3=@@C");
  });

  test("preserves {worker-N-handle} when N exceeds the worker list", () => {
    const out = substituteTeamTemplate(
      "W1={worker-1-handle} W5={worker-5-handle}",
      {
        hostHandle: "@@H",
        leadHandle: "@@L",
        workerHandles: ["@@A"],
      },
    );
    expect(out).toBe("W1=@@A W5={worker-5-handle}");
  });

  test("defaults {team-name} to 'team' when unset", () => {
    const out = substituteTeamTemplate("Team is {team-name}.", {
      hostHandle: "@@H",
      leadHandle: "@@L",
      workerHandles: [],
    });
    expect(out).toBe("Team is team.");
  });

  test("leaves unknown {placeholder} tokens as-is (audit-friendly)", () => {
    const out = substituteTeamTemplate(
      "Known: {host-handle}; unknown: {phase-label}; CamelCase: {HostHandle}",
      {
        hostHandle: "@@H",
        leadHandle: "@@L",
        workerHandles: [],
      },
    );
    expect(out).toBe(
      "Known: @@H; unknown: {phase-label}; CamelCase: {HostHandle}",
    );
  });

  test("handles repeated tokens in a single template", () => {
    const out = substituteTeamTemplate(
      "{host-handle} says: {host-handle} is the host. {lead-handle} leads {worker-1-handle}.",
      {
        hostHandle: "@@H",
        leadHandle: "@@L",
        workerHandles: ["@@A"],
      },
    );
    expect(out).toBe(
      "@@H says: @@H is the host. @@L leads @@A.",
    );
  });
});

describe("fullstack-a-81: CHAN_INTERNAL_TEAM_VARS", () => {
  test("chan-internal vars map to canonical chan handles", () => {
    expect(CHAN_INTERNAL_TEAM_VARS.hostHandle).toBe("@@Alex");
    expect(CHAN_INTERNAL_TEAM_VARS.leadHandle).toBe("@@Architect");
    expect(CHAN_INTERNAL_TEAM_VARS.workerHandles).toEqual([
      "@@FullStackA",
      "@@FullStackB",
      "@@Systacean",
      "@@CI",
      "@@WebtestA",
      "@@WebtestB",
    ]);
    expect(CHAN_INTERNAL_TEAM_VARS.teamName).toBe("chan");
  });

  test("substituting the chan-internal vars renders the chan-canonical handles", () => {
    const tpl = "Host: {host-handle}; Lead: {lead-handle}; W1: {worker-1-handle}; W6: {worker-6-handle}";
    const out = substituteTeamTemplate(tpl, CHAN_INTERNAL_TEAM_VARS);
    expect(out).toBe(
      "Host: @@Alex; Lead: @@Architect; W1: @@FullStackA; W6: @@WebtestB",
    );
  });
});

describe("fullstack-a-81 slice 4: phase-slug substitution", () => {
  test("substitutes {phase-slug} from vars.phaseSlug", () => {
    const out = substituteTeamTemplate(
      "Working dir: docs/journals/{phase-slug}/",
      {
        hostHandle: "@@H",
        leadHandle: "@@L",
        workerHandles: [],
        teamName: "team-alpha",
        phaseSlug: "phase-2",
      },
    );
    expect(out).toBe("Working dir: docs/journals/phase-2/");
  });

  test("defaults {phase-slug} to `phase-1` when unset (new-team friendly)", () => {
    const out = substituteTeamTemplate(
      "Working dir: docs/journals/{phase-slug}/",
      {
        hostHandle: "@@H",
        leadHandle: "@@L",
        workerHandles: [],
      },
    );
    expect(out).toBe("Working dir: docs/journals/phase-1/");
  });

  test("CHAN_INTERNAL_TEAM_VARS carries phaseSlug='phase-8'", () => {
    expect(CHAN_INTERNAL_TEAM_VARS.phaseSlug).toBe("phase-8");
  });

  test("chan-internal substitution renders bootstrap-style paths as `phase-8`", () => {
    const tpl = "INBOUND: docs/journals/{phase-slug}/alex/event-x.md";
    const out = substituteTeamTemplate(tpl, CHAN_INTERNAL_TEAM_VARS);
    expect(out).toBe("INBOUND: docs/journals/phase-8/alex/event-x.md");
  });
});
