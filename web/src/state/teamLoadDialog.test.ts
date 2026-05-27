import { describe, expect, test } from "vitest";
import orchestrator from "./teamOrchestrator.svelte.ts?raw";
import tree from "../components/FileTree.svelte?raw";
import client from "../api/client.ts?raw";
import { wireToDialog } from "./teamOrchestrator.svelte";
import type { TeamConfigWire } from "../api/client";

// `fullstack-a-80` slice 2: unblocked by `systacean-42`'s
// `GET /api/teams/:name/config`. Tests pin the wire→dialog
// translator + the FB Load Team handler's pivot from notify-
// only to a real dialog populated from the persisted config.

describe("fullstack-a-80 slice 2: api client", () => {
  test("teamGetConfig GETs /api/teams/{name}/config", () => {
    expect(client).toMatch(
      /teamGetConfig: \(name: string\) =>[\s\S]{1,400}`\/api\/teams\/\$\{encodeURIComponent\(name\)\}\/config`/,
    );
  });
});

describe("fullstack-a-80 slice 2: wireToDialog translator", () => {
  test("inverse of translateConfig: snake_case → camelCase round-trip", () => {
    const wire: TeamConfigWire = {
      team_name: "demo",
      host_name: "Alice",
      host_handle: "@@Alice",
      auto_prefix_at: true,
      created_at: "2026-05-23T08:00:00.000Z",
      members: [
        {
          handle: "@@Lead",
          command: "claude",
          env: { CHAN_TAB_NAME: "@@Lead", FOO: "bar" },
          is_lead: true,
        },
        {
          handle: "@@Worker1",
          command: "claude",
          env: { CHAN_TAB_NAME: "@@Worker1" },
          is_lead: false,
        },
      ],
    };
    const dialog = wireToDialog(wire);
    expect(dialog.hostName).toBe("Alice");
    expect(dialog.teamName).toBe("demo");
    expect(dialog.autoPrefix).toBe(true);
    expect(dialog.size).toBe(2);
    expect(dialog.members).toEqual([
      { name: "@@Lead", command: "claude", env: "FOO=bar", isLead: true },
      { name: "@@Worker1", command: "claude", env: "", isLead: false },
    ]);
    // Real estate defaults to tabs — chan-workspace's Member
    // doesn't persist a real-estate field today.
    expect(dialog.realEstate).toEqual({ kind: "tabs" });
  });

  test("CHAN_TAB_NAME is stripped from the visible env field (auto-injected on submit)", () => {
    const wire: TeamConfigWire = {
      team_name: "demo",
      host_name: "Alice",
      host_handle: "@@Alice",
      auto_prefix_at: true,
      created_at: "2026-05-23T08:00:00.000Z",
      members: [
        {
          handle: "@@Worker1",
          command: "claude",
          env: { CHAN_TAB_NAME: "@@Worker1", DEBUG: "1" },
          is_lead: false,
        },
      ],
    };
    const dialog = wireToDialog(wire);
    expect(dialog.members[0].env).toBe("DEBUG=1");
    expect(dialog.members[0].env).not.toContain("CHAN_TAB_NAME");
  });

  test("env Record → KEY=VALUE\\n string with one entry per line", () => {
    const wire: TeamConfigWire = {
      team_name: "demo",
      host_name: "Alice",
      host_handle: "@@Alice",
      auto_prefix_at: true,
      created_at: "2026-05-23T08:00:00.000Z",
      members: [
        {
          handle: "@@Worker1",
          command: "claude",
          env: { ALPHA: "1", BETA: "2", GAMMA: "3" },
          is_lead: false,
        },
      ],
    };
    const dialog = wireToDialog(wire);
    const lines = dialog.members[0].env.split("\n").sort();
    expect(lines).toEqual(["ALPHA=1", "BETA=2", "GAMMA=3"]);
  });

  test("preserves auto_prefix_at when false", () => {
    const wire: TeamConfigWire = {
      team_name: "demo",
      host_name: "Alice",
      host_handle: "Alice",
      auto_prefix_at: false,
      created_at: "2026-05-23T08:00:00.000Z",
      members: [
        {
          handle: "Lead",
          command: "claude",
          env: {},
          is_lead: true,
        },
      ],
    };
    const dialog = wireToDialog(wire);
    expect(dialog.autoPrefix).toBe(false);
  });
});

describe("fullstack-a-80 slice 2: FileTree loadTeamFromMenu populates dialog from config", () => {
  test("not-loaded branch reads config + opens dialog with initial: wireToDialog(...)", () => {
    expect(tree).toMatch(
      /const wire = await api\.teamGetConfig\(name\);[\s\S]{1,400}const initial = wireToDialog\(wire\);[\s\S]{1,400}openTeamDialog\(\{[\s\S]{1,800}initial,/,
    );
  });

  test("dialog onBootstrap runs runTeamBootstrap (orchestrator chain)", () => {
    expect(tree).toMatch(
      /openTeamDialog\(\{[\s\S]{1,2000}onBootstrap: async \(config\) => \{[\s\S]{1,400}await runTeamBootstrap\(config\);/,
    );
  });

  test("imports openTeamDialog + wireToDialog + runTeamBootstrap", () => {
    expect(tree).toMatch(
      /import \{ openTeamDialog \} from "\.\.\/state\/teamDialog\.svelte";/,
    );
    expect(tree).toMatch(
      /import \{[\s\S]{1,200}runTeamBootstrap,[\s\S]{1,200}wireToDialog,[\s\S]{1,40}\} from "\.\.\/state\/teamOrchestrator\.svelte";/,
    );
  });

  test("already-loaded branch unchanged (notify + duplicate prompt)", () => {
    expect(tree).toMatch(
      /if \(teams\.includes\(name\)\) \{[\s\S]{1,1600}await api\.teamDuplicate\(name, trimmed\);/,
    );
  });
});

describe("fullstack-a-80 slice 2: orchestrator gates kept", () => {
  test("wireToDialog default realEstate: tabs (chan-workspace doesn't persist real-estate)", () => {
    expect(orchestrator).toMatch(
      /realEstate: \{ kind: "tabs" \},/,
    );
  });
});
