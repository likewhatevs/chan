import { describe, expect, test } from "vitest";
import { Terminal } from "@xterm/xterm";

// A2 (htop-after-reload): the backend re-asserts the live DEC private-mode set
// (DECCKM + mouse + bracketed paste) on reattach. This proves the FIX REACHES
// THE RENDERER: xterm.js natively honors those `\e[?…h/l` output sequences via
// its core mode service, so no SPA-side (keymap.ts) mode tracking is needed —
// plain arrow keys are encoded by xterm.js from `applicationCursorKeysMode`, and
// keymap.ts only intercepts Enter + Alt+arrows (verified: terminalMetaKeyBytes
// returns null for a bare arrow). If a reattach drops the re-assert, these modes
// stay at their post-reload defaults and arrows/wheel break.

function write(term: Terminal, data: string): Promise<void> {
  return new Promise((resolve) => term.write(data, resolve));
}

describe("xterm honors re-asserted PTY private modes", () => {
  test("DECCKM / mouse / bracketed-paste set+reset from the output stream", async () => {
    const term = new Terminal();
    // Post-reload defaults: a fresh terminal comes up with everything off.
    expect(term.modes.applicationCursorKeysMode).toBe(false);
    expect(term.modes.mouseTrackingMode).toBe("none");
    expect(term.modes.bracketedPasteMode).toBe(false);

    // The exact re-assert the backend emits for an htop session
    // (`\e[?1h` DECCKM, `\e[?1000h` mouse, `\e[?1006h` SGR encoding).
    await write(term, "\x1b[?1h\x1b[?1000h\x1b[?1006h\x1b[?2004h");
    expect(term.modes.applicationCursorKeysMode).toBe(true);
    expect(term.modes.mouseTrackingMode).not.toBe("none");
    expect(term.modes.bracketedPasteMode).toBe(true);

    // And the resets clear them, so the tracker mirrors reality.
    await write(term, "\x1b[?1l\x1b[?1000l\x1b[?2004l");
    expect(term.modes.applicationCursorKeysMode).toBe(false);
    expect(term.modes.mouseTrackingMode).toBe("none");
    expect(term.modes.bracketedPasteMode).toBe(false);

    term.dispose();
  });
});
