import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import client from "../api/client.ts?raw";
import shortcuts from "../state/shortcuts.ts?raw";

// `fullstack-a-66` slice 1: SPA Cmd+N → /api/drafts/new →
// open in active pane.

describe("fullstack-a-66 slice 1: api.createDraft helper", () => {
  test("createDraft posts to /api/drafts/new + returns { path, name }", () => {
    expect(client).toMatch(
      /createDraft: \(\) =>[\s\S]*?req<\{ path: string; name: string \}>\("POST", "\/api\/drafts\/new"\)/,
    );
  });
});

describe("fullstack-a-66 slice 1: app.draft.new shortcut registry", () => {
  test("app.draft.new bound to Mod+N (web + native)", () => {
    expect(shortcuts).toMatch(
      /id: "app\.draft\.new",[\s\S]*?label: "New draft",[\s\S]*?web: "Mod\+N",[\s\S]*?native: "Mod\+N",/,
    );
  });
});

describe("fullstack-a-66 slice 1: Cmd+N keymap branch", () => {
  test("App.svelte keymap intercepts bare Cmd+N (no shift/alt/ctrl)", () => {
    expect(app).toMatch(
      /if \(meta && !e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyN"\) \{[\s\S]*?e\.preventDefault\(\);[\s\S]*?void createDraftAndOpen\(\);/,
    );
  });

  test("createDraftAndOpen calls api.createDraft then openInActivePane", () => {
    expect(app).toMatch(
      /async function createDraftAndOpen\(\): Promise<void> \{[\s\S]*?const \{ path \} = await api\.createDraft\(\);[\s\S]*?await openInActivePane\(path\);/,
    );
  });

  test("createDraftAndOpen swallows errors via try/catch (UX: don't blow up on failed draft creation)", () => {
    expect(app).toMatch(
      /async function createDraftAndOpen\(\)[\s\S]*?try \{[\s\S]*?\} catch \(err\) \{[\s\S]*?console\.warn\(/,
    );
  });

  test("api imported from api/client", () => {
    expect(app).toMatch(/import \{ api \} from "\.\/api\/client";/);
  });
});
