import { describe, expect, test } from "vitest";
import shortcuts from "../state/shortcuts.ts?raw";
import app from "../App.svelte?raw";
import tabs from "../state/tabs.svelte.ts?raw";
import editor from "./FileEditorTab.svelte?raw";

// Mod+E "Show Source Code" chord. Pins: registry entry, keymap handler
// in App.svelte, runCommand branch, toggleActiveFileTabMode helper,
// and the chord-hint surface in the editor menu.

describe("shortcut registry entry", () => {
  test("app.editor.toggleMode entry exists with Mod+E (web + native)", () => {
    expect(shortcuts).toMatch(
      /id: "app\.editor\.toggleMode",[\s\S]{1,200}label: "Show Source Code \(toggle rendered\/source\)",[\s\S]{1,200}native: "Mod\+E",[\s\S]{1,80}web: "Mod\+E",/,
    );
  });

  test("entry sits in the Editor group + does NOT escape terminal", () => {
    // Off macOS Mod+E is Ctrl+E, which a focused terminal needs for
    // readline (move-to-end-of-line), so the chord must stay inside xterm.
    expect(shortcuts).toMatch(
      /id: "app\.editor\.toggleMode",[\s\S]{1,800}group: "Editor",[\s\S]{1,800}escapeTerminal: false,/,
    );
  });

  test("Editor group added to the ShortcutGroup union", () => {
    expect(shortcuts).toMatch(/export type ShortcutGroup =[\s\S]{1,200}\| "Editor";/);
  });
});

describe("keymap + runCommand routing", () => {
  test("Mod+E hotkey in onWindowKey calls toggleActiveFileTabMode (per-OS)", () => {
    expect(app).toMatch(
      /const toggleModeChord =[\s\S]{1,80}currentOS\(\) === "mac"[\s\S]{1,120}e\.metaKey && !e\.ctrlKey[\s\S]{1,80}e\.code === "KeyE"[\s\S]{1,120}: e\.ctrlKey && !e\.metaKey[\s\S]{1,80}e\.code === "KeyE";[\s\S]{1,200}toggleActiveFileTabMode\(\);/,
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

describe("store-side helper", () => {
  test("toggleActiveFileTabMode flips between source and the file's rendered mode", () => {
    // Gated to renderable files via defaultModeForPath (md→wysiwyg, json→pretty,
    // csv→table); source-only files (.rs/.py) yield "source" and the toggle
    // no-ops. Mirrors FileEditorTab's hasRenderedMode / renderedModeForTab gate.
    expect(tabs).toMatch(
      /export function toggleActiveFileTabMode\(\): void \{[\s\S]{1,800}if \(!tab \|\| tab\.kind !== "file"\) return;[\s\S]{1,300}const rendered = defaultModeForPath\(tab\.path, tab\.fileKind\);[\s\S]{1,120}if \(rendered === "source"\) return;[\s\S]{1,200}setMode\(tab, tab\.mode === "source" \? rendered : "source"\);/,
    );
  });

  test("helper is a no-op when the active tab isn't a file", () => {
    expect(tabs).toMatch(
      /export function toggleActiveFileTabMode\(\): void \{[\s\S]{1,400}const node = layout\.nodes\[layout\.activePaneId\];[\s\S]{1,200}if \(!node \|\| node\.kind !== "leaf"\) return;/,
    );
  });
});

describe("chord hint in the editor menu", () => {
  test("Show Source Code button surfaces the Mod+E chord via chordLabel", () => {
    expect(editor).toMatch(
      /\{inSource \? renderedLabel : "Show Source Code"\}[\s\S]{1,400}<span class="mbtn-chord">\{chordLabel\("app\.editor\.toggleMode"\)\}<\/span>/,
    );
  });
});
