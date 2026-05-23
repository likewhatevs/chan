import { describe, expect, test } from "vitest";
import editor from "./FileEditorTab.svelte?raw";

// `fullstack-a-67f`: FileEditorTab right-click menu revamp per
// addendum-a's verbatim Editor spec. Slice 1 covers:
//
// * Menu-top editable Name input (mirror of Drive name in FB +
//   Terminal name).
// * Show Source Code + Collapse Code Blocks band.
// * View-toggles + cleanup utilities (Outline / Details / Style
//   Toolbar / Syntax Highlight / Highlight TW / Remove TW) kept
//   against spec — orphan risk; flagged in journal.
// * Search / Find / Copy path to file / Copy path to $CWD band.
// * "From $CWD" spawn band (Duplicate / New File / New Terminal /
//   New File Browser / New Graph).
// * Settings (flipHybrid) + Reopen Closed Tab + Close foot.
// * Reload Window / Open Inspector tail dropped (handled in
//   tabMenuReloadInspector.test.ts).

describe("fullstack-a-67f: menu-top Name input", () => {
  test("name-row + name-input + name-label rendered inside the action-list", () => {
    expect(editor).toMatch(
      /<div class="action-list">[\s\S]{1,800}<label class="name-row">[\s\S]{1,400}<span class="name-label">[\s\S]{1,400}<input[\s\S]{1,800}class="name-input"/,
    );
  });

  test("Page width follows Name after the first separator", () => {
    expect(editor).toMatch(
      /<div class="action-list">[\s\S]{1,800}<label class="name-row">[\s\S]{1,1400}<\/label>\s*<div class="msep" role="separator"><\/div>\s*<!-- Page-width slider:[\s\S]{1,400}<div class="page-width-row">/,
    );
  });

  test("input is bound to nameDraft + commits on blur via commitTabName", () => {
    expect(editor).toMatch(
      /bind:value=\{nameDraft\}[\s\S]{1,400}onkeydown=\{onTabNameKey\}[\s\S]{1,200}onblur=\{commitTabName\}/,
    );
  });
});

