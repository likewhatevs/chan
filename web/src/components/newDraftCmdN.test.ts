import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import client from "../api/client.ts?raw";
import shortcuts from "../state/shortcuts.ts?raw";
import tabs from "../state/tabs.svelte.ts?raw";

// SPA Cmd+N creates a draft via /api/drafts/new and opens it in the
// active pane.

describe("api.createDraft helper", () => {
  test("createDraft posts to /api/drafts/new + returns { path, name }", () => {
    expect(client).toMatch(
      /createDraft: \(\) =>[\s\S]*?req<\{ path: string; name: string \}>\("POST", "\/api\/drafts\/new"\)/,
    );
  });
});

describe("app.draft.new shortcut registry", () => {
  test("app.draft.new bound to Mod+N (web + native)", () => {
    expect(shortcuts).toMatch(
      /id: "app\.draft\.new",[\s\S]*?label: "New draft",[\s\S]*?web: "Mod\+N",[\s\S]*?native: "Mod\+N",/,
    );
  });
});

describe("Cmd+N keymap branch", () => {
  test("App.svelte keymap intercepts bare Mod+N per-OS (Cmd+N mac / Ctrl+N else)", () => {
    expect(app).toMatch(
      /const newDraftChord =[\s\S]{1,80}currentOS\(\) === "mac"[\s\S]{1,120}e\.metaKey && !e\.ctrlKey[\s\S]{1,80}e\.code === "KeyN"[\s\S]{1,120}: e\.ctrlKey && !e\.metaKey[\s\S]{1,80}e\.code === "KeyN";[\s\S]{1,200}void createDraftAndOpen\(\);/,
    );
  });

  test("createDraftAndOpen calls api.createDraft then opens with title selected", () => {
    expect(app).toMatch(
      /async function createDraftAndOpen\(\): Promise<void> \{[\s\S]*?const \{ path \} = await api\.createDraft\(\);[\s\S]*?await noteDraftCreated\(path\);[\s\S]*?await openInActivePane\(path, \{[\s\S]*?initialSelection: NEW_DRAFT_TITLE_SELECTION,/,
    );
  });

  test("createDraftAndOpen surfaces errors via transient status", () => {
    expect(app).toMatch(
      /async function createDraftAndOpen\(\)[\s\S]*?catch \(err\) \{[\s\S]*?console\.warn\([\s\S]*?setTransientStatus\(`New draft failed:/,
    );
  });

  test("staged draft materialization also refreshes Drafts state before opening", () => {
    expect(app).toMatch(
      /const \{ path \} = await api\.createDraft\(\);[\s\S]*?await noteDraftCreated\(path\);[\s\S]*?await openInPane\(entry\.paneId, path, \{[\s\S]*?initialSelection: NEW_DRAFT_TITLE_SELECTION,/,
    );
  });

  test("new draft selection spans the seeded Draft heading text", () => {
    expect(app).toMatch(
      /const NEW_DRAFT_TITLE_SELECTION = \{[\s\S]*?from: "# "\.length,[\s\S]*?to: "# Draft"\.length,/,
    );
  });

  test("openInPane applies optional initial selection to new tabs", () => {
    expect(tabs).toMatch(
      /type OpenFileOptions = \{[\s\S]*?initialSelection\?: EditorSelection;/,
    );
    expect(tabs).toMatch(
      /if \(opts\.initialSelection\) newTab\.caret = \{ \.\.\.opts\.initialSelection \};/,
    );
  });

  test("api imported from api/client", () => {
    expect(app).toMatch(/import \{ api \} from "\.\/api\/client";/);
  });
});
