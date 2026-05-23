import { describe, expect, test } from "vitest";
import orchestrator from "./teamOrchestrator.svelte.ts?raw";
import viteConfig from "../../vite.config.ts?raw";
import {
  placeTeamTemplates,
  templateVarsForWire,
} from "./teamOrchestrator.svelte";
import type { TeamConfigWire } from "../api/client";

// `fullstack-a-79` slice 3: process-template placement. Tests
// pin the vite ?raw bundle path, the template-vars derivation,
// the orchestrator's new step 2 wiring + error handling, and
// the substituted file write path.

describe("fullstack-a-79 slice 3: bundle path + vite fs.allow", () => {
  test("orchestrator imports bootstrap.md.tpl via vite ?raw", () => {
    expect(orchestrator).toMatch(
      /import bootstrapTemplate from "\.\.\/\.\.\/\.\.\/docs\/templates\/team-process\/bootstrap\.md\.tpl\?raw";/,
    );
  });

  test("vite.config.ts grants fs.allow parent for ?raw resolve", () => {
    expect(viteConfig).toMatch(
      /fs: \{[\s\S]{1,200}allow: \[".", "\.\."\],/,
    );
  });
});

describe("fullstack-a-79 slice 3: templateVarsForWire", () => {
  test("derives hostHandle / leadHandle / workerHandles / teamName from the wire config", () => {
    const wire: TeamConfigWire = {
      team_name: "demo",
      host_name: "Alice",
      host_handle: "@@Alice",
      auto_prefix_at: true,
      created_at: "2026-05-23T08:00:00.000Z",
      members: [
        { handle: "@@Lead", command: "claude", env: {}, is_lead: true },
        { handle: "@@Worker1", command: "claude", env: {}, is_lead: false },
        { handle: "@@Worker2", command: "claude", env: {}, is_lead: false },
      ],
    };
    const vars = templateVarsForWire(wire);
    expect(vars.hostHandle).toBe("@@Alice");
    expect(vars.leadHandle).toBe("@@Lead");
    expect(vars.workerHandles).toEqual(["@@Worker1", "@@Worker2"]);
    expect(vars.teamName).toBe("demo");
  });

  test("falls back to host_handle when no member is flagged as lead", () => {
    const wire: TeamConfigWire = {
      team_name: "demo",
      host_name: "Alice",
      host_handle: "@@Alice",
      auto_prefix_at: true,
      created_at: "2026-05-23T08:00:00.000Z",
      members: [
        { handle: "@@Worker1", command: "claude", env: {}, is_lead: false },
      ],
    };
    const vars = templateVarsForWire(wire);
    expect(vars.leadHandle).toBe("@@Alice");
  });

  test("worker order matches the declared member order", () => {
    const wire: TeamConfigWire = {
      team_name: "demo",
      host_name: "Alice",
      host_handle: "@@Alice",
      auto_prefix_at: true,
      created_at: "2026-05-23T08:00:00.000Z",
      members: [
        { handle: "@@WorkerA", command: "claude", env: {}, is_lead: false },
        { handle: "@@Lead", command: "claude", env: {}, is_lead: true },
        { handle: "@@WorkerB", command: "claude", env: {}, is_lead: false },
      ],
    };
    const vars = templateVarsForWire(wire);
    expect(vars.workerHandles).toEqual(["@@WorkerA", "@@WorkerB"]);
  });
});

describe("fullstack-a-79 slice 3: placeTeamTemplates writes to Drafts/team-{name}/docs/", () => {
  test("substituted bootstrap.md lands at Drafts/team-{name}/docs/bootstrap.md via api.create", () => {
    // The helper's shape is pinned via the source pattern;
    // behavioral tests against api.create would need a stub
    // harness which lives in the empirical walks @@WebtestA
    // runs. The pin captures: substituteTeamTemplate result
    // → api.create with the team-relative docs path.
    expect(orchestrator).toMatch(
      /export async function placeTeamTemplates\(wire: TeamConfigWire\): Promise<void> \{[\s\S]{1,1200}const bootstrap = substituteTeamTemplate\(bootstrapTemplate, vars\);[\s\S]{1,600}await api\.create\([\s\S]{1,800}`Drafts\/team-\$\{wire\.team_name\}\/docs\/bootstrap\.md`,[\s\S]{1,200}false,[\s\S]{1,200}bootstrap,/,
    );
  });

  test("placeTeamTemplates is a real function (defensive: ensures the export exists)", () => {
    expect(typeof placeTeamTemplates).toBe("function");
  });
});

describe("fullstack-a-79 slice 3: orchestrator step 2 wiring", () => {
  test("placeTeamTemplates step sits between teamCreate (step 1) and teamLoad (step 3)", () => {
    expect(orchestrator).toMatch(
      /\/\/ 1\. Persist config\.[\s\S]{1,400}await api\.teamCreate\(wire\.team_name, wire\);[\s\S]{1,800}\/\/ 2\. Place process templates\.[\s\S]{1,2000}await placeTeamTemplates\(wire\);[\s\S]{1,800}\/\/ 3\. Load watcher\.[\s\S]{1,200}await api\.teamLoad\(wire\.team_name\);/,
    );
  });

  test("placement failure does NOT bail the chain — notify + continue", () => {
    expect(orchestrator).toMatch(
      /try \{[\s\S]{1,400}await placeTeamTemplates\(wire\);[\s\S]{1,200}\} catch \(err\) \{[\s\S]{1,400}notify\([\s\S]{1,200}Template placement failed/,
    );
  });
});
