import { describe, expect, test } from "vitest";
import editor from "./FileEditorTab.svelte?raw";

// FileEditorTab right-click menu. Pins: menu-top editable Name input;
// Show Source Code + Collapse Code Blocks; Find / Copy paths; From-$CWD
// spawn band (Duplicate, New File, New Terminal, New File Browser, New
// Graph); Settings (flipHybrid) + Reopen + Close foot. Reload + Open
// Inspector dropped (see tabMenuReloadInspector.test.ts).

describe("menu-top Name input", () => {
  test("name-row + name-input + name-label rendered inside the action-list", () => {
    expect(editor).toMatch(
      /<div class="action-list">[\s\S]{1,800}<label class="name-row">[\s\S]{1,400}<span class="name-label">[\s\S]{1,400}<input[\s\S]{1,800}class="name-input"/,
    );
  });

  test("Page width follows Name after the first separator", () => {
    expect(editor).toMatch(
      /<div class="action-list">[\s\S]{1,1200}<label class="name-row">[\s\S]{1,1400}<\/label>[\s\S]{1,120}<div class="msep" role="separator"><\/div>\s*<!-- Page-width slider:[\s\S]{1,400}<div class="page-width-row">/,
    );
  });

  test("input is bound to nameDraft + commits on blur via commitTabName", () => {
    expect(editor).toMatch(
      /bind:value=\{nameDraft\}[\s\S]{1,400}onkeydown=\{onTabNameKey\}[\s\S]{1,200}onblur=\{commitTabName\}/,
    );
  });

  test("Draft tabs replace the Name row with Save to Workspace", () => {
    expect(editor).toMatch(/const isDraftEditorTab = \$derived/);
    expect(editor).toMatch(
      /\{#if isDraftEditorTab\}[\s\S]{1,500}onclick=\{doSaveDraftToWorkspace\}[\s\S]{1,300}<span class="mbtn-label">Save to Workspace<\/span>/,
    );
    expect(editor).toMatch(/saveDraftTabToWorkspace\(tab\)/);
  });
});

describe("Show Source Code + Collapse Code Blocks", () => {
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

describe("From-$CWD spawn band", () => {
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

describe("Find / Copy paths", () => {
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

  test("Export to PDF is NOT in the editor menu (moved to the Inspector, A3-iii)", () => {
    // A3-iii moved Export to PDF out of the editor right-click menu and into
    // the file Inspector (FileInfoBody), shown for markdown files. The editor
    // no longer references the print helper.
    expect(editor).not.toContain("Export to PDF");
    expect(editor).not.toContain("printMarkdownDocument");
  });

  test("Copy path to file + Copy path to $CWD entries", () => {
    expect(editor).toMatch(
      /<span class="mbtn-label">Copy path to file<\/span>/,
    );
    expect(editor).toMatch(
      /<span class="mbtn-label">Copy path to \$CWD<\/span>/,
    );
  });

  test("doCopyCwdPath helper writes the parent-dir path to clipboard", () => {
    // Routes through copyTextToClipboard so the editor, inspector COPY,
    // and warnings dialog all share one writeText + fallback path.
    expect(editor).toMatch(
      /async function doCopyCwdPath\(\): Promise<void> \{[\s\S]{1,400}lastIndexOf\("\/"\)[\s\S]{1,400}copyTextToClipboard\(cwd/,
    );
  });
});

describe("Settings (flipHybrid) + Reopen + Close foot", () => {
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

describe("dropped entries", () => {
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

  test("\"Copy File Path\" is gone (replaced by \"Copy path to file\")", () => {
    expect(editor).not.toMatch(/<span class="mbtn-label">Copy File Path<\/span>/);
  });
});

describe("imports", () => {
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

describe("F4: editor body-context vs tab-context split", () => {
  test("body right-click opens the body source", () => {
    expect(editor).toMatch(
      /function onEditorContext[\s\S]{1,500}openTabMenu\([\s\S]{1,300}"body",/,
    );
  });

  test("body menu is the tight Cut / Copy / Paste + Find set", () => {
    expect(editor).toContain('{#if tabMenu.source === "body"}');
    expect(editor).toMatch(
      /tabMenu\.source === "body"[\s\S]{1,1500}onclick=\{doCutSelection\}[\s\S]{1,600}onclick=\{doCopySelection\}[\s\S]{1,600}onclick=\{doPasteClipboard\}[\s\S]{1,1800}onclick=\{doFind\}/,
    );
  });

  test("Cut / Copy gate on a selection (disabled when empty)", () => {
    expect(editor).toMatch(
      /onclick=\{doCutSelection\}[\s\S]{1,120}disabled=\{!bodyHasSelection\}/,
    );
    expect(editor).toMatch(
      /onclick=\{doCopySelection\}[\s\S]{1,120}disabled=\{!bodyHasSelection\}/,
    );
  });

  test("clipboard actions route to the active editor ref", () => {
    expect(editor).toMatch(
      /function activeEditorRef\(\)[\s\S]{1,200}tab\.mode === "source" \? sourceRef : wysiwygRef/,
    );
    expect(editor).toMatch(/activeEditorRef\(\)\?\.cutSelection\(\)/);
    expect(editor).toMatch(/activeEditorRef\(\)\?\.copySelection\(\)/);
    expect(editor).toMatch(/activeEditorRef\(\)\?\.pasteClipboard\(\)/);
  });
});

describe("F4 #3/#4: link affordances (editor body menu)", () => {
  test("link state is captured at right-click, before the menu covers it", () => {
    // elementFromPoint would resolve the menu portal once it is on
    // screen, so onEditorContext samples the link under the cursor first.
    expect(editor).toMatch(
      /function onEditorContext[\s\S]{1,400}externalUrlAtCoords\(e\.clientX, e\.clientY\)[\s\S]{1,200}internalLinkAtPoint\(e\.clientX, e\.clientY\)[\s\S]{1,200}openTabMenu/,
    );
  });

  test("Open link / Copy link show only on an external URL (#3 = A, no bubble)", () => {
    expect(editor).toMatch(
      /\{#if bodyLinkUrl\}[\s\S]{1,400}onclick=\{doOpenLink\}[\s\S]{1,400}onclick=\{doCopyLink\}/,
    );
    expect(editor).toMatch(/function doOpenLink[\s\S]{1,160}openExternalUrl\(url\)/);
    expect(editor).toMatch(/doCopyLink[\s\S]{1,200}copyTextToClipboard\(url/);
  });

  test("Preview shows only on an internal wiki link (#4) + reuses the popover", () => {
    expect(editor).toMatch(
      /\{#if bodyPreviewHit\}[\s\S]{1,300}onclick=\{doPreviewLink\}/,
    );
    expect(editor).toMatch(
      /function doPreviewLink[\s\S]{1,300}openLinkPreview\(\{[\s\S]{1,200}openInActivePane\(hit\.target\)/,
    );
  });
});
