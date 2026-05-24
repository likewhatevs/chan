import { describe, expect, test } from "vitest";
import source from "./screensaver.svelte.ts?raw";
import overlay from "../components/ScreensaverOverlay.svelte?raw";
import app from "../App.svelte?raw";

// `fullstack-a-77` slice 2: screensaver state machine +
// overlay component. Tests pin the architectural shape;
// behavioral testing of the inactivity timer happens via
// @@WebtestA's empirical walk + a follow-up integration
// pin in slice 3 if needed.

describe("fullstack-a-77 slice 2: state singleton shape", () => {
  test("singleton declared with the 5 expected fields", () => {
    expect(source).toMatch(
      /export const screensaver = \$state<ScreensaverState>\(\{[\s\S]*?enabled: false,[\s\S]*?timeout_secs: SCREENSAVER_DEFAULT_TIMEOUT_SECS,[\s\S]*?theme: SCREENSAVER_DEFAULT_THEME,[\s\S]*?pin_set: false,[\s\S]*?locked: false,[\s\S]*?loaded: false,/,
    );
  });

  test("ScreensaverState interface carries enabled / timeout / pin_set / locked / loaded", () => {
    expect(source).toMatch(/export interface ScreensaverState \{[\s\S]*?enabled: boolean;[\s\S]*?timeout_secs: number;[\s\S]*?theme: ScreensaverTheme;[\s\S]*?pin_set: boolean;[\s\S]*?locked: boolean;[\s\S]*?loaded: boolean;/);
  });
});

describe("fullstack-a-77 slice 2: state machine helpers", () => {
  test("loadScreensaverState calls api.screensaverState + arms the timer", () => {
    expect(source).toMatch(
      /export async function loadScreensaverState\(\): Promise<void> \{[\s\S]*?const s = await api\.screensaverState\(\);[\s\S]*?screensaver\.loaded = true;[\s\S]*?armInactivityTimer\(\);/,
    );
  });

  test("noteScreensaverActivity short-circuits when locked or disabled", () => {
    expect(source).toMatch(
      /export function noteScreensaverActivity\(\): void \{[\s\S]*?if \(screensaver\.locked\) return;[\s\S]*?if \(!screensaver\.enabled\) return;[\s\S]*?armInactivityTimer\(\);/,
    );
  });

  test("lockNow flips locked + cancels timer", () => {
    expect(source).toMatch(
      /export function lockNow\(\): void \{[\s\S]*?screensaver\.locked = true;[\s\S]*?cancelInactivityTimer\(\);/,
    );
  });

  test("unlockWithPin hashes + verifies + flips locked on success", () => {
    expect(source).toMatch(
      /export async function unlockWithPin\([\s\S]*?const hash = await hashPin\(pin, driveSalt\);[\s\S]*?const result = await api\.screensaverVerify\(hash\);[\s\S]*?if \(result\.verified\) \{[\s\S]*?screensaver\.locked = false;[\s\S]*?armInactivityTimer\(\);/,
    );
  });

  test("pauseScreensaverTimer returns idempotent release fn", () => {
    expect(source).toMatch(
      /export function pauseScreensaverTimer\(\): \(\) => void \{[\s\S]*?pauseCount \+= 1;[\s\S]*?cancelInactivityTimer\(\);[\s\S]*?let released = false;[\s\S]*?if \(released\) return;[\s\S]*?released = true;[\s\S]*?pauseCount = Math\.max\(0, pauseCount - 1\);[\s\S]*?if \(pauseCount === 0\) armInactivityTimer\(\);/,
    );
  });

  test("installScreensaverTracker registers the wider event set", () => {
    expect(source).toMatch(
      /const events = \[[\s\S]*?"keydown",[\s\S]*?"mousedown",[\s\S]*?"touchstart",[\s\S]*?"click",[\s\S]*?"scroll",[\s\S]*?"wheel",[\s\S]*?"pointermove",[\s\S]*?\] as const;/,
    );
  });

  test("timer arming guards on enabled + locked + pause count", () => {
    expect(source).toMatch(
      /function armInactivityTimer\(\): void \{[\s\S]*?if \(!screensaver\.enabled\) return;[\s\S]*?if \(screensaver\.locked\) return;[\s\S]*?if \(pauseCount > 0\) return;/,
    );
  });
});

describe("fullstack-a-77 slice 2: overlay component", () => {
  test("renders only when screensaver.locked is true", () => {
    expect(overlay).toMatch(/\{#if screensaver\.locked\}/);
  });

  test("overlay has aria-modal + role=dialog", () => {
    // `fullstack-a-77c`: backdrop now carries any-input
    // dismiss handlers (onkeydown/onclick/tabindex) for the
    // no-PIN branch. Match the role + aria attrs without
    // pinning the rest of the opening tag.
    expect(overlay).toMatch(
      /class="screensaver-backdrop"[\s\S]{0,400}role="dialog"[\s\S]{0,80}aria-modal="true"[\s\S]{0,80}aria-label="Screen locked"/,
    );
  });

  test("PIN input is password-type + auto-focused on lock", () => {
    expect(overlay).toMatch(/type="password"/);
    expect(overlay).toMatch(
      /\$effect\(\(\) => \{[\s\S]*?if \(!screensaver\.locked\) return;[\s\S]*?inputEl\?\.focus\(\);[\s\S]*?inputEl\?\.select\(\);/,
    );
  });

  test("Enter key triggers submit", () => {
    expect(overlay).toMatch(
      /function onKey\(e: KeyboardEvent\): void \{[\s\S]*?if \(e\.key === "Enter"\) \{[\s\S]*?void submit\(\);/,
    );
  });

  test("submit calls unlockWithPin with the drive root salt", () => {
    expect(overlay).toMatch(
      /const salt = drive\.info\?\.root \?\? "";[\s\S]*?const ok = await unlockWithPin\(pin, salt\);/,
    );
  });

  test("wrong PIN triggers shake + clears input", () => {
    expect(overlay).toMatch(
      /if \(!ok\) \{[\s\S]*?shake = true;[\s\S]*?pin = "";[\s\S]*?setTimeout\(\(\) => \{[\s\S]*?shake = false;[\s\S]*?\}, 400\);/,
    );
  });

  test("CSS animation `screensaver-shake` defined", () => {
    expect(overlay).toMatch(
      /@keyframes screensaver-shake \{[\s\S]*?transform: translateX/,
    );
  });

  test("backdrop z-index is 2000 (above every other overlay)", () => {
    expect(overlay).toMatch(/z-index: 2000;/);
  });

  test("overlay renders Matrix only when the matrix theme is active", () => {
    expect(overlay).toMatch(/import MatrixRain from "\.\/screensaver\/MatrixRain\.svelte";/);
    expect(overlay).toMatch(
      /\{#if screensaver\.theme === "matrix"\}[\s\S]{1,80}<MatrixRain \/>[\s\S]{1,80}\{\/if\}/,
    );
  });
});

describe("fullstack-a-77 slice 2: App.svelte wiring", () => {
  test("App imports installScreensaverTracker + loadScreensaverState", () => {
    expect(app).toMatch(
      /import \{[\s\S]*?installScreensaverTracker,[\s\S]*?loadScreensaverState,[\s\S]*?\} from "\.\/state\/screensaver\.svelte";/,
    );
  });

  test("App imports + mounts ScreensaverOverlay", () => {
    expect(app).toMatch(/import ScreensaverOverlay from "\.\/components\/ScreensaverOverlay\.svelte";/);
    expect(app).toMatch(/<ScreensaverOverlay \/>/);
  });

  test("App calls installScreensaverTracker at onMount", () => {
    expect(app).toMatch(/installScreensaverTracker\(\);/);
  });

  test("App calls loadScreensaverState after bootstrap", () => {
    expect(app).toMatch(/void loadScreensaverState\(\);/);
  });
});
