import { describe, expect, test } from "vitest";
import {
  identityPrompt,
  memberHandle,
  parseEnvLines,
  translateConfig,
} from "./teamOrchestrator.svelte";
import type { TeamDialogConfig } from "./teamDialog.svelte";

// `fullstack-a-79` slice 1: orchestrator translation +
// helpers. The full bootstrap chain (api.teamCreate → teamLoad
// → spawnTerminal per worker → notify) is wired in
// `runTeamBootstrap` and tested via integration paths;
// here we pin the pure translators.

describe("fullstack-a-79: parseEnvLines", () => {
  test("parses KEY=value lines into a Record", () => {
    expect(parseEnvLines("FOO=bar\nBAZ=qux")).toEqual({
      FOO: "bar",
      BAZ: "qux",
    });
  });

  test("skips empty lines + trims whitespace", () => {
    expect(parseEnvLines("\n  FOO=bar  \n\nBAZ=qux\n")).toEqual({
      FOO: "bar",
      BAZ: "qux",
    });
  });

  test("preserves values containing equals signs", () => {
    expect(parseEnvLines("URL=https://example.com?x=1&y=2")).toEqual({
      URL: "https://example.com?x=1&y=2",
    });
  });

  test("throws on line without an `=`", () => {
    expect(() => parseEnvLines("INVALID")).toThrow(/env line 1 must be KEY=value/);
  });

  test("throws on invalid key (starts with digit)", () => {
    expect(() => parseEnvLines("1FOO=bar")).toThrow(/invalid key/);
  });

  test("empty text returns empty record", () => {
    expect(parseEnvLines("")).toEqual({});
  });
});

describe("fullstack-a-79: memberHandle", () => {
  test("auto-prefixes with @@ when autoPrefix is on", () => {
    expect(
      memberHandle(
        { name: "Lead", command: "claude", env: "", isLead: true },
        true,
      ),
    ).toBe("@@Lead");
  });

  test("returns raw name when autoPrefix is off", () => {
    expect(
      memberHandle(
        { name: "Lead", command: "claude", env: "", isLead: true },
        false,
      ),
    ).toBe("Lead");
  });

  test("skips double-prefix when name already starts with @@", () => {
    expect(
      memberHandle(
        { name: "@@Lead", command: "claude", env: "", isLead: true },
        true,
      ),
    ).toBe("@@Lead");
  });
});

describe("fullstack-a-79: translateConfig", () => {
  function sample(overrides: Partial<TeamDialogConfig> = {}): TeamDialogConfig {
    return {
      hostName: "Alice",
      teamName: "demo",
      size: 2,
      autoPrefix: true,
      members: [
        { name: "Lead", command: "claude", env: "", isLead: true },
        { name: "Worker1", command: "claude", env: "", isLead: false },
      ],
      realEstate: { kind: "tabs" },
      ...overrides,
    };
  }

  test("maps camelCase → snake_case shape (team_name, host_name, host_handle, auto_prefix_at, created_at, members)", () => {
    const out = translateConfig(sample());
    expect(out.team_name).toBe("demo");
    expect(out.host_name).toBe("Alice");
    expect(out.host_handle).toBe("@@Alice");
    expect(out.auto_prefix_at).toBe(true);
    expect(typeof out.created_at).toBe("string");
    expect(out.members.length).toBe(2);
  });

  test("each member carries handle / command / env / is_lead in snake_case", () => {
    const out = translateConfig(sample());
    expect(out.members[0]).toMatchObject({
      handle: "@@Lead",
      command: "claude",
      is_lead: true,
    });
    expect(out.members[1]).toMatchObject({
      handle: "@@Worker1",
      command: "claude",
      is_lead: false,
    });
  });

  test("auto-injects CHAN_TAB_NAME=<handle> when env doesn't already carry it", () => {
    const out = translateConfig(sample());
    expect(out.members[0].env.CHAN_TAB_NAME).toBe("@@Lead");
    expect(out.members[1].env.CHAN_TAB_NAME).toBe("@@Worker1");
  });

  test("preserves user-supplied CHAN_TAB_NAME override", () => {
    const out = translateConfig(
      sample({
        members: [
          {
            name: "Lead",
            command: "claude",
            env: "CHAN_TAB_NAME=Custom",
            isLead: true,
          },
          {
            name: "Worker1",
            command: "claude",
            env: "",
            isLead: false,
          },
        ],
      }),
    );
    expect(out.members[0].env.CHAN_TAB_NAME).toBe("Custom");
  });

  test("created_at is ISO 8601 UTC", () => {
    const out = translateConfig(sample());
    expect(out.created_at).toMatch(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z/);
  });
});

describe("fullstack-a-79: identityPrompt", () => {
  test("constructs the addendum-a 2026-05-23 prompt verbatim with host/lead asymmetry", () => {
    expect(
      identityPrompt(
        "@@Alice",
        "@@Lead",
        "Drafts/team-foo/docs/bootstrap.md",
      ),
    ).toBe(
      "Hello, I am @@Alice and you are $CHAN_TAB_NAME. Our team lead is @@Lead. Identify yourself and read Drafts/team-foo/docs/bootstrap.md with the chan MCP read_file tool (a Drafts/ path is a chan workspace location, not a file under your working directory).",
    );
  });

  test("Drafts bootstrap paths are routed through the chan MCP read_file tool", () => {
    const out = identityPrompt(
      "@@Alice",
      "@@Lead",
      "Drafts/team-foo/docs/bootstrap.md",
    );
    expect(out).toContain("chan MCP read_file tool");
  });

  test("plain workspace paths get no MCP hint (they resolve relative to cwd)", () => {
    const out = identityPrompt("@@Alice", "@@Lead", "notes/bootstrap.md");
    expect(out).toBe(
      "Hello, I am @@Alice and you are $CHAN_TAB_NAME. Our team lead is @@Lead. Identify yourself and read notes/bootstrap.md.",
    );
    expect(out).not.toContain("read_file");
  });

  test("does NOT escape $CHAN_TAB_NAME (agents read it as a live env-var)", () => {
    const out = identityPrompt(
      "@@Bob",
      "@@Lead",
      "Drafts/team-bar/docs/bootstrap.md",
    );
    expect(out).toContain("$CHAN_TAB_NAME");
    expect(out).not.toContain("\\$CHAN_TAB_NAME");
  });
});
