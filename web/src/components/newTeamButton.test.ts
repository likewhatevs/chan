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

describe("fullstack-a-78 slice 1: Rich Prompt button repurpose", () => {
  test("imports openTeamDialog from state/teamDialog", () => {
    expect(prompt).toMatch(
      /import \{ openTeamDialog as openGlobalTeamDialog \} from "\.\.\/state\/teamDialog\.svelte";/,
    );
  });

  test("openNewTeamDialog helper exists + opens the global dialog", () => {
    expect(prompt).toMatch(
      /function openNewTeamDialog\(\): void \{[\s\S]*?openGlobalTeamDialog\(\{[\s\S]*?hostSessionId: terminalSessionId,/,
    );
  });

  test("icon-btn switched from watchDirectory to openNewTeamDialog", () => {
    expect(prompt).toMatch(
      /class:on=\{Boolean\(watcherPath\)\}\s+onclick=\{openNewTeamDialog\}[\s\S]*?title="New Team"[\s\S]*?aria-label="New Team"/,
    );
  });

  test("the watchDirectory dropdown entry stays for now (legacy attach-watcher flow)", () => {
    // Slice 1 leaves the dropdown entry; the icon-btn is the
    // load-bearing repurposed button.
    expect(prompt).toMatch(/<button type="button" onclick=\{watchDirectory\}>/);
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
