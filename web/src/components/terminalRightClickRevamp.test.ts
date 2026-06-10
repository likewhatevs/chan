import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";

// TerminalTab right-click menu shape. Tests pin the structure;
// behavioral coverage (Mod+L equivalents, the broadcast checkbox
// flow) belongs in TerminalTab.test.ts. The menu has:
//
// * Status row "connected: <detail>" (colon, not em dash).
// * MCP env vars + Restart above the find/copy band.
// * Find / Copy / Paste / Copy path to $CWD / Copy Scrollback.
// * "From $CWD" section: New File / New Terminal / New File
//   Browser / New Graph (with chord hints).
// * Settings (flipHybrid) + Reopen Closed Tab + Close anchor
//   the foot.
// * No Reload Window / Open Inspector entries.
// * MCP info-button opens a modal dialog.

describe("status row colon", () => {
  test("status row uses colon-separator (no em dash)", () => {
    expect(terminal).toMatch(/statusDetail \? `: \$\{statusDetail\}`/);
    expect(terminal).not.toMatch(/statusDetail \? ` - \$\{statusDetail\}`/);
  });

  test("Name then Group rows precede the status row", () => {
    // Name row, then the Group row (broadcast group, restart-gated), then
    // the status row. The Group row's restart prompt is conditional, so
    // allow it (or its absence) before the status comment.
    expect(terminal).toMatch(
      /<label class="rename-row">[\s\S]{1,800}<span>Name<\/span>[\s\S]{1,800}<\/label>\s*<label class="rename-row">[\s\S]{1,400}<span>Group<\/span>[\s\S]{1,1000}<\/label>[\s\S]{1,800}<!-- Status reads "connected: <detail>"[\s\S]{1,200}<div class="terminal-status-row">/,
    );
  });
});

