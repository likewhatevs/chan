import { describe, expect, test, vi } from "vitest";
import {
  applyKittyKeyboardProtocol,
  applyXtermModifierKeys,
  createTerminalKeyboardProtocolState,
  disableXtermModifierKeys,
  handleTerminalMetaKey,
  queryXtermModifierKeys,
  resetTerminalKeyboardProtocolState,
  terminalMetaKeyBytes,
} from "./keymap";

function keyEvent(init: KeyboardEventInit & { type?: string }): KeyboardEvent {
  const { type = "keydown", ...eventInit } = init;
  return new KeyboardEvent(type, {
    cancelable: true,
    ...eventInit,
  });
}

describe("terminal meta key mapping", () => {
  test.each([
    ["ArrowLeft", "\x1bb"],
    ["ArrowRight", "\x1bf"],
    ["Backspace", "\x1b\x7f"],
    ["Delete", "\x1bd"],
  ])("maps Alt+%s to readline bytes", (key, bytes) => {
    expect(terminalMetaKeyBytes(keyEvent({ key, altKey: true }))).toBe(bytes);
  });

  test("leaves non-target keys and modified chords to xterm", () => {
    expect(terminalMetaKeyBytes(keyEvent({ key: "d", altKey: true }))).toBeNull();
    expect(terminalMetaKeyBytes(keyEvent({ key: "ArrowLeft", altKey: true, ctrlKey: true }))).toBeNull();
    expect(terminalMetaKeyBytes(keyEvent({ key: "ArrowLeft", altKey: true, metaKey: true }))).toBeNull();
    expect(terminalMetaKeyBytes(keyEvent({ key: "ArrowLeft" }))).toBeNull();
    expect(terminalMetaKeyBytes(keyEvent({ type: "keyup", key: "ArrowLeft", altKey: true }))).toBeNull();
  });

  test("leaves modified Enter to xterm until an app enables enhanced keyboard reporting", () => {
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }))).toBeNull();
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", ctrlKey: true }))).toBeNull();
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", metaKey: true }))).toBeNull();
  });

  test("maps modified Enter to xterm modifyOtherKeys while that mode is active", () => {
    const protocol = createTerminalKeyboardProtocolState();
    applyXtermModifierKeys(protocol, [4, 2]);

    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBe(
      "\x1b[27;2;13~",
    );
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", ctrlKey: true }), protocol)).toBe(
      "\x1b[27;5;13~",
    );
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", metaKey: true }), protocol)).toBe(
      "\x1b[27;9;13~",
    );
    expect(
      terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true, ctrlKey: true }), protocol),
    ).toBeNull();
  });

  test("maps modified Enter to CSI-u when kitty report-all-keys is active", () => {
    const protocol = createTerminalKeyboardProtocolState();
    applyKittyKeyboardProtocol(protocol, "push", [8]);

    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBe(
      "\x1b[13;2u",
    );
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", ctrlKey: true }), protocol)).toBe(
      "\x1b[13;5u",
    );
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", metaKey: true }), protocol)).toBe(
      "\x1b[13;9u",
    );
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", ctrlKey: true, metaKey: true }))).toBeNull();
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", altKey: true }))).toBeNull();
  });

  test("does not treat kitty disambiguate-only mode as report-all Enter mode", () => {
    const protocol = createTerminalKeyboardProtocolState();
    applyKittyKeyboardProtocol(protocol, "push", [1]);

    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBeNull();
  });

  test("sends matched bytes and suppresses xterm default handling", () => {
    const sendInput = vi.fn();
    const ev = keyEvent({ key: "Backspace", altKey: true });

    expect(handleTerminalMetaKey(ev, sendInput)).toBe(false);
    expect(sendInput).toHaveBeenCalledWith("\x1b\x7f");
    expect(ev.defaultPrevented).toBe(true);
  });

  test("returns true without sending for keys xterm should handle", () => {
    const sendInput = vi.fn();
    const ev = keyEvent({ key: "d", altKey: true });

    expect(handleTerminalMetaKey(ev, sendInput)).toBe(true);
    expect(sendInput).not.toHaveBeenCalled();
    expect(ev.defaultPrevented).toBe(false);
  });

  test("passes modified Enter through when no foreground app requested it", () => {
    const sendInput = vi.fn();
    const ev = keyEvent({ key: "Enter", shiftKey: true });

    expect(handleTerminalMetaKey(ev, sendInput)).toBe(true);
    expect(sendInput).not.toHaveBeenCalled();
    expect(ev.defaultPrevented).toBe(false);
  });

  test("xterm modifyOtherKeys supports query and disable", () => {
    const protocol = createTerminalKeyboardProtocolState();
    applyXtermModifierKeys(protocol, [4, 2]);

    expect(queryXtermModifierKeys(protocol, [4])).toBe("\x1b[>4;2m");
    disableXtermModifierKeys(protocol, [4]);
    expect(queryXtermModifierKeys(protocol, [4])).toBe("\x1b[>4;0m");
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBeNull();
  });

  test("kitty keyboard protocol push, pop, and flag edits are scoped to the active screen", () => {
    const protocol = createTerminalKeyboardProtocolState();

    applyKittyKeyboardProtocol(protocol, "push", [8]);
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBe(
      "\x1b[13;2u",
    );

    protocol.kitty.screen = "alternate";
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBeNull();
    applyKittyKeyboardProtocol(protocol, "set", [8, 2]);
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBe(
      "\x1b[13;2u",
    );
    applyKittyKeyboardProtocol(protocol, "set", [8, 3]);
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBeNull();

    protocol.kitty.screen = "main";
    applyKittyKeyboardProtocol(protocol, "pop", []);
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBeNull();
  });

  test("terminal reset clears negotiated keyboard modes", () => {
    const protocol = createTerminalKeyboardProtocolState();
    applyXtermModifierKeys(protocol, [4, 2]);
    applyKittyKeyboardProtocol(protocol, "push", [8]);

    resetTerminalKeyboardProtocolState(protocol);

    expect(queryXtermModifierKeys(protocol, [4])).toBe("\x1b[>4;0m");
    expect(terminalMetaKeyBytes(keyEvent({ key: "Enter", shiftKey: true }), protocol)).toBeNull();
  });
});
