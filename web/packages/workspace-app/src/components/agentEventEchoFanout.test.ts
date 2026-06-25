import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";

// Broadcast survey-reply echo fan-out. The SPA-side intercept routes
// the `poke + chord` payload through `sendUserInput` so the broadcast
// layer fans the echo to selected broadcast targets. Server-side
// `dispatch_agent_event` emits an `agent_event_echo` WS frame instead
// of writing the bytes directly to the PTY.

describe("ServerFrame discriminator", () => {
  test("ServerFrame union includes `agent_event_echo` variant", () => {
    expect(terminal).toMatch(
      /type: "agent_event_echo";[\s\S]*?seq: number;[\s\S]*?event_id: string;[\s\S]*?payload_b64: string;/,
    );
  });

  test("rationale comment cites the broadcast-layer reuse + base64 framing", () => {
    expect(terminal).toMatch(/broadcast layer fans the echo/i);
    expect(terminal).toMatch(/base64[\s\S]{1,80}non-UTF8/i);
  });
});

describe("WS handler routes the payload through sendUserInput", () => {
  test("agent_event_echo branch decodes + calls sendUserInput", () => {
    expect(terminal).toMatch(
      /else if \(frame\.type === "agent_event_echo"\) \{[\s\S]*?const payload = decodeAgentEventEcho\(frame\.payload_b64\);[\s\S]*?if \(payload\) \{[\s\S]*?sendUserInput\(payload\);/,
    );
  });

  test("agent_event_echo branch records replay sequence after injection", () => {
    expect(terminal).toMatch(
      /tab\.lastAgentEchoSeq = Math\.max\([\s\S]*?Math\.floor\(tab\.lastAgentEchoSeq \?\? 0\),[\s\S]*?Math\.floor\(frame\.seq\),/,
    );
  });

  test("decodeAgentEventEcho uses `atob` + null-soft on malformed b64", () => {
    expect(terminal).toMatch(
      /function decodeAgentEventEcho\(payload_b64: string\): string \| null \{[\s\S]*?const binary = atob\(payload_b64\);[\s\S]*?return binary;[\s\S]*?\} catch \{[\s\S]*?return null;/,
    );
  });

  test("sendUserInput preserves the broadcast fan-out (sendInput + broadcastTerminalInput pair)", () => {
    // Sanity check: the helper that the echo routes through is
    // the same one user-typed input uses, so broadcast fan-out
    // is automatic.
    expect(terminal).toMatch(
      /function sendUserInput\(data: string\): void \{[\s\S]*?sendInput\(data\);[\s\S]*?broadcastTerminalInput\(tab, data\);/,
    );
  });
});
