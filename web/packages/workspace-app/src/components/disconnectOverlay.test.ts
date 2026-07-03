import { describe, expect, test } from "vitest";
import overlay from "./DisconnectOverlay.svelte?raw";

// The disconnect overlay is a reconnecting status plus recovery actions on a
// devserver-backed desktop window: a primary Reconnect (force-close the dead
// control terminal + re-dial) and Abandon (give up). The manual "Retry now"
// button is gone (the watcher loop auto-reconnects), the never-emitted "closed"
// branch is neutralized, and the standing subline is removed everywhere.
describe("DisconnectOverlay", () => {
  test("drops the manual Retry button and reconnectWatcher wiring", () => {
    expect(overlay).not.toMatch(/Retry now/);
    expect(overlay).not.toMatch(/reconnectWatcher/);
  });

  test("offers Reconnect + Abandon, gated on canRecover, via the desktop IPC", () => {
    expect(overlay).toMatch(
      /const canRecover = isTauriDesktop\(\) && windowLibraryId\(\) !== "local"/,
    );
    // Both recovery buttons render together under the same desktop gate.
    expect(overlay).toMatch(
      /\{#if canRecover\}[\s\S]{1,300}onclick=\{reconnect\}[\s\S]{1,200}onclick=\{abandon\}/,
    );
    // Each button invokes its own desktop IPC wrapper (best-effort).
    expect(overlay).toMatch(/reconnectDevserverForWindow\(\)/);
    expect(overlay).toMatch(/abandonDevserverForWindow\(\)/);
  });

  test("removes the standing subline (Q7=b: everywhere)", () => {
    expect(overlay).not.toMatch(/this usually clears on its own/);
    expect(overlay).not.toMatch(/class="subline"/);
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
