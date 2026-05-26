// @vitest-environment jsdom
//
// I1 (inspector consistency + layout): the inspector Download button
// routes through fileOps.downloadPathWithProgress, which must branch on
// isTauriDesktop():
//   - desktop  -> @@LaneB's runDesktopDownload (progress-tracked, drives
//                 the downloadTransfer store the indicator binds to),
//   - browser  -> the existing <a download> hand-off to the native
//                 download manager.
// These tests pin that branch so the desktop integration can't silently
// regress to the browser-only path.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

// Mock the lane-B capability before the store module evaluates so the
// import binding fileOps reads is the mock.
const isTauriDesktop = vi.fn<() => boolean>();
const runDesktopDownload = vi.fn<(url: string, name: string) => Promise<string>>();

vi.mock("../api/desktop", () => ({
  isTauriDesktop: () => isTauriDesktop(),
  runDesktopDownload: (url: string, name: string) =>
    runDesktopDownload(url, name),
}));

let fileOps: typeof import("./store.svelte").fileOps;

beforeEach(async () => {
  vi.resetAllMocks();
  runDesktopDownload.mockResolvedValue("/Users/me/Downloads/note.md");
  ({ fileOps } = await import("./store.svelte"));
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("fileOps.downloadPathWithProgress branch", () => {
  test("desktop routes through runDesktopDownload with a resolved URL", () => {
    isTauriDesktop.mockReturnValue(true);
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click");

    fileOps.downloadPathWithProgress("notes/note.md", false);

    expect(runDesktopDownload).toHaveBeenCalledTimes(1);
    const [url, filename] = runDesktopDownload.mock.calls[0]!;
    // Absolute (resolved against window.location), and the suggested
    // name is the basename (downloadFilename for a file).
    expect(url).toMatch(/^https?:\/\//);
    expect(url).toContain("/api/files/");
    expect(filename).toBe("note.md");
    // The browser <a download> path must NOT fire on desktop.
    expect(click).not.toHaveBeenCalled();
  });

  test("desktop directory download suggests the .tar archive name", () => {
    isTauriDesktop.mockReturnValue(true);

    fileOps.downloadPathWithProgress("notes", true);

    expect(runDesktopDownload).toHaveBeenCalledTimes(1);
    const [, filename] = runDesktopDownload.mock.calls[0]!;
    expect(filename).toBe("notes.tar");
  });

  test("browser falls back to the <a download> manager", () => {
    isTauriDesktop.mockReturnValue(false);
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click");

    fileOps.downloadPathWithProgress("notes/note.md", false);

    expect(runDesktopDownload).not.toHaveBeenCalled();
    expect(click).toHaveBeenCalledTimes(1);
  });
});
