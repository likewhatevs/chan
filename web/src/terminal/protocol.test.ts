import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";
import tab from "../components/TerminalTab.svelte?raw";
import session from "./session.ts?raw";

const route = readFileSync("../crates/chan-server/src/routes/terminal.rs", "utf8");

describe("terminal protocol invariants", () => {
  test("reattach asks the server for a full replay ring", () => {
    expect(session).toContain('params.set("since", "0")');
    expect(tab).not.toContain("lastSeq");
  });

  test("server attach prelude sends control, binary replay, alt-screen prelude, then ready", () => {
    const prelude = route.match(/async fn send_attach_prelude[\s\S]*?\n}\n\nfn terminal_cwd_payload/)?.[0];
    expect(prelude).toBeTruthy();
    const sessionFrame = prelude!.indexOf("ServerFrame::Session");
    const replay = prelude!.indexOf("for chunk in &session.replay");
    const altScreen = prelude!.indexOf("ALT_SCREEN_ATTACH_PRELUDE");
    const ready = prelude!.indexOf("ServerFrame::Ready");

    expect(sessionFrame).toBeGreaterThanOrEqual(0);
    expect(replay).toBeGreaterThan(sessionFrame);
    expect(altScreen).toBeGreaterThan(replay);
    expect(ready).toBeGreaterThan(altScreen);
  });

  test("PTY output remains binary on both sides of the websocket", () => {
    expect(route).toMatch(/SessionEvent::Output\(data\)[\s\S]*?Message::Binary\(data\)/);
    expect(route).toMatch(/socket\.send\(Message::Binary\(chunk\.clone\(\)\)\)/);
    expect(tab).toContain('ws.binaryType = "arraybuffer"');
    expect(tab).toContain("terminalMessageBytes(event.data)");
    expect(tab).not.toMatch(/term\?\.write\(String\(/);
  });

  test("client sends initial and resize-observed PtySize frames", () => {
    expect(tab).toMatch(/ws\.onopen = \(\) => \{[\s\S]*?send\(\{ type: "resize", cols: term\.cols, rows: term\.rows \}\)/);
    expect(tab).toMatch(/term\.onResize\(\(\{ cols, rows \}\) => send\(\{ type: "resize", cols, rows \}\)\)/);
    expect(route).toMatch(/ClientFrame::Resize \{ cols, rows \}[\s\S]*?session\.resize\(pty_size\(Some\(cols\), Some\(rows\)\)\)/);
  });

  test("terminal-generated replies bypass broadcast and are not replay-gated", () => {
    expect(tab).toContain("routeXtermData(data, ptyWrites, sendInput, sendUserInput)");
    expect(tab).not.toContain("replayingReattach");
    expect(tab).not.toContain("clearReplayWhenDrained");
  });
});
