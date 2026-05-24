import { describe, expect, test } from "vitest";
import types from "../api/types.ts?raw";
import store from "./store.svelte.ts?raw";

describe("drive boot warnings", () => {
  test("DriveInfo carries non-fatal warning records", () => {
    expect(types).toMatch(/warnings: DriveWarning\[\];/);
    expect(types).toMatch(/export type DriveWarning = \{[\s\S]{1,160}kind: string;/);
  });

  test("bootstrap surfaces broken draft warnings persistently", () => {
    expect(store).toMatch(
      /function surfaceDriveWarnings\(info: DriveInfo\): void \{[\s\S]{1,500}warning\.kind === "broken_draft"[\s\S]{1,500}ui\.status = `Broken draft \$\{warning\.path\}: \$\{warning\.message\}`;[\s\S]{1,160}ui\.statusKind = "persistent";/,
    );
    expect(store).toMatch(
      /const info = await api\.drive\(\);[\s\S]{1,160}drive\.info = info;[\s\S]{1,160}surfaceDriveWarnings\(info\);/,
    );
  });
});
