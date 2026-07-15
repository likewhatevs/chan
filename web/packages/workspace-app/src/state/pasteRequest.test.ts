// @vitest-environment jsdom
//
// The `cs paste` pending race and the paste card's reply contract: a read
// that settles inside the threshold answers with NO card; a parked read
// raises the card; [Paste] runs ONE fresh gesture-bound read; [Cancel]
// answers the CLI immediately; a stale reply's 404 is swallowed; a newer
// request replaces a stale card without the old settle dismissing it.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const windowReply = vi.fn<(reply: unknown) => Promise<void>>();
vi.mock("../api/client", () => ({
  api: { windowReply: (reply: unknown) => windowReply(reply) },
}));

const readClipboardPayload = vi.fn<() => Promise<{ mime: string; bytes: Uint8Array }>>();
const readWebClipboardPayload = vi.fn<() => Promise<{ mime: string; bytes: Uint8Array }>>();
vi.mock("../api/clipboard", () => ({
  bytesToBase64: (bytes: Uint8Array) => btoa(String.fromCharCode(...bytes)),
  hintClipboardError: (e: unknown) => (e instanceof Error ? e.message : String(e)),
  readClipboardPayload: () => readClipboardPayload(),
  readWebClipboardPayload: () => readWebClipboardPayload(),
}));

import {
  PASTE_CARD_PENDING_MS,
  cancelPasteCard,
  confirmPasteCard,
  pasteRequestState,
  respondClipboardRead,
  warnUnlessStaleReply,
} from "./pasteRequest.svelte";

/// A promise plus its out-of-band settle handles, standing in for a
/// clipboard read parked on a browser permission prompt.
function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const payload = { mime: "text/plain;charset=utf-8", bytes: new Uint8Array([65, 66]) };
const payloadReply = { mime: "text/plain;charset=utf-8", data_b64: btoa("AB") };

beforeEach(() => {
  vi.useFakeTimers();
  windowReply.mockResolvedValue(undefined);
});

afterEach(async () => {
  // Drain a card a test left showing so it cannot leak into the next one.
  if (pasteRequestState.card) {
    pasteRequestState.card.busy = false;
    await cancelPasteCard();
  }
  vi.useRealTimers();
  vi.clearAllMocks();
});

describe("respondClipboardRead pending race", () => {
  it("a fast settle replies with no card", async () => {
    readClipboardPayload.mockResolvedValue(payload);
    await respondClipboardRead("r1", "auto");
    expect(pasteRequestState.card).toBeNull();
    expect(windowReply).toHaveBeenCalledExactlyOnceWith({
      requestId: "r1",
      payload: payloadReply,
    });
  });

  it("a fast denial replies the error with no card", async () => {
    readClipboardPayload.mockRejectedValue(new Error("clipboard access denied"));
    await respondClipboardRead("r1", "auto");
    expect(pasteRequestState.card).toBeNull();
    expect(windowReply).toHaveBeenCalledExactlyOnceWith({
      requestId: "r1",
      payload: { error: "clipboard access denied" },
    });
  });

  it("a read still pending at the threshold raises the card", async () => {
    readClipboardPayload.mockReturnValue(deferred<typeof payload>().promise);
    const done = respondClipboardRead("r1", "auto");
    await vi.advanceTimersByTimeAsync(PASTE_CARD_PENDING_MS - 1);
    expect(pasteRequestState.card).toBeNull();
    await vi.advanceTimersByTimeAsync(1);
    await done;
    expect(pasteRequestState.card).toMatchObject({ requestId: "r1", prefer: "auto" });
    expect(windowReply).not.toHaveBeenCalled();
  });

  it("the original read settling while the card shows replies and dismisses it", async () => {
    const read = deferred<typeof payload>();
    readClipboardPayload.mockReturnValue(read.promise);
    const done = respondClipboardRead("r1", "auto");
    await vi.advanceTimersByTimeAsync(PASTE_CARD_PENDING_MS);
    await done;
    expect(pasteRequestState.card?.requestId).toBe("r1");
    // The user clicked the BROWSER's own permission UI instead of the card:
    // the original read resolves, replies, and takes the card down without
    // the card's buttons ever being touched.
    read.resolve(payload);
    await vi.advanceTimersByTimeAsync(0);
    expect(windowReply).toHaveBeenCalledExactlyOnceWith({
      requestId: "r1",
      payload: payloadReply,
    });
    expect(pasteRequestState.card).toBeNull();
  });
});

