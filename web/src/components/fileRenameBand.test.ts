// `fullstack-a-35` → `fullstack-a-67f`: inline file-rename moved
// from a full-width band above the editor body to a menu-top
// input row, per addendum-a's "Name, editable like Terminal's"
// spec. This test file now pins the NEW shape (menu-top input
// + nameDraft state + commitTabName + onTabNameKey) and the
// drop of the legacy `.rename-band` / `doRename` / `renameActive`
// state. The `fileOps.renameInPlace` helper in store.svelte.ts
// stays the canonical commit path.

import { describe, expect, test } from "vitest";
import fileEditor from "./FileEditorTab.svelte?raw";
import store from "../state/store.svelte.ts?raw";

describe("fullstack-a-67f: menu-top Name input replaces the inline rename band", () => {
  test("legacy doRename / renameActive / renameDraft state dropped", () => {
    expect(fileEditor).not.toMatch(/function doRename\(\): void/);
    expect(fileEditor).not.toMatch(/renameActive = \$state\(false\)/);
    expect(fileEditor).not.toMatch(/renameDraft = \$state\(/);
    expect(fileEditor).not.toMatch(/function cancelRename\(\)/);
    expect(fileEditor).not.toMatch(/function commitRename\(\)/);
    expect(fileEditor).not.toMatch(/function onRenameKeydown\(/);
  });

  test("legacy rename-band markup dropped", () => {
    expect(fileEditor).not.toMatch(/class="rename-band"/);
    expect(fileEditor).not.toMatch(/\{#if renameActive\}/);
  });

  test("new nameDraft state + effect that syncs to tab.path", () => {
    expect(fileEditor).toMatch(/let nameDraft = \$state\(""\);/);
    expect(fileEditor).toMatch(
      /\$effect\(\(\) => \{[\s\S]{1,400}nameDraft = tab\.path;/,
    );
  });

  test("commitTabName calls fileOps.renameInPlace with the trimmed draft", () => {
    expect(fileEditor).toMatch(
      /async function commitTabName\(\): Promise<void> \{[\s\S]{1,600}fileOps\.renameInPlace\(tab\.path, next, false\)/,
    );
  });

  test("onTabNameKey binds Enter and Escape to blur (Enter commits via onblur; Escape reverts)", () => {
    expect(fileEditor).toMatch(
      /function onTabNameKey\(e: KeyboardEvent\): void \{[\s\S]{1,400}if \(e\.key === "Enter"\) \{[\s\S]{1,200}\.blur\(\);/,
    );
    expect(fileEditor).toMatch(
      /onTabNameKey[\s\S]{1,800}if \(e\.key === "Escape"\) \{[\s\S]{1,200}nameDraft = tab\.path;[\s\S]{1,200}\.blur\(\);/,
    );
  });

  test("menu-top Name input lives inside the action-list, before any separator", () => {
    expect(fileEditor).toMatch(
      /<div class="action-list">[\s\S]{1,800}<label class="name-row">[\s\S]{1,1200}bind:value=\{nameDraft\}[\s\S]{1,400}onkeydown=\{onTabNameKey\}[\s\S]{1,200}onblur=\{commitTabName\}/,
    );
  });
});

describe("fullstack-a-67f: store.svelte renameInPlace unchanged (still the canonical commit path)", () => {
  test("fileOps.renameInPlace still calls performMove + preserveExtension", () => {
    expect(store).toContain(
      "async renameInPlace(path: string, next: string, isDir = false)",
    );
    expect(store).toMatch(
      /renameInPlace[\s\S]*?await performMove\(path, target\)/,
    );
    expect(store).toMatch(
      /renameInPlace[\s\S]*?preserveExtension\(path, trimmed\)/,
    );
  });
});
