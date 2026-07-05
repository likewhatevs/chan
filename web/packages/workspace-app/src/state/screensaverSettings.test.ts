import { describe, expect, test } from "vitest";
import shortcuts from "./shortcuts.ts?raw";
import app from "../App.svelte?raw";
import dashboard from "../components/dashboard/AboutSlotConfig.svelte?raw";

// Screensaver Settings UI + Hybrid Nav lock chord. Tests pin the
// architectural shape: there is no global Settings overlay; Screen Lock
// + Screensaver controls live on the Dashboard's back-of-card, in the
// About slot body (dashboard/AboutSlotConfig.svelte).

describe("Hybrid Nav lock chord", () => {
  test("screen lock has no built-in chord in the registry (no-defaults)", () => {
    // The Mod+. L default was dropped in the no-defaults round. Screen lock
    // stays reachable via the Dashboard back-of-card and the launcher, and is
    // assignable in the config UI, so no app.screensaver.lock entry remains in
    // SHORTCUTS. Plain Mod+L is likewise never claimed.
    expect(shortcuts).not.toContain('id: "app.screensaver.lock"');
    expect(shortcuts).not.toMatch(/web: "Mod\+L"[\s\S]{1,80}native: "Mod\+L"/);
  });

  test("App.svelte runCommand branch routes app.screensaver.lock through lockNow", () => {
    expect(app).toMatch(
      /case "app\.screensaver\.lock":[\s\S]{1,60}lockNow\(\);/,
    );
  });

  test("App.svelte does not claim plain Mod+L", () => {
    expect(app).not.toMatch(/e\.code === "KeyL"[\s\S]{1,160}lockNow\(\);/);
  });

  test("App.svelte no longer binds a Hybrid Nav L handler", () => {
    // The no-defaults round dropped the Mod+. L screen-lock binding; lock is
    // reached via the app.screensaver.lock command and the launcher.
    expect(app).not.toMatch(/case "l":[\s\S]{1,40}case "L":[\s\S]{1,220}lockNow\(\);/);
  });

  test("App.svelte imports lockNow alongside the tracker + loader", () => {
    expect(app).toMatch(
      /import \{[\s\S]{1,400}lockNow,[\s\S]{1,200}\} from "\.\/state\/screensaver\.svelte";/,
    );
  });
});

