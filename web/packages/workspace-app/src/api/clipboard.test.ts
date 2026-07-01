import { afterEach, describe, expect, test, vi } from "vitest";

// The bridge branches desktop-native vs navigator.clipboard; pin it to the
// desktop path so the tests exercise the representation logic without a
// jsdom navigator.clipboard.
vi.mock("./desktop", () => ({
  isTauriDesktop: vi.fn(() => true),
  readClipboardText: vi.fn(),
  readClipboardImage: vi.fn(),
  readClipboardHtml: vi.fn(),
  writeClipboardText: vi.fn(() => Promise.resolve()),
  writeClipboardImage: vi.fn(() => Promise.resolve()),
  writeClipboardHtml: vi.fn(() => Promise.resolve()),
}));

import {
  base64ToBytes,
  bytesToBase64,
  hintClipboardError,
  readClipboardPayload,
  writeClipboardPayload,
} from "./clipboard";
import * as desktop from "./desktop";

const enc = (s: string) => new TextEncoder().encode(s);

afterEach(() => vi.clearAllMocks());

describe("base64 round-trip", () => {
  test("bytesToBase64 / base64ToBytes are inverses over arbitrary bytes", () => {
    const bytes = new Uint8Array([0, 1, 2, 253, 254, 255, 65, 66]);
    expect(Array.from(base64ToBytes(bytesToBase64(bytes)))).toEqual(Array.from(bytes));
  });
});

describe("writeClipboardPayload", () => {
  test("plain text writes through writeClipboardText", async () => {
    await writeClipboardPayload("text/plain;charset=utf-8", enc("hello"));
    expect(desktop.writeClipboardText).toHaveBeenCalledWith("hello");
  });

  test("html writes html plus a derived plain-text fallback", async () => {
    await writeClipboardPayload("text/html", enc("<p>hi <b>there</b></p>"));
    expect(desktop.writeClipboardHtml).toHaveBeenCalledWith("<p>hi <b>there</b></p>", "hi there");
  });
});

describe("readClipboardPayload", () => {
  test("auto is image-first", async () => {
    vi.mocked(desktop.readClipboardImage).mockResolvedValue(new Uint8Array([1, 2, 3]));
    const payload = await readClipboardPayload("auto");
    expect(payload.mime).toBe("image/png");
    expect(Array.from(payload.bytes)).toEqual([1, 2, 3]);
    expect(desktop.readClipboardText).not.toHaveBeenCalled();
  });

  test("auto falls back to text when no image is present", async () => {
    vi.mocked(desktop.readClipboardImage).mockResolvedValue(null);
    vi.mocked(desktop.readClipboardText).mockResolvedValue("clipboard words");
    const payload = await readClipboardPayload("auto");
    expect(payload.mime).toBe("text/plain;charset=utf-8");
    expect(new TextDecoder().decode(payload.bytes)).toBe("clipboard words");
  });

  test("text prefer never reads the image", async () => {
    vi.mocked(desktop.readClipboardText).mockResolvedValue("just text");
    const payload = await readClipboardPayload("text");
    expect(payload.mime).toBe("text/plain;charset=utf-8");
    expect(desktop.readClipboardImage).not.toHaveBeenCalled();
  });

  test("an empty clipboard throws", async () => {
    vi.mocked(desktop.readClipboardImage).mockResolvedValue(null);
    vi.mocked(desktop.readClipboardText).mockResolvedValue("");
    await expect(readClipboardPayload("auto")).rejects.toThrow(/empty/);
  });

  test("an over-cap clipboard payload is refused (not base64'd)", async () => {
    // 32 MiB + 1 byte exceeds MAX_CLIPBOARD_BYTES -> reject before building a
    // giant base64 string that the reply route would 413.
    vi.mocked(desktop.readClipboardImage).mockResolvedValue(
      new Uint8Array(32 * 1024 * 1024 + 1),
    );
    await expect(readClipboardPayload("image")).rejects.toThrow(/too large/);
  });
});

describe("readImagePayload label (web path)", () => {
  test("uses the actual clipboard image type, not a hardcoded png", async () => {
    vi.mocked(desktop.isTauriDesktop).mockReturnValue(false);
    const jpeg = new Uint8Array([0xff, 0xd8, 0xff, 0xe0]);
    const item = {
      types: ["image/jpeg"],
      getType: async () => new Blob([jpeg], { type: "image/jpeg" }),
    };
    Object.defineProperty(navigator, "clipboard", {
      value: { read: async () => [item] },
      configurable: true,
    });
    const payload = await readClipboardPayload("image");
    expect(payload.mime).toBe("image/jpeg");
  });
});

describe("hintClipboardError", () => {
  test("maps a permission denial to an actionable hint", () => {
    const err = new DOMException("Read permission denied.", "NotAllowedError");
    expect(hintClipboardError(err)).toMatch(/denied|permission/i);
  });

  test("passes through an unrelated message", () => {
    expect(hintClipboardError(new Error("boom"))).toBe("boom");
  });
});