describe("fullstack-a-67f: Show Source Code + Collapse Code Blocks", () => {
  test("Show Source Code toggle present (via doToggleMode); label switches based on mode", () => {
    expect(editor).toMatch(
      /onclick=\{doToggleMode\}[\s\S]{1,800}\{inSource \? renderedLabel : "Show Source Code"\}/,
    );
  });

  test("Collapse Code Blocks label uses title-case + gated on markdownToolsEnabled", () => {
    expect(editor).toMatch(
      /\{#if markdownToolsEnabled\}[\s\S]{1,800}\{tab\.codeBlocksCollapsed \? "Expand Code Blocks" : "Collapse Code Blocks"\}/,
    );
  });
});

describe("fullstack-a-67f: From-$CWD spawn band", () => {
  test("from-cwd-label rendered above the spawn buttons", () => {
    expect(editor).toMatch(/class="from-cwd-label">From \$CWD/);
  });

  test("doNewTerminal / doNewFileBrowser / doNewGraph helpers exist + dispatch chan:command", () => {
    expect(editor).toMatch(
      /function doNewTerminal\(\): void \{[\s\S]{1,200}dispatchChanCommand\("app\.terminal\.toggle"\)/,
    );
    expect(editor).toMatch(
      /function doNewFileBrowser\(\): void \{[\s\S]{1,200}dispatchChanCommand\("app\.files\.toggle"\)/,
    );
    expect(editor).toMatch(
      /function doNewGraph\(\): void \{[\s\S]{1,200}dispatchChanCommand\("app\.graph\.toggle"\)/,
    );
  });

  test("dispatchChanCommand fires the canonical chan:command event", () => {
    expect(editor).toMatch(
      /function dispatchChanCommand\(id: string\): void \{[\s\S]{1,400}new CustomEvent\("chan:command", \{ detail: \{ name: id \} \}\)/,
    );
  });

  test("Duplicate / New File / New Terminal / New File Browser / New Graph buttons rendered", () => {
    expect(editor).toMatch(
      /onclick=\{doDuplicate\}[\s\S]{1,400}<span class="mbtn-label">Duplicate File<\/span>/,
    );
    expect(editor).toMatch(
      /onclick=\{doNewFile\}[\s\S]{1,400}<span class="mbtn-label">New File<\/span>/,
    );
    expect(editor).toMatch(
      /onclick=\{doNewTerminal\}[\s\S]{1,400}<span class="mbtn-label">New Terminal<\/span>/,
    );
    expect(editor).toMatch(
      /onclick=\{doNewFileBrowser\}[\s\S]{1,400}<span class="mbtn-label">New File Browser<\/span>/,
    );
    expect(editor).toMatch(
      /onclick=\{doNewGraph\}[\s\S]{1,400}<span class="mbtn-label">New Graph<\/span>/,
    );
  });
});

describe("fullstack-a-67f: Find / Copy paths", () => {
  test("doFind opens the per-tab find bar via openFind(tab.id)", () => {
    expect(editor).toMatch(
      /function doFind\(\): void \{[\s\S]{1,200}closeTabMenu\(\);[\s\S]{1,200}openFind\(tab\.id\)/,
    );
  });

  test("Find button rendered with chord hint", () => {
    expect(editor).toMatch(
      /onclick=\{doFind\}[\s\S]{1,400}<span class="mbtn-label">Find<\/span>/,
    );
  });

  test("Copy path to file (renamed) + Copy path to $CWD (new) entries", () => {
    expect(editor).toMatch(
      /<span class="mbtn-label">Copy path to file<\/span>/,
    );
    expect(editor).toMatch(
      /<span class="mbtn-label">Copy path to \$CWD<\/span>/,
    );
  });

  test("doCopyCwdPath helper writes the parent-dir path to clipboard", () => {
    expect(editor).toMatch(
      /async function doCopyCwdPath\(\): Promise<void> \{[\s\S]{1,400}lastIndexOf\("\/"\)[\s\S]{1,400}navigator\.clipboard\?\.writeText\(cwd\)/,
    );
  });
});

describe("fullstack-a-67f: Settings (flipHybrid) + Reopen + Close foot", () => {
  test("flipToSettings calls flipHybrid via paneIdForTab", () => {
    expect(editor).toMatch(
      /function flipToSettings\(\): void \{[\s\S]{1,400}const paneId = paneIdForTab\(\);[\s\S]{1,200}if \(paneId\) flipHybrid\(paneId\)/,
    );
  });

  test("Settings button wired to flipToSettings (not the legacy doOpenSettings)", () => {
    expect(editor).toMatch(
      /onclick=\{flipToSettings\}[\s\S]{1,400}<span class="mbtn-label">Settings<\/span>/,
    );
    expect(editor).not.toMatch(/onclick=\{doOpenSettings\}/);
  });

  test("Reopen Closed Tab + Close buttons land in the foot block in order", () => {
    expect(editor).toMatch(
      /<span class="mbtn-label">Settings<\/span>[\s\S]{1,1000}<div class="msep" role="separator"><\/div>[\s\S]{1,2000}<span class="mbtn-label">Reopen Closed Tab<\/span>[\s\S]{1,1000}<span class="mbtn-label">Close<\/span>/,
    );
  });
});

describe("fullstack-a-67f: dropped entries", () => {
  test("Close others / Close all entries dropped (per spec)", () => {
    expect(editor).not.toMatch(/<span class="mbtn-label">Close others<\/span>/);
    expect(editor).not.toMatch(/<span class="mbtn-label">Close all<\/span>/);
  });

  test("\"Rename File\" entry dropped (replaced by menu-top Name input)", () => {
    expect(editor).not.toMatch(/<span class="mbtn-label">Rename File<\/span>/);
  });

  test("\"Terminal from here\" dropped (replaced by \"New Terminal\" in From-$CWD band)", () => {
    expect(editor).not.toMatch(
      /<span class="mbtn-label">Terminal from here<\/span>/,
    );
  });

  test("\"Copy File Path\" renamed to \"Copy path to file\"", () => {
    expect(editor).not.toMatch(/<span class="mbtn-label">Copy File Path<\/span>/);
  });
});

describe("fullstack-a-67f: imports", () => {
  test("flipHybrid + openFind imported from tabs.svelte", () => {
    expect(editor).toMatch(
      /import \{[\s\S]{1,2000}flipHybrid,[\s\S]{1,800}\} from "\.\.\/state\/tabs\.svelte";/,
    );
    expect(editor).toMatch(
      /import \{[\s\S]{1,2000}openFind,[\s\S]{1,800}\} from "\.\.\/state\/tabs\.svelte";/,
    );
  });

  test("Settings2 + Terminal as TerminalIcon imported from lucide", () => {
    expect(editor).toMatch(/Settings2,/);
    expect(editor).toMatch(/Terminal as TerminalIcon,/);
  });
});