describe("Screen lock + Screensaver UI on Dashboard back-of-card", () => {
  test("Dashboard imports hashPin + bounds + lock helpers", () => {
    expect(dashboard).toMatch(
      /import \{[\s\S]{1,400}hashPin,[\s\S]{1,200}SCREENSAVER_MAX_TIMEOUT_SECS,[\s\S]{1,80}SCREENSAVER_MIN_TIMEOUT_SECS,[\s\S]{1,40}\} from "\.\.\/\.\.\/state\/screensaver";/,
    );
    expect(dashboard).toMatch(
      /import \{[\s\S]{1,200}loadScreensaverState,[\s\S]{1,80}lockNow,[\s\S]{1,80}screensaver,[\s\S]{1,40}\} from "\.\.\/\.\.\/state\/screensaver\.svelte";/,
    );
  });

  test("Dashboard carries the screensaver-settings reactive state vars", () => {
    expect(dashboard).toMatch(
      /let screensaverEnabled = \$state<boolean \| null>\(null\);/,
    );
    expect(dashboard).toMatch(/let screensaverTimeoutSecs = \$state<number>\(300\);/);
    expect(dashboard).toMatch(/let screensaverTheme = \$state<ScreensaverTheme>\("plain"\);/);
    expect(dashboard).toMatch(/let screensaverPinSet = \$state\(false\);/);
    expect(dashboard).toMatch(/let screensaverBusy = \$state\(false\);/);
    expect(dashboard).toMatch(/let screensaverError = \$state<string \| null>\(null\);/);
    expect(dashboard).toMatch(
      /let pinDialog = \$state<\{ pin1: string; pin2: string \} \| null>\(null\);/,
    );
  });

  test("loadScreenLockState fetches screensaver state via api.screensaverState", () => {
    expect(dashboard).toMatch(
      /const s = await api\.screensaverState\(\);[\s\S]{1,200}screensaverEnabled = s\.enabled;[\s\S]{1,200}screensaverTimeoutSecs = s\.timeout_secs;[\s\S]{1,200}screensaverTheme = s\.theme;[\s\S]{1,200}screensaverPinSet = s\.pin_set;/,
    );
  });

  test("theme picker persists plain/matrix through screensaverPatch", () => {
    expect(dashboard).toMatch(/type ScreensaverTheme/);
    expect(dashboard).toMatch(
      /async function commitScreensaverTheme\(e: Event\): Promise<void> \{[\s\S]{1,700}api\.screensaverPatch\(\{ theme \}\);[\s\S]{1,300}await loadScreensaverState\(\);/,
    );
    expect(dashboard).toMatch(
      /<select[\s\S]{1,300}bind:value=\{screensaverTheme\}[\s\S]{1,200}onchange=\{commitScreensaverTheme\}[\s\S]{1,300}<option value="plain">Default<\/option>[\s\S]{1,120}<option value="matrix">Matrix<\/option>/,
    );
  });

  test("Test button reloads state and locks immediately (no overlay open/close dance)", () => {
    // The back-of-card flip survives the screensaver cover, so
    // testScreenLock simply reloads state + calls lockNow; unlocking
    // returns to the same flipped view. No returnToSettingsAfterTest.
    expect(dashboard).toMatch(
      /async function testScreenLock\(\): Promise<void> \{[\s\S]{1,400}await loadScreensaverState\(\);[\s\S]{1,200}if \(!screensaver\.loaded\) \{[\s\S]{1,200}screen lock state unavailable[\s\S]{1,200}lockNow\(\);/,
    );
    expect(dashboard).not.toMatch(/returnToSettingsAfterTest/);
    expect(dashboard).toMatch(
      /<button type="button" onclick=\{testScreenLock\} disabled=\{screensaverBusy\}>[\s\S]{1,80}Test[\s\S]{1,80}<\/button>/,
    );
  });

  test("toggle handler patches enabled + reloads singleton", () => {
    expect(dashboard).toMatch(
      /async function toggleScreensaverEnabled\(\): Promise<void> \{[\s\S]{1,600}api\.screensaverPatch\(\{ enabled: target \}\);[\s\S]{1,400}await loadScreensaverState\(\);/,
    );
  });

  test("commit timeout clamps to MIN/MAX + patches + reloads", () => {
    expect(dashboard).toMatch(
      /async function commitTimeout\(\): Promise<void> \{[\s\S]{1,800}SCREENSAVER_MIN_TIMEOUT_SECS[\s\S]{1,400}SCREENSAVER_MAX_TIMEOUT_SECS[\s\S]{1,400}api\.screensaverPatch\(\{ timeout_secs: screensaverTimeoutSecs \}\);[\s\S]{1,400}await loadScreensaverState\(\);/,
    );
  });

  test("commit PIN validates match + hashes with workspace root salt + posts", () => {
    expect(dashboard).toMatch(
      /async function commitPin\(\): Promise<void> \{[\s\S]{1,600}if \(pin1 !== pin2\) \{[\s\S]{1,200}screensaverError = "PINs don't match";[\s\S]{1,400}const salt = workspace\.info\?\.root \?\? "";[\s\S]{1,200}const hash = await hashPin\(pin1, salt\);[\s\S]{1,200}api\.screensaverSetPin\(hash\);[\s\S]{1,400}await loadScreensaverState\(\);/,
    );
  });

  test("clearPin calls screensaverClearPin + reloads", () => {
    expect(dashboard).toMatch(
      /async function clearPin\(\): Promise<void> \{[\s\S]{1,400}api\.screensaverClearPin\(\);[\s\S]{1,400}await loadScreensaverState\(\);/,
    );
  });

  test("markup renders the screen-lock row with the enable toggle", () => {
    expect(dashboard).toMatch(
      /<section class="screen-lock">[\s\S]{1,2000}<h3>Screen lock<\/h3>/,
    );
    expect(dashboard).toMatch(
      /onchange=\{toggleScreensaverEnabled\}/,
    );
  });

  test("timeout input + PIN buttons gated on enabled=true", () => {
    expect(dashboard).toMatch(
      /\{#if screensaverEnabled === true\}[\s\S]{1,4000}bind:value=\{screensaverTimeoutSecs\}/,
    );
    expect(dashboard).toMatch(/onclick=\{openPinDialog\}/);
    expect(dashboard).toMatch(/onclick=\{clearPin\}/);
  });

  test("Theme picker renders INSIDE the screen lock enabled gate", () => {
    // The screensaver theme picker must live inside the
    // `{#if screensaverEnabled === true}` block within
    // `<section class="screen-lock">`, not as a standalone
    // `<section class="screensaver">` sibling. Toggling Screen
    // lock OFF hides the theme picker and timeout/PIN controls together.
    expect(dashboard).toMatch(
      /<section class="screen-lock">[\s\S]{1,4000}\{#if screensaverEnabled === true\}[\s\S]{1,4000}bind:value=\{screensaverTheme\}[\s\S]{1,4000}\{\/if\}[\s\S]{1,200}<\/section>/,
    );
    expect(dashboard).not.toMatch(/<section class="screensaver">/);
    expect(dashboard).not.toMatch(/<h3>Screensaver<\/h3>/);
  });

  test("inline PIN dialog binds pin1/pin2 + wires save+cancel", () => {
    expect(dashboard).toMatch(
      /\{#if pinDialog === null\}[\s\S]{1,4000}\{:else\}[\s\S]{1,2000}bind:value=\{pinDialog\.pin1\}[\s\S]{1,400}bind:value=\{pinDialog\.pin2\}[\s\S]{1,400}onclick=\{commitPin\}[\s\S]{1,200}onclick=\{cancelPinDialog\}/,
    );
  });
});
