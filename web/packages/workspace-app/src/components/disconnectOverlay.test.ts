import { describe, expect, test } from "vitest";
import overlay from "./DisconnectOverlay.svelte?raw";

// #A2-frontend: the disconnect overlay is a reconnecting status + a single
// Abandon action. The manual "Retry now" button is gone (the watcher loop
// auto-reconnects, so a manual retry is redundant), and the never-emitted
// "closed" branch is neutralized.
describe("DisconnectOverlay", () => {
  test("drops the manual Retry button and reconnectWatcher wiring", () => {
    expect(overlay).not.toMatch(/Retry now/);
    expect(overlay).not.toMatch(/reconnectWatcher/);
  });

  test("keeps the Abandon action gated on canAbandon, via the unchanged IPC", () => {
    expect(overlay).toMatch(
      /const canAbandon = isTauriDesktop\(\) && windowLibraryId\(\) !== "local"/,
    );
    expect(overlay).toMatch(/\{#if canAbandon\}[\s\S]{1,200}onclick=\{abandon\}/);
    // The abandon contract with @@CLI is unchanged: the UI invokes the same
    // desktop IPC wrapper, which calls abandon_devserver_for_window.
    expect(overlay).toMatch(/abandonDevserverForWindow\(\)/);
  });

  test("neutralizes the never-emitted 'closed' branch", () => {
    expect(overlay).not.toMatch(/disconnected from the chan server/);
    expect(overlay).not.toMatch(/the server may have stopped/);
  });

  // The overlay adopts the desktop connecting screen's retry readout: a live
  // elapsed timer and an "attempt N" counter driven by the watcher transport's
  // reconnect count (`ui.wsAttempt`), alongside the existing spinner.
  test("shows the retry-counter presentation (attempt count + elapsed timer)", () => {
    expect(overlay).toMatch(/ui\.wsAttempt/);
    expect(overlay).toMatch(/function fmtElapsed/);
    expect(overlay).toMatch(/attempt \$\{ui\.wsAttempt\}/);
    expect(overlay).toMatch(/class="meta"/);
  });
});
