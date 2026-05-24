import { describe, expect, test } from "vitest";
import shortcuts from "./shortcuts.ts?raw";
import app from "../App.svelte?raw";
import panel from "../components/SettingsPanel.svelte?raw";

// `fullstack-a-77` slice 3: Settings UI + Mod+L lock chord.
// Tests pin the architectural shape; behavioral testing of
// the timeout slider + PIN flow happens via @@WebtestA's
// empirical walk.

describe("fullstack-a-77 slice 3: Mod+L chord", () => {
  test("shortcuts entry exists for app.screensaver.lock", () => {
    expect(shortcuts).toMatch(
      /id: "app\.screensaver\.lock",[\s\S]{1,60}label: "Lock screen",[\s\S]{1,200}web: "Mod\+L",[\s\S]{1,80}native: "Mod\+L",/,
    );
  });

  test("shortcut group + escapeTerminal pinned", () => {
    expect(shortcuts).toMatch(
      /id: "app\.screensaver\.lock",[\s\S]{1,400}group: "App",[\s\S]{1,80}escapeTerminal: true,/,
    );
  });

  test("App.svelte runCommand branch routes app.screensaver.lock through lockNow", () => {
    expect(app).toMatch(
      /case "app\.screensaver\.lock":[\s\S]{1,60}lockNow\(\);/,
    );
  });

  test("App.svelte onWindowKey handler fires lockNow on Mod+L", () => {
    expect(app).toMatch(
      /if \(meta && !e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyL"\) \{[\s\S]{1,160}lockNow\(\);/,
    );
  });

  test("App.svelte imports lockNow alongside the tracker + loader", () => {
    expect(app).toMatch(
      /import \{[\s\S]{1,400}lockNow,[\s\S]{1,200}\} from "\.\/state\/screensaver\.svelte";/,
    );
  });
});

describe("fullstack-a-77 slice 3: Settings UI", () => {
  test("Settings imports hashPin + bounds + lock helpers", () => {
    expect(panel).toMatch(
      /import \{[\s\S]{1,400}hashPin,[\s\S]{1,200}SCREENSAVER_MAX_TIMEOUT_SECS,[\s\S]{1,80}SCREENSAVER_MIN_TIMEOUT_SECS,[\s\S]{1,40}\} from "\.\.\/state\/screensaver";/,
    );
    expect(panel).toMatch(
      /import \{[\s\S]{1,200}loadScreensaverState,[\s\S]{1,80}lockNow,[\s\S]{1,80}pauseScreensaverTimer,[\s\S]{1,80}screensaver,[\s\S]{1,40}\} from "\.\.\/state\/screensaver\.svelte";/,
    );
  });

  test("Settings carries the slice-3 reactive state vars", () => {
    expect(panel).toMatch(
      /let screensaverEnabled = \$state<boolean \| null>\(null\);/,
    );
    expect(panel).toMatch(/let screensaverTimeoutSecs = \$state<number>\(300\);/);
    expect(panel).toMatch(/let screensaverTheme = \$state<ScreensaverTheme>\("plain"\);/);
    expect(panel).toMatch(/let screensaverPinSet = \$state\(false\);/);
    expect(panel).toMatch(/let screensaverBusy = \$state\(false\);/);
    expect(panel).toMatch(/let screensaverError = \$state<string \| null>\(null\);/);
    expect(panel).toMatch(/let returnToSettingsAfterTest = \$state\(false\);/);
    expect(panel).toMatch(
      /let pinDialog = \$state<\{ pin1: string; pin2: string \} \| null>\(null\);/,
    );
  });

  test("loadScreenLockState fetches screensaver state via api.screensaverState", () => {
    expect(panel).toMatch(
      /const s = await api\.screensaverState\(\);[\s\S]{1,200}screensaverEnabled = s\.enabled;[\s\S]{1,200}screensaverTimeoutSecs = s\.timeout_secs;[\s\S]{1,200}screensaverTheme = s\.theme;[\s\S]{1,200}screensaverPinSet = s\.pin_set;/,
    );
  });

  test("theme picker persists plain/matrix through screensaverPatch", () => {
    expect(panel).toMatch(/type ScreensaverTheme/);
    expect(panel).toMatch(
      /async function commitScreensaverTheme\(e: Event\): Promise<void> \{[\s\S]{1,700}api\.screensaverPatch\(\{ theme \}\);[\s\S]{1,300}await loadScreensaverState\(\);/,
    );
    expect(panel).toMatch(
      /<select[\s\S]{1,300}bind:value=\{screensaverTheme\}[\s\S]{1,200}onchange=\{commitScreensaverTheme\}[\s\S]{1,300}<option value="plain">Plain<\/option>[\s\S]{1,120}<option value="matrix">Matrix<\/option>/,
    );
  });

  test("Test button reloads state and locks immediately", () => {
    expect(panel).toMatch(
      /async function testScreenLock\(\): Promise<void> \{[\s\S]{1,400}await loadScreensaverState\(\);[\s\S]{1,200}if \(!screensaver\.loaded\) \{[\s\S]{1,200}screen lock state unavailable[\s\S]{1,200}returnToSettingsAfterTest = true;[\s\S]{1,120}settingsOverlay\.open = false;[\s\S]{1,120}lockNow\(\);/,
    );
    expect(panel).toMatch(
      /<button type="button" onclick=\{testScreenLock\} disabled=\{screensaverBusy\}>[\s\S]{1,80}Test[\s\S]{1,80}<\/button>/,
    );
  });

  test("test mode restores Settings after unlock", () => {
    expect(panel).toMatch(
      /\$effect\(\(\) => \{[\s\S]{1,120}if \(!returnToSettingsAfterTest\) return;[\s\S]{1,80}if \(screensaver\.locked\) return;[\s\S]{1,120}returnToSettingsAfterTest = false;[\s\S]{1,80}settingsOverlay\.open = true;/,
    );
  });

  test("toggle handler patches enabled + reloads singleton", () => {
    expect(panel).toMatch(
      /async function toggleScreensaverEnabled\(\): Promise<void> \{[\s\S]{1,600}api\.screensaverPatch\(\{ enabled: target \}\);[\s\S]{1,400}await loadScreensaverState\(\);/,
    );
  });

  test("commit timeout clamps to MIN/MAX + patches + reloads", () => {
    expect(panel).toMatch(
      /async function commitTimeout\(\): Promise<void> \{[\s\S]{1,800}SCREENSAVER_MIN_TIMEOUT_SECS[\s\S]{1,400}SCREENSAVER_MAX_TIMEOUT_SECS[\s\S]{1,400}api\.screensaverPatch\(\{ timeout_secs: screensaverTimeoutSecs \}\);[\s\S]{1,400}await loadScreensaverState\(\);/,
    );
  });

  test("commit PIN validates match + hashes with drive root salt + posts", () => {
    expect(panel).toMatch(
      /async function commitPin\(\): Promise<void> \{[\s\S]{1,600}if \(pin1 !== pin2\) \{[\s\S]{1,200}screensaverError = "PINs don't match";[\s\S]{1,400}const salt = drive\.info\?\.root \?\? "";[\s\S]{1,200}const hash = await hashPin\(pin1, salt\);[\s\S]{1,200}api\.screensaverSetPin\(hash\);[\s\S]{1,400}await loadScreensaverState\(\);/,
    );
  });

  test("clearPin calls screensaverClearPin + reloads", () => {
    expect(panel).toMatch(
      /async function clearPin\(\): Promise<void> \{[\s\S]{1,400}api\.screensaverClearPin\(\);[\s\S]{1,400}await loadScreensaverState\(\);/,
    );
  });

  test("Settings overlay pauses the screensaver timer only while visible", () => {
    expect(panel).toMatch(
      /\$effect\(\(\) => \{[\s\S]{1,120}if \(visible\) \{[\s\S]{1,160}screensaverPauseRelease = pauseScreensaverTimer\(\);[\s\S]{1,260}screensaverPauseRelease\?\.\(\);[\s\S]{1,80}screensaverPauseRelease = null;/,
    );
    expect(panel).toMatch(
      /return \(\) => \{[\s\S]{1,200}screensaverPauseRelease\?\.\(\);[\s\S]{1,80}screensaverPauseRelease = null;/,
    );
  });

  test("markup renders the screen-lock row with the enable toggle", () => {
    expect(panel).toMatch(
      /<section class="screen-lock">[\s\S]{1,2000}<h3>Screen lock<\/h3>/,
    );
    expect(panel).toMatch(
      /onchange=\{toggleScreensaverEnabled\}/,
    );
  });

  test("timeout input + PIN buttons gated on enabled=true", () => {
    expect(panel).toMatch(
      /\{#if screensaverEnabled === true\}[\s\S]{1,4000}bind:value=\{screensaverTheme\}[\s\S]{1,4000}bind:value=\{screensaverTimeoutSecs\}/,
    );
    expect(panel).toMatch(/onclick=\{openPinDialog\}/);
    expect(panel).toMatch(/onclick=\{clearPin\}/);
  });

  test("inline PIN dialog binds pin1/pin2 + wires save+cancel", () => {
    expect(panel).toMatch(
      /\{#if pinDialog === null\}[\s\S]{1,4000}\{:else\}[\s\S]{1,2000}bind:value=\{pinDialog\.pin1\}[\s\S]{1,400}bind:value=\{pinDialog\.pin2\}[\s\S]{1,400}onclick=\{commitPin\}[\s\S]{1,200}onclick=\{cancelPinDialog\}/,
    );
  });
});