describe("From-$CWD spawn band", () => {
  test("From-$CWD label is present", () => {
    expect(terminal).toMatch(/class="from-cwd-label">From \$CWD/);
  });

  test("openNewTerminal / openNewFileBrowser / openNewGraph helpers exported", () => {
    expect(terminal).toMatch(/function openNewTerminal\(\): void \{/);
    expect(terminal).toMatch(/function openNewFileBrowser\(\): void \{/);
    expect(terminal).toMatch(/function openNewGraph\(\): void \{/);
  });

  test("each From-$CWD helper closes the menu + dispatches the matching chan:command", () => {
    expect(terminal).toMatch(
      /function openNewTerminal\(\): void \{[\s\S]{1,200}closeTabMenu\(\);[\s\S]{1,200}dispatchChanCommand\("app\.terminal\.toggle"\);/,
    );
    expect(terminal).toMatch(
      /function openNewFileBrowser\(\): void \{[\s\S]{1,200}closeTabMenu\(\);[\s\S]{1,200}dispatchChanCommand\("app\.files\.toggle"\);/,
    );
    expect(terminal).toMatch(
      /function openNewGraph\(\): void \{[\s\S]{1,200}closeTabMenu\(\);[\s\S]{1,200}dispatchChanCommand\("app\.graph\.toggle"\);/,
    );
  });

  test("dispatchChanCommand fires the canonical chan:command event", () => {
    expect(terminal).toMatch(
      /function dispatchChanCommand\(id: string\): void \{[\s\S]{1,400}new CustomEvent\("chan:command", \{ detail: \{ name: id \} \}\)/,
    );
  });

  test("New File / New Terminal / New File Browser / New Graph buttons wired", () => {
    expect(terminal).toMatch(
      /onclick=\{openNewTerminal\}[\s\S]{1,400}<span class="mbtn-label">New Terminal<\/span>/,
    );
    expect(terminal).toMatch(
      /onclick=\{openNewFileBrowser\}[\s\S]{1,400}<span class="mbtn-label">New File Browser<\/span>/,
    );
    expect(terminal).toMatch(
      /onclick=\{openNewGraph\}[\s\S]{1,400}<span class="mbtn-label">New Graph<\/span>/,
    );
  });

  test("Copy path label uses the dollar form (\"Copy path to $CWD\")", () => {
    expect(terminal).toMatch(
      /<span class="mbtn-label">Copy path to \$CWD<\/span>/,
    );
  });
});

describe("header: Restart above the find/copy band", () => {
  test("Restart entry exists with destructive class", () => {
    expect(terminal).toMatch(
      /<button class="mbtn destructive" onclick=\{\(\) => void restart\(\)\}>[\s\S]{1,400}<span class="mbtn-label">Restart<\/span>/,
    );
  });

  test("Restart sits directly above the first separator + Copy path to $CWD", () => {
    // The per-terminal "Set MCP env vars" row was removed (the toggle
    // moved to the global Terminal Settings panel). Restart → SEP →
    // Copy path to $CWD is now the top of the TAB menu (Find / Copy /
    // Paste / Copy Scrollback live in the body-context menu since F4).
    expect(terminal).toMatch(
      /<span class="mbtn-label">Restart<\/span>[\s\S]{1,400}<div class="msep" role="separator"><\/div>[\s\S]{1,800}<span class="mbtn-label">Copy path to \$CWD<\/span>/,
    );
  });

  test("no per-terminal MCP env toggle remains in the menu", () => {
    expect(terminal).not.toContain("mcp-env-row");
    expect(terminal).not.toContain("Set MCP env vars");
  });
});

describe("F4: terminal body-context vs tab-context split", () => {
  test("body right-click opens the body source", () => {
    expect(terminal).toMatch(
      /function onTerminalContextMenu[\s\S]{1,200}openTabMenu\([\s\S]{1,300}"body",/,
    );
  });

  test("body menu is the tight Find / Copy / Paste / Copy Scrollback set", () => {
    // The body branch renders these four; the tab branch no longer
    // carries them. Pin the branch + the four entries' presence.
    expect(terminal).toContain('{#if tabMenu.source === "body"}');
    expect(terminal).toMatch(
      /tabMenu\.source === "body"[\s\S]{1,1200}onclick=\{openFind\}[\s\S]{1,400}onclick=\{copySelectionOrScrollback\}[\s\S]{1,400}onclick=\{pasteClipboard\}[\s\S]{1,400}onclick=\{copyScrollback\}/,
    );
  });
});

describe("Settings + Reopen + Close anchor the foot", () => {
  test("Settings entry routes to flipToSettings", () => {
    expect(terminal).toMatch(
      /onclick=\{flipToSettings\}[\s\S]{1,400}<span class="mbtn-label">Settings<\/span>/,
    );
  });

  test("flipToSettings calls flipHybrid(paneId)", () => {
    expect(terminal).toMatch(
      /function flipToSettings\(\): void \{[\s\S]{1,200}closeTabMenu\(\);[\s\S]{1,200}flipHybrid\(paneId\);/,
    );
  });

  test("Close menu entry wired to closeFromMenu", () => {
    expect(terminal).toMatch(
      /onclick=\{closeFromMenu\}[\s\S]{1,400}<span class="mbtn-label">Close<\/span>/,
    );
  });

  test("closeFromMenu invokes closeTab with paneId + tab.id", () => {
    expect(terminal).toMatch(
      /function closeFromMenu\(\): void \{[\s\S]{1,200}closeTabMenu\(\);[\s\S]{1,200}void closeTab\(paneId, tab\.id\);/,
    );
  });

  test("Reopen Closed Tab + Close land in the foot block in order", () => {
    expect(terminal).toMatch(
      /<span class="mbtn-label">Settings<\/span>[\s\S]{1,800}<div class="msep" role="separator"><\/div>[\s\S]{1,2000}<span class="mbtn-label">Reopen Closed Tab<\/span>[\s\S]{1,1000}<span class="mbtn-label">Close<\/span>/,
    );
  });
});

describe("flipHybrid imported from tabs.svelte", () => {
  test("flipHybrid imported from ../state/tabs.svelte", () => {
    expect(terminal).toMatch(
      /import \{[\s\S]{1,4000}flipHybrid,[\s\S]{1,2000}\} from "\.\.\/state\/tabs\.svelte";/,
    );
  });
});
