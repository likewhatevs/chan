import { describe, expect, test } from "vitest";
import modal from "./McpEnvInfoModal.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";

// MCP env info button opens a modal dialog. The "Show MCP env in
// terminal" CTA lives inside the modal; the inline popover is gone.

describe("McpEnvInfoModal component", () => {
  test("modal renders gated on the `open` prop with dialog role + aria-modal", () => {
    expect(modal).toMatch(/\{#if open\}/);
    expect(modal).toMatch(
      /<div class="mcp-modal" role="dialog" aria-modal="true" aria-label="MCP env vars">/,
    );
  });

  test("modal width matches PathPromptModal (min-width 420px, max-width 80vw)", () => {
    expect(modal).toMatch(/min-width: 420px;[\s\S]{1,100}max-width: 80vw;/);
  });

  test("body explains the CHAN_MCP env vars + applies-to-new-sessions caveat", () => {
    expect(modal).toMatch(/CHAN_MCP_SOCKET/);
    expect(modal).toMatch(/CHAN_MCP_SERVER_JSON/);
    expect(modal).toMatch(/Applies to new[\s\S]{1,40}sessions only/);
  });

  test("CTA \"Show MCP env in terminal\" fires onShowInTerminal then closes", () => {
    expect(modal).toMatch(
      /function commitShow\(\): void \{[\s\S]{1,200}onShowInTerminal\(\);[\s\S]{1,200}onClose\(\);/,
    );
    expect(modal).toMatch(/onclick=\{commitShow\}/);
    expect(modal).toMatch(/Show MCP env in terminal/);
  });

  test("CTA disabled state follows showInTerminalDisabled prop", () => {
    expect(modal).toMatch(/disabled=\{showInTerminalDisabled\}/);
  });

  test("backdrop click + Escape both close via onClose", () => {
    expect(modal).toMatch(
      /function onBackdropClick\(e: MouseEvent\): void \{[\s\S]{1,400}if \(e\.target === e\.currentTarget\) onClose\(\);/,
    );
    expect(modal).toMatch(
      /function onKey\(e: KeyboardEvent\): void \{[\s\S]{1,400}if \(e\.key === "Escape"\)[\s\S]{1,200}onClose\(\);/,
    );
  });
});

describe("TerminalTab wiring", () => {
  test("McpEnvInfoModal imported + mounted", () => {
    expect(terminal).toMatch(
      /import McpEnvInfoModal from "\.\/McpEnvInfoModal\.svelte";/,
    );
    expect(terminal).toMatch(
      /<McpEnvInfoModal[\s\S]{1,400}open=\{mcpInfoOpen\}[\s\S]{1,200}onClose=\{closeMcpInfoModal\}[\s\S]{1,200}onShowInTerminal=\{showMcpEnv\}[\s\S]{1,200}showInTerminalDisabled=\{showMcpEnvDisabled\}/,
    );
  });

  test("info button opens the modal via openMcpInfoModal (no inline popover)", () => {
    expect(terminal).toMatch(
      /function openMcpInfoModal\(\): void \{[\s\S]{1,200}closeTabMenu\(\);[\s\S]{1,200}mcpInfoOpen = true;/,
    );
    expect(terminal).toMatch(
      /class="info-btn"[\s\S]{1,400}onclick=\{openMcpInfoModal\}/,
    );
  });

  test("inline mcp-info popover block dropped", () => {
    expect(terminal).not.toMatch(/<div class="mcp-info">/);
    expect(terminal).not.toMatch(/aria-expanded=\{mcpInfoOpen\}/);
  });

  test("standalone \"Show MCP env in terminal\" menu row dropped (CTA lives in modal)", () => {
    expect(terminal).not.toMatch(
      /<span class="mbtn-label">Show MCP env in terminal<\/span>/,
    );
  });
});
