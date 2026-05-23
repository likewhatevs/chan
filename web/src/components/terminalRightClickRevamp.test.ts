import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";

// `fullstack-a-67d`: TerminalTab right-click menu revamp per
// addendum-a's verbatim spec. Tests pin the architectural shape;
// behavioral coverage (Mod+L equivalents, the broadcast checkbox
// flow) belongs in TerminalTab.test.ts. Slice 1 covers:
//
// * Status row "connected: <detail>" (colon, not em dash).
// * MCP env vars + Restart pulled up above the find/copy band.
// * Find / Copy / Paste / Copy path to $CWD / Copy Scrollback.
// * "From $CWD" section: New File / New Terminal / New File
//   Browser / New Graph (with chord hints).
// * Settings (flipHybrid) + Reopen Closed Tab + Close anchor
//   the foot.
// * Reload Window + Open Inspector tail entries dropped.
//
// Deferred to slice 2: MCP info-button → modal dialog +
// Terminals expander dropdown + Jitter slider (Jitter has a
// chan-server gap; scope-poked separately).

describe("fullstack-a-67d: status row colon", () => {
  test("status row uses colon-separator (no em dash)", () => {
    expect(terminal).toMatch(/statusDetail \? `: \$\{statusDetail\}`/);
    expect(terminal).not.toMatch(/statusDetail \? ` - \$\{statusDetail\}`/);
  });

  test("Name row flows directly into status row with one visual separator", () => {
    expect(terminal).toMatch(
      /<label class="rename-row">[\s\S]{1,1400}<\/label>\s*<!-- `fullstack-a-67d`: status reads "connected: <detail>"[\s\S]{1,200}<div class="terminal-status-row">/,
    );
  });
});

describe("fullstack-a-67d: From-$CWD spawn band", () => {
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

describe("fullstack-a-67d: header moved up — MCP-env + Restart above the find/copy band", () => {
  test("Restart entry exists with destructive class", () => {
    expect(terminal).toMatch(
      /<button class="mbtn destructive" onclick=\{\(\) => void restart\(\)\}>[\s\S]{1,400}<span class="mbtn-label">Restart<\/span>/,
    );
  });

  test("Set MCP env vars row exists above the first separator inside action-list", () => {
    // mcp-env-row → mcp-info conditional → Show MCP env in
    // terminal → Restart → SEP → Find — that's the slice-1
    // ordering. Pin the relative order.
    expect(terminal).toMatch(
      /<div class="mcp-env-row">[\s\S]{1,4000}<span class="mbtn-label">Restart<\/span>[\s\S]{1,400}<div class="msep" role="separator"><\/div>[\s\S]{1,800}<span class="mbtn-label">Find<\/span>/,
    );
  });
});

describe("fullstack-a-67d: Settings + Reopen + Close anchor the foot", () => {
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

describe("fullstack-a-67d: flipHybrid imported from tabs.svelte", () => {
  test("flipHybrid imported from ../state/tabs.svelte", () => {
    expect(terminal).toMatch(
      /import \{[\s\S]{1,4000}flipHybrid,[\s\S]{1,2000}\} from "\.\.\/state\/tabs\.svelte";/,
    );
  });
});
