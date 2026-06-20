import { describe, expect, test } from "vitest";
import terminalSource from "./TerminalTab.svelte?raw";

// Ghost-tab fix (the @@Desktop CloseReason::Restart contract, wire "restart"):
// a server-side restart() swaps the session in place (same id, fresh shell) and
// closes the old one with reason "restart". The SPA must REATTACH to the same
// session_id instead of dropping the tab (the `explicit` path) or showing
// "session ended" (the idle/shutdown/etc. path). These source-pins keep the
// reattach wired; the real end-to-end reattach is browser-smoked once the
// backend emits "restart".
describe("TerminalTab restart-reattach", () => {
  test("CloseReason carries the restart wire value", () => {
    expect(terminalSource).toMatch(/\|\s*"restart"/);
  });

  test("a closed frame with reason restart reattaches (connect + return) before the ended/clear branch", () => {
    // On restart: reconnect to the same session_id and return BEFORE the
    // session-ended / clear logic, so the tab + its session id survive.
    expect(terminalSource).toMatch(
      /frame\.reason === "restart"\) \{[\s\S]{1,500}void connect\(\);[\s\S]{0,40}return;/,
    );
    // The restart branch precedes clearTerminalSession (it returns first), so
    // restart never zeros the session id the reattach needs.
    const restartIdx = terminalSource.indexOf('frame.reason === "restart"');
    const clearIdx = terminalSource.indexOf("clearTerminalSession(tab)");
    expect(restartIdx).toBeGreaterThan(-1);
    expect(clearIdx).toBeGreaterThan(restartIdx);
  });
});
