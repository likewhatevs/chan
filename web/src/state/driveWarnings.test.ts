import { describe, expect, test } from "vitest";
import types from "../api/types.ts?raw";
import app from "../App.svelte?raw";
import statusBar from "../components/AppStatusBar.svelte?raw";
import modal from "../components/DriveWarningsModal.svelte?raw";
import store from "./store.svelte.ts?raw";

describe("drive boot warnings", () => {
  test("DriveInfo carries non-fatal warning records", () => {
    expect(types).toMatch(/warnings: DriveWarning\[\];/);
    expect(types).toMatch(/export type DriveWarning = \{[\s\S]{1,160}kind: string;/);
  });

  test("bootstrap surfaces drive warnings as typed status actions", () => {
    expect(store).toMatch(
      /export const driveWarningsDialog = \$state<[\s\S]{1,600}warnings: \[\],[\s\S]{1,220}notice: null,/,
    );
    expect(store).toMatch(
      /function surfaceDriveWarnings\(info: DriveInfo\): void \{[\s\S]{1,500}driveWarningsDialog\.warnings = warnings;[\s\S]{1,700}ui\.statusAction = \{ kind: "drive-warnings", label \};/,
    );
    expect(store).toMatch(
      /const info = await api\.drive\(\);[\s\S]{1,160}drive\.info = info;[\s\S]{1,160}surfaceDriveWarnings\(info\);/,
    );
    expect(store).toMatch(
      /export async function refreshDrive\(\): Promise<void> \{[\s\S]{1,200}surfaceDriveWarnings\(info\);/,
    );
  });

  test("status bar opens warning dialog only for matching typed actions", () => {
    expect(statusBar).toMatch(
      /ui\.statusAction\?\.kind === "drive-warnings"[\s\S]{1,120}ui\.statusAction\.label === ui\.status/,
    );
    expect(statusBar).toMatch(
      /<button[\s\S]{1,220}class="section status-msg status-action"[\s\S]{1,180}onclick=\{activateStatus\}/,
    );
    expect(statusBar).toMatch(/openDriveWarningsDialog/);
  });

  test("warning dialog can copy, dismiss, and discard through draft API", () => {
    expect(app).toMatch(/import DriveWarningsModal from "\.\/components\/DriveWarningsModal\.svelte";/);
    expect(app).toMatch(/<DriveWarningsModal \/>/);
    expect(modal).toMatch(/role="dialog"/);
    expect(modal).toMatch(/Copy path/);
    expect(modal).toMatch(/Dismiss/);
    expect(modal).toMatch(/Discard metadata/);
    expect(store).toMatch(
      /export async function discardDriveWarning\(warning: DriveWarning\): Promise<void> \{[\s\S]{1,500}await api\.discardDraft\(warning\.path\);[\s\S]{1,240}surfaceDriveWarnings\(info\);/,
    );
  });
});
