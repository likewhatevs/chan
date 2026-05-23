import { describe, expect, test } from "vitest";
import shortcuts from "../state/shortcuts.ts?raw";
import app from "../App.svelte?raw";
import tabs from "../state/tabs.svelte.ts?raw";
import editor from "./FileEditorTab.svelte?raw";

// `fullstack-a-67f` slice 2: Obsidian-style Mod+E "Show Source
// Code" chord. Pin: registry entry + the keymap handler in
// App.svelte + the runCommand branch + the store-side
// toggleActiveFileTabMode helper + the chord-hint surface in
// the editor menu.

describe("fullstack-a-67f slice 2: shortcut registry entry", () => {
  test("app.editor.toggleMode entry exists with Mod+E (web + native)", () => {
    expect(shortcuts).toMatch(
      /id: "app\.editor\.toggleMode",[\s\S]{1,200}label: "Show Source Code \(toggle rendered\/source\)",[\s\S]{1,200}native: "Mod\+E",[\s\S]{1,80}web: "Mod\+E",/,
    );
  });

  test("entry sits in the Editor group + escapes terminal", () => {
    expect(shortcuts).toMatch(
      /id: "app\.editor\.toggleMode",[\s\S]{1,600}group: "Editor",[\s\S]{1,80}escapeTerminal: true,/,
    );
  });

  test("Editor group added to the ShortcutGroup union", () => {
    expect(shortcuts).toMatch(/export type ShortcutGroup =[\s\S]{1,200}\| "Editor";/);
  });
});

describe("fullstack-a-67f slice 2: keymap + runCommand routing", () => {
  test("Mod+E hotkey in onWindowKey calls toggleActiveFileTabMode", () => {
    expect(app).toMatch(
      /if \(meta && !e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyE"\) \{[\s\S]{1,400}toggleActiveFileTabMode\(\);/,
    );
  });

  test("runCommand branch routes app.editor.toggleMode through the same helper", () => {
    expect(app).toMatch(
      /case "app\.editor\.toggleMode":[\s\S]{1,400}toggleActiveFileTabMode\(\);/,
    );
  });

  test("toggleActiveFileTabMode imported from tabs.svelte", () => {
    expect(app).toMatch(
      /import \{[\s\S]{1,4000}toggleActiveFileTabMode,[\s\S]{1,200}\} from "\.\/state\/tabs\.svelte";/,
    );
  });
});

describe("fullstack-a-67f slice 2: store-side helper", () => {
  test("toggleActiveFileTabMode flips between source and wysiwyg on the active file tab", () => {
    expect(tabs).toMatch(
      /export function toggleActiveFileTabMode\(\): void \{[\s\S]{1,800}if \(!tab \|\| tab\.kind !== "file"\) return;[\s\S]{1,200}tab\.mode = tab\.mode === "source" \? "wysiwyg" : "source";/,
    );
  });

  test("helper is a no-op when the active tab isn't a file", () => {
    expect(tabs).toMatch(
      /export function toggleActiveFileTabMode\(\): void \{[\s\S]{1,400}const node = layout\.nodes\[layout\.activePaneId\];[\s\S]{1,200}if \(!node \|\| node\.kind !== "leaf"\) return;/,
    );
  });
});

describe("fullstack-a-67f slice 2: chord hint in the editor menu", () => {
  test("Show Source Code button surfaces the Mod+E chord via chordLabel", () => {
    expect(editor).toMatch(
      /\{inSource \? renderedLabel : "Show Source Code"\}[\s\S]{1,400}<span class="mbtn-chord">\{chordLabel\("app\.editor\.toggleMode"\)\}<\/span>/,
    );
  });
});
