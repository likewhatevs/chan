import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";

// `fullstack-a-92`: broadcast survey-reply echo fan-out. The
// SPA-side intercept (option 2) routes the `poke + chord`
// payload through `sendUserInput` so the existing `-a-31`
// broadcast layer fans the echo to selected broadcast targets.
// Server-side `dispatch_agent_event` emits an
// `agent_event_echo` WS frame instead of writing the bytes
// directly to the PTY.

describe("fullstack-a-92: ServerFrame discriminator", () => {
  test("ServerFrame union includes `agent_event_echo` variant", () => {
    expect(terminal).toMatch(
      /\| \{ type: "agent_event_echo"; payload_b64: string \};/,
    );
  });

  test("rationale comment cites the broadcast-layer reuse + base64 framing", () => {
    expect(terminal).toMatch(/`fullstack-a-92`/);
    expect(terminal).toMatch(/broadcast layer.*-a-31.*fans the echo/i);
    expect(terminal).toMatch(/base64[\s\S]{1,80}non-UTF8/i);
  });
});

describe("fullstack-a-92: WS handler routes the payload through sendUserInput", () => {
  test("agent_event_echo branch decodes + calls sendUserInput", () => {
    expect(terminal).toMatch(
      /else if \(frame\.type === "agent_event_echo"\) \{[\s\S]*?const payload = decodeAgentEventEcho\(frame\.payload_b64\);[\s\S]*?if \(payload\) sendUserInput\(payload\);/,
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
