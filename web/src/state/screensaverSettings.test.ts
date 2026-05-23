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
  test("Settings imports hashPin + the min/max bounds + pause helper", () => {
    expect(panel).toMatch(
      /import \{[\s\S]{1,400}hashPin,[\s\S]{1,200}SCREENSAVER_MAX_TIMEOUT_SECS,[\s\S]{1,80}SCREENSAVER_MIN_TIMEOUT_SECS,[\s\S]{1,40}\} from "\.\.\/state\/screensaver";/,
    );
    expect(panel).toMatch(
      /import \{[\s\S]{1,200}loadScreensaverState,[\s\S]{1,80}pauseScreensaverTimer,[\s\S]{1,40}\} from "\.\.\/state\/screensaver\.svelte";/,
    );
  });

  test("Settings carries the slice-3 reactive state vars", () => {
    expect(panel).toMatch(
      /let screensaverEnabled = \$state<boolean \| null>\(null\);/,
    );
    expect(panel).toMatch(/let screensaverTimeoutSecs = \$state<number>\(300\);/);
    expect(panel).toMatch(/let screensaverTheme = \$state<ScreensaverTheme>\("matrix"\);/);
    expect(panel).toMatch(/let screensaverPinSet = \$state\(false\);/);
    expect(panel).toMatch(/let screensaverBusy = \$state\(false\);/);
    expect(panel).toMatch(/let screensaverError = \$state<string \| null>\(null\);/);
    expect(panel).toMatch(
      /let pinDialog = \$state<\{ pin1: string; pin2: string \} \| null>\(null\);/,
    );
  });

  test("loadFeaturesState fetches screensaver state via api.screensaverState", () => {
    expect(panel).toMatch(
      /const s = await api\.screensaverState\(\);[\s\S]{1,200}screensaverEnabled = s\.enabled;[\s\S]{1,200}screensaverTimeoutSecs = s\.timeout_secs;[\s\S]{1,200}screensaverTheme = s\.theme;[\s\S]{1,200}screensaverPinSet = s\.pin_set;/,
    );
  });

  test("theme picker persists matrix/castaway through screensaverPatch", () => {
    expect(panel).toMatch(/type ScreensaverTheme/);
    expect(panel).toMatch(
      /async function commitScreensaverTheme\(e: Event\): Promise<void> \{[\s\S]{1,700}api\.screensaverPatch\(\{ theme \}\);[\s\S]{1,300}await loadScreensaverState\(\);/,
    );
    expect(panel).toMatch(
      /<select[\s\S]{1,300}bind:value=\{screensaverTheme\}[\s\S]{1,200}onchange=\{commitScreensaverTheme\}[\s\S]{1,300}<option value="matrix">Matrix<\/option>[\s\S]{1,120}<option value="castaway">Castaway<\/option>/,
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

  test("Settings overlay pauses the screensaver timer on mount + releases on destroy", () => {
    expect(panel).toMatch(
      /screensaverPauseRelease = pauseScreensaverTimer\(\);[\s\S]{1,400}return \(\) => \{[\s\S]{1,200}screensaverPauseRelease\?\.\(\);[\s\S]{1,80}screensaverPauseRelease = null;/,
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
