// Desktop-native download-progress store that the inspector's indicator
// binds to. Tests pin the lifecycle so the wiring contract stays stable.

import { afterEach, describe, expect, test } from "vitest";
import {
  beginDownloadTransfer,
  clearDownloadTransfer,
  downloadTransfer,
  downloadTransferActive,
  failDownloadTransfer,
  finishDownloadTransfer,
  setDownloadProgress,
} from "./downloadTransfer.svelte";

afterEach(() => {
  clearDownloadTransfer();
});

describe("downloadTransfer store lifecycle", () => {
  test("idle by default", () => {
    expect(downloadTransfer.value).toBeNull();
    expect(downloadTransferActive()).toBe(false);
  });

  test("begin -> progress -> finish", () => {
    const cancel = () => {};
    beginDownloadTransfer("note.md", cancel);
    expect(downloadTransfer.value).toMatchObject({
      filename: "note.md",
      progress: null,
      savedPath: null,
      error: null,
    });
    expect(downloadTransfer.value?.cancel).toBe(cancel);
    expect(downloadTransferActive()).toBe(true);

    setDownloadProgress(0.5);
    expect(downloadTransfer.value?.progress).toBe(0.5);

    finishDownloadTransfer("/Users/x/Downloads/note.md");
    expect(downloadTransfer.value).toMatchObject({
      progress: 1,
      cancel: null,
      savedPath: "/Users/x/Downloads/note.md",
      error: null,
    });
    // No longer "active" once a savedPath lands.
    expect(downloadTransferActive()).toBe(false);
  });

  test("begin -> fail records the error and stops being active", () => {
    beginDownloadTransfer("big.tar", () => {});
    failDownloadTransfer("download failed: HTTP 500");
    expect(downloadTransfer.value).toMatchObject({
      cancel: null,
      error: "download failed: HTTP 500",
      savedPath: null,
    });
    expect(downloadTransferActive()).toBe(false);
  });

  test("indeterminate progress stays null", () => {
    beginDownloadTransfer("stream.bin", null);
    setDownloadProgress(null);
    expect(downloadTransfer.value?.progress).toBeNull();
    // active even with an indeterminate (null) progress until it
    // finishes or fails.
    expect(downloadTransferActive()).toBe(true);
  });

  test("clear resets to idle", () => {
    beginDownloadTransfer("x.md", () => {});
    finishDownloadTransfer("/p/x.md");
    clearDownloadTransfer();
    expect(downloadTransfer.value).toBeNull();
  });

  test("setters are no-ops when there is no active transfer", () => {
    // Guards the inspector against a late event after clear.
    setDownloadProgress(0.9);
    finishDownloadTransfer("/p");
    failDownloadTransfer("e");
    expect(downloadTransfer.value).toBeNull();
  });
});
