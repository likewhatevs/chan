import { describe, expect, test } from "vitest";
import tree from "./FileTree.svelte?raw";

// `fullstack-a-80` slice 1: FB team-dir badge + Load Team
// right-click entry + Duplicate flow on the already-loaded
// branch. Tests pin the architectural shape; behavioral
// coverage of the load/duplicate round-trips lands when
// @@WebtestA walks against a populated drive.

describe("fullstack-a-80 slice 1: team-dir detection", () => {
  test("TEAM_DIR_RE matches Drafts/team-{name}", () => {
    expect(tree).toMatch(/const TEAM_DIR_RE = \/\^Drafts\\\/team-\(\[\^\/\]\+\)\$\//);
  });

  test("teamNameFromPath extracts the {name} group", () => {
    expect(tree).toMatch(
      /function teamNameFromPath\(path: string\): string \| null \{[\s\S]{1,400}const match = TEAM_DIR_RE\.exec\(path\);[\s\S]{1,200}return match \? match\[1\] : null;/,
    );
  });

  test("isTeamDir piggy-backs on teamNameFromPath", () => {
    expect(tree).toMatch(
      /function isTeamDir\(path: string\): boolean \{[\s\S]{1,200}return teamNameFromPath\(path\) !== null;/,
    );
  });
});

describe("fullstack-a-80 slice 1: team-badge in the tree", () => {
  test("Users icon renders for team dirs (overrides Folder)", () => {
    expect(tree).toMatch(
      /\{#if isTeamDir\(node\.path\)\}[\s\S]{1,600}<Users size=\{14\}/,
    );
  });

  test("Users icon imported from lucide-svelte", () => {
    expect(tree).toMatch(/Users,/);
  });
});

describe("fullstack-a-80 slice 1: Load Team menu entry", () => {
  test("entry gated on menu.isDir && isTeamDir(menu.path)", () => {
    expect(tree).toMatch(
      /\{#if menu\.isDir && isTeamDir\(menu\.path\)\}[\s\S]{1,800}onclick=\{\(\) => void loadTeamFromMenu\(menu!\.path\)\}[\s\S]{1,400}<span>Load Team<\/span>/,
    );
  });

  test("Play icon imported (Load Team affordance)", () => {
    expect(tree).toMatch(/Play,/);
  });
});

describe("fullstack-a-80 slice 1: loadTeamFromMenu handler", () => {
  test("walks teamListLoaded first", () => {
    expect(tree).toMatch(
      /async function loadTeamFromMenu\(path: string\): Promise<void> \{[\s\S]{1,1000}const \{ teams \} = await api\.teamListLoaded\(\);/,
    );
  });

  test("already-loaded branch: notify + uiPrompt + teamDuplicate", () => {
    expect(tree).toMatch(
      /if \(teams\.includes\(name\)\) \{[\s\S]{1,1200}const newName = await uiPrompt\([\s\S]{1,400}await api\.teamDuplicate\(name, trimmed\);[\s\S]{1,400}notify\(/,
    );
  });

  test("not-loaded branch (slice 2): teamGetConfig + openTeamDialog with wireToDialog(...) initial", () => {
    // `fullstack-a-80` slice 2 replaced the slice-1
    // teamLoad-and-notify placeholder with the real
    // dialog-from-config flow. The handler now reads the
    // persisted TeamConfig, translates it back to the SPA
    // shape, and opens the global team dialog. Bootstrap
    // runs the standard `-a-79` orchestrator chain.
    expect(tree).toMatch(
      /const wire = await api\.teamGetConfig\(name\);[\s\S]{1,400}const initial = wireToDialog\(wire\);[\s\S]{1,400}openTeamDialog\(\{[\s\S]{1,800}initial,/,
    );
  });

  test("api.teamListLoaded + teamLoad + teamDuplicate all reachable via the api import", () => {
    expect(tree).toMatch(/import \{ api \} from "\.\.\/api\/client";/);
  });
});
