import { describe, expect, test, vi } from "vitest";
import { handleTerminalMetaKey, terminalMetaKeyBytes } from "./keymap";

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
});
