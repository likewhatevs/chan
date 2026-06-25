// Pin the desktop download capability's shape so the contract
// the inspector Download button relies on stays stable. Source
// asserts (`?raw`) mirror the repo's other capability-shape tests.

import { describe, expect, test } from "vitest";
import desktop from "./desktop.ts?raw";

describe("desktop download capability", () => {
  test("runDesktopDownload fetches with XHR progress and saves via Tauri", () => {
    expect(desktop).toContain("export async function runDesktopDownload(");
    // XHR download (not upload) progress workspaces the indicator.
    expect(desktop).toContain("xhr.responseType = \"arraybuffer\"");
    expect(desktop).toContain("xhr.onprogress");
    expect(desktop).toContain("event.loaded / event.total");
    // A streamed-tar directory download has no Content-Length, so the progress
    // is guarded to an indeterminate `null` (the bubble renders the moving bar)
    // rather than dividing by a zero/absent total into a NaN%.
    expect(desktop).toContain("event.lengthComputable && event.total > 0");
    // Hands the finished bytes to the Tauri command as a Vec<u8>.
    expect(desktop).toContain('tauriInvoke<{ path: string }>(\n      "save_file_to_downloads"');
    expect(desktop).toContain("Array.from(bytes)");
  });

  test("it gates on isTauriDesktop and drives the unified transfer model", () => {
    expect(desktop).toContain('runDesktopDownload called outside chan-desktop');
    // The transfer bubble is the single download surface (the old per-flow
    // downloadTransfer store is retired).
    expect(desktop).toContain("beginTransfer({");
    expect(desktop).toContain("finishTransfer(xferId, saved.path)");
    expect(desktop).toContain("failTransfer(xferId, message)");
    // The model-backed cancel aborts the in-flight fetch.
    expect(desktop).toContain("() => xhr.abort()");
  });
});
