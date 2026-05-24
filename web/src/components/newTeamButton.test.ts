import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import prompt from "./TerminalRichPrompt.svelte?raw";
import dialog from "./TeamDialog.svelte?raw";

// `fullstack-a-78` slice 1: button repurpose + dialog mount.
// Tests pin the wiring shape so the airplane-grid slice 2 can
// extend the dialog without re-litigating slice 1's contract.

describe("fullstack-a-78 slice 1: App.svelte mounts TeamDialog at root", () => {
  test("imports TeamDialog + teamDialogState", () => {
    expect(app).toMatch(/import TeamDialog from "\.\/components\/TeamDialog\.svelte";/);
    expect(app).toMatch(/import \{ teamDialogState \} from "\.\/state\/teamDialog\.svelte";/);
  });

  test("renders the dialog only when a request is pending", () => {
    expect(app).toMatch(
      /\{#if teamDialogState\.request\}\s*<TeamDialog request=\{teamDialogState\.request\} \/>\s*\{\/if\}/,
    );
  });
});

describe("fullstack-a-78 slice 1: Rich Prompt Spawn agents button", () => {
  test("imports openTeamDialog from state/teamDialog", () => {
    expect(prompt).toMatch(
      /import \{[\s\S]*?openTeamDialog as openGlobalTeamDialog,[\s\S]*?\} from "\.\.\/state\/teamDialog\.svelte";/,
    );
  });

  test("openNewTeamDialog helper exists + opens the global dialog", () => {
    expect(prompt).toMatch(
      /function openNewTeamDialog\(\): void \{[\s\S]*?openGlobalTeamDialog\(\{[\s\S]*?hostSessionId: terminalSessionId,/,
    );
  });

  test("plus menu opens the Spawn agents dialog", () => {
    expect(prompt).toMatch(
      /onclick=\{openMenuFromButton\}[\s\S]*?aria-label="Rich Prompt actions"/,
    );
    expect(prompt).toMatch(
      /<button type="button" onclick=\{openNewTeamDialog\}>[\s\S]*?<span>Spawn agents<\/span>/,
    );
  });

  test("legacy file and manual watcher actions are gone", () => {
    expect(prompt).not.toMatch(/New File from here/);
    expect(prompt).not.toMatch(/function watchDirectory/);
    expect(prompt).not.toMatch(/Stop watching/);
  });
});

describe("fullstack-a-78 slice 1: TeamDialog component shell", () => {
  test("renders Bootstrap + Cancel footer buttons", () => {
    expect(dialog).toMatch(/class="team-dialog-bootstrap"[\s\S]*?onclick=\{onBootstrap\}/);
    expect(dialog).toMatch(/class="team-dialog-cancel"[\s\S]*?onclick=\{onCancel\}/);
  });

  test("inputs cover host name + team name + size slider + auto-prefix checkbox", () => {
    expect(dialog).toMatch(/bind:value=\{config\.hostName\}/);
    expect(dialog).toMatch(/bind:value=\{config\.teamName\}/);
    expect(dialog).toMatch(/bind:checked=\{config\.autoPrefix\}/);
    expect(dialog).toMatch(/type="range"[\s\S]*?min=\{TEAM_MIN_SIZE\}[\s\S]*?max=\{TEAM_MAX_SIZE\}/);
  });

  test("renders copy/paste config actions", () => {
    expect(dialog).toMatch(/onclick=\{\(\) => void onCopyConfig\(\)\}/);
    expect(dialog).toMatch(/onclick=\{\(\) => void onPasteConfig\(\)\}/);
    expect(dialog).toMatch(/Copy config/);
    expect(dialog).toMatch(/Paste config/);
  });

  test("per-member row renders name + command + env + lead radio", () => {
    expect(dialog).toMatch(/class="team-member-name"/);
    expect(dialog).toMatch(/class="team-member-command"/);
    expect(dialog).toMatch(/class="team-member-env"/);
    expect(dialog).toMatch(/name="team-lead"/);
  });

  test("airplane-grid placeholder for slice 2 present", () => {
    // Match loosely: text uses `&` literal in Svelte source but
    // the compiled HTML may entity-encode it; assert the
    // key terms + the slice-2 reference.
    expect(dialog).toMatch(/Airplane-grid/);
    expect(dialog).toMatch(/fullstack-a-78[\s\S]*?slice 2/);
  });

  test("Escape key closes the dialog (svelte:window onkeydown wired)", () => {
    expect(dialog).toMatch(
      /function onKeydown\(e: KeyboardEvent\): void \{[\s\S]*?if \(e\.key === "Escape" && !busy\) \{[\s\S]*?onCancel\(\);/,
    );
  });
});