describe("paste card actions", () => {
  async function raiseCard(requestId = "r1"): Promise<void> {
    readClipboardPayload.mockReturnValue(deferred<typeof payload>().promise);
    const done = respondClipboardRead(requestId, "auto");
    await vi.advanceTimersByTimeAsync(PASTE_CARD_PENDING_MS);
    await done;
    expect(pasteRequestState.card?.requestId).toBe(requestId);
  }

  it("Paste runs one fresh web read and replies its payload", async () => {
    await raiseCard();
    readWebClipboardPayload.mockResolvedValue(payload);
    await confirmPasteCard();
    expect(readWebClipboardPayload).toHaveBeenCalledTimes(1);
    expect(windowReply).toHaveBeenCalledExactlyOnceWith({
      requestId: "r1",
      payload: payloadReply,
    });
    expect(pasteRequestState.card).toBeNull();
  });

  it("a denied Paste click replies the hinted error", async () => {
    await raiseCard();
    readWebClipboardPayload.mockRejectedValue(new Error("denied again"));
    await confirmPasteCard();
    expect(windowReply).toHaveBeenCalledExactlyOnceWith({
      requestId: "r1",
      payload: { error: "denied again" },
    });
    expect(pasteRequestState.card).toBeNull();
  });

  it("Cancel replies the cancellation error immediately", async () => {
    await raiseCard();
    await cancelPasteCard();
    expect(windowReply).toHaveBeenCalledExactlyOnceWith({
      requestId: "r1",
      payload: { error: "paste cancelled in the window" },
    });
    expect(pasteRequestState.card).toBeNull();
  });

  it("a stale reply's 404 is swallowed and still dismisses the card", async () => {
    await raiseCard();
    windowReply.mockRejectedValue({ status: 404 });
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    await cancelPasteCard();
    expect(pasteRequestState.card).toBeNull();
    expect(warn).not.toHaveBeenCalled();
    warn.mockRestore();
  });

  it("a newer request replaces the card; the old settle leaves it up", async () => {
    const first = deferred<typeof payload>();
    readClipboardPayload.mockReturnValueOnce(first.promise);
    const firstDone = respondClipboardRead("r1", "auto");
    readClipboardPayload.mockReturnValueOnce(deferred<typeof payload>().promise);
    const secondDone = respondClipboardRead("r2", "text");
    await vi.advanceTimersByTimeAsync(PASTE_CARD_PENDING_MS);
    await firstDone;
    await secondDone;
    expect(pasteRequestState.card).toMatchObject({ requestId: "r2", prefer: "text" });
    // The old request's read settles late: its reply POSTs (the bus 404s a
    // stale id server-side) but the NEWER card stays.
    first.resolve(payload);
    await vi.advanceTimersByTimeAsync(0);
    expect(windowReply).toHaveBeenCalledExactlyOnceWith({
      requestId: "r1",
      payload: payloadReply,
    });
    expect(pasteRequestState.card?.requestId).toBe("r2");
  });
});

describe("warnUnlessStaleReply", () => {
  it("swallows a 404 and warns on anything else", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    warnUnlessStaleReply({ status: 404 });
    expect(warn).not.toHaveBeenCalled();
    warnUnlessStaleReply({ status: 413 });
    expect(warn).toHaveBeenCalledTimes(1);
    warn.mockRestore();
  });
});
