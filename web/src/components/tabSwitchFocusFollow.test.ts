import { describe, expect, test } from "vitest";
import tabs from "../state/tabs.svelte.ts?raw";
import terminalTab from "./TerminalTab.svelte?raw";
import fileEditorTab from "./FileEditorTab.svelte?raw";
import pane from "./Pane.svelte?raw";
import source from "../editor/Source.svelte?raw";
import wysiwyg from "../editor/Wysiwyg.svelte?raw";

// Chord-driven tab switch (Cmd+Shift+[/], Ctrl+Alt+1..9) must
// move keyboard focus to the new tab so keystrokes go to the
// correct surface. The `tabFocusPulse` $state mechanism drives
// this: helpers bump the pulse, and each tab-kind component
// re-focuses its surface in a $effect that tracks it.
// `bumpTabFocusPulse` also blurs the current element so the
// prior contenteditable releases DOM focus before the new
// tab's focus call lands.

describe("tabFocusPulse mechanism", () => {
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

describe("tab header click refocuses input-capable tabs", () => {
  test("Pane imports bumpTabFocusPulse for tab-strip clicks", () => {
    expect(pane).toMatch(/import \{[\s\S]*?\bbumpTabFocusPulse,[\s\S]*?\} from "\.\.\/state\/tabs\.svelte";/);
  });

  test("terminal/editor tab mousedown selects the tab then pulses content focus", () => {
    expect(pane).toMatch(
      /onmousedown=\{\(\) => \{[\s\S]*?pane\.activeTabId = t\.id;[\s\S]*?if \(t\.kind === "terminal"\) setTerminalActivity\(t, false\);[\s\S]*?if \(t\.kind === "terminal" \|\| t\.kind === "file"\) bumpTabFocusPulse\(\);/,
    );
  });
});

describe("TerminalTab reacts to pulse", () => {
  test("TerminalTab imports tabFocusPulse", () => {
    expect(terminalTab).toMatch(/tabFocusPulse,/);
  });

  test("focus effect reads tabFocusPulse.value so chord switches re-focus xterm", () => {
    expect(terminalTab).toMatch(
      /\$effect\(\(\) => \{[\s\S]*?if \(!focused\) return;[\s\S]*?tabFocusPulse\.value;[\s\S]*?term\?\.focus\(\);/,
    );
  });
});

describe("FileEditorTab reacts to pulse", () => {
  test("FileEditorTab imports tabFocusPulse", () => {
    expect(fileEditorTab).toMatch(/tabFocusPulse,/);
  });

  test("focus effect routes to the live editor ref (Wysiwyg vs Source) on pulse", () => {
    expect(fileEditorTab).toMatch(
      /\$effect\(\(\) => \{[\s\S]*?tabFocusPulse\.value;[\s\S]*?queueMicrotask\([\s\S]*?if \(tab\.mode === "wysiwyg"\) wysiwygRef\?\.focus\(\);[\s\S]*?else sourceRef\?\.focus\(\);/,
    );
  });
});

describe("editor refs expose focus", () => {
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
