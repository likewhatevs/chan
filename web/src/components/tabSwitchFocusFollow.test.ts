import { describe, expect, test } from "vitest";
import tabs from "../state/tabs.svelte.ts?raw";
import terminalTab from "./TerminalTab.svelte?raw";
import fileEditorTab from "./FileEditorTab.svelte?raw";
import source from "../editor/Source.svelte?raw";
import wysiwyg from "../editor/Wysiwyg.svelte?raw";

// `fullstack-a-64` CRITICAL: chord-driven tab switch
// (Cmd+Shift+[/], Ctrl+Alt+1..9) leaves keyboard focus on the
// PREVIOUS tab, causing typed/pasted keystrokes to damage the doc
// when the user thought they had switched to a terminal.
//
// Fix: introduce a global `tabFocusPulse` $state that all
// `select*TabInActivePane` helpers bump. Each tab-kind component
// has a $effect that depends on the pulse + re-focuses its
// surface. `bumpTabFocusPulse` also blurs the currently-focused
// element so the prior tab's contenteditable releases DOM focus
// before the new tab's focus call lands.

describe("fullstack-a-64: tabFocusPulse mechanism", () => {
  test("tabFocusPulse exported from tabs.svelte.ts", () => {
    expect(tabs).toMatch(/export const tabFocusPulse = \$state\(\{ value: 0 \}\);/);
  });

  test("bumpTabFocusPulse increments the pulse + blurs the prior focus", () => {
    expect(tabs).toMatch(
      /function bumpTabFocusPulse\(\): void \{[\s\S]*?tabFocusPulse\.value \+= 1;[\s\S]*?if \(typeof document === "undefined"\) return;[\s\S]*?const el = document\.activeElement;[\s\S]*?el\.blur\(\);/,
    );
  });

  test("selectPrevTabInActivePane bumps the pulse after mutating activeTabId", () => {
    expect(tabs).toMatch(
      /export function selectPrevTabInActivePane\(\): void \{[\s\S]*?p\.activeTabId = p\.tabs\[next\]\.id;[\s\S]*?bumpTabFocusPulse\(\);/,
    );
  });

  test("selectNextTabInActivePane bumps the pulse", () => {
    expect(tabs).toMatch(
      /export function selectNextTabInActivePane\(\): void \{[\s\S]*?p\.activeTabId = p\.tabs\[next\]\.id;[\s\S]*?bumpTabFocusPulse\(\);/,
    );
  });

  test("selectTabAtIndexInActivePane bumps the pulse", () => {
    expect(tabs).toMatch(
      /export function selectTabAtIndexInActivePane\(index: number\): void \{[\s\S]*?p\.activeTabId = p\.tabs\[index\]\.id;[\s\S]*?bumpTabFocusPulse\(\);/,
    );
  });
});

describe("fullstack-a-64: TerminalTab reacts to pulse", () => {
  test("TerminalTab imports tabFocusPulse", () => {
    expect(terminalTab).toMatch(/tabFocusPulse,/);
  });

  test("focus effect reads tabFocusPulse.value so chord switches re-focus xterm", () => {
    expect(terminalTab).toMatch(
      /\$effect\(\(\) => \{[\s\S]*?if \(!focused\) return;[\s\S]*?tabFocusPulse\.value;[\s\S]*?term\?\.focus\(\);/,
    );
  });
});

describe("fullstack-a-64: FileEditorTab reacts to pulse", () => {
  test("FileEditorTab imports tabFocusPulse", () => {
    expect(fileEditorTab).toMatch(/tabFocusPulse,/);
  });

  test("focus effect routes to the live editor ref (Wysiwyg vs Source) on pulse", () => {
    expect(fileEditorTab).toMatch(
      /\$effect\(\(\) => \{[\s\S]*?tabFocusPulse\.value;[\s\S]*?queueMicrotask\([\s\S]*?if \(tab\.mode === "wysiwyg"\) wysiwygRef\?\.focus\(\);[\s\S]*?else sourceRef\?\.focus\(\);/,
    );
  });
});

describe("fullstack-a-64: editor refs expose focus()", () => {
  test("Source.svelte exports focus()", () => {
    expect(source).toMatch(
      /export function focus\(\): boolean \{[\s\S]*?if \(!view\) return false;[\s\S]*?view\.focus\(\);[\s\S]*?return true;/,
    );
  });

  test("Wysiwyg.svelte exports focus()", () => {
    expect(wysiwyg).toMatch(
      /export function focus\(\): boolean \{[\s\S]*?if \(!view\) return false;[\s\S]*?view\.focus\(\);[\s\S]*?return true;/,
    );
  });
});
