// Bug 2b: pin the desktop download capability's shape so the contract
// @@LaneA wires the inspector Download button to stays stable. Source
// asserts (`?raw`) mirror the repo's other capability-shape tests.

import { describe, expect, test } from "vitest";
import desktop from "./desktop.ts?raw";

describe("desktop download capability", () => {
  test("runDesktopDownload fetches with XHR progress and saves via Tauri", () => {
    expect(desktop).toContain("export async function runDesktopDownload(");
    // XHR download (not upload) progress drives the indicator.
    expect(desktop).toContain("xhr.responseType = \"arraybuffer\"");
    expect(desktop).toContain("xhr.onprogress");
    expect(desktop).toContain("event.loaded / event.total");
    // Hands the finished bytes to the Tauri command as a Vec<u8>.
    expect(desktop).toContain('tauriInvoke<{ path: string }>(\n      "save_file_to_downloads"');
    expect(desktop).toContain("Array.from(bytes)");
  });

  test("it gates on isTauriDesktop and drives the transfer store", () => {
    expect(desktop).toContain('runDesktopDownload called outside chan-desktop');
    expect(desktop).toContain("beginDownloadTransfer(filename");
    expect(desktop).toContain("finishDownloadTransfer(saved.path)");
    expect(desktop).toContain("failDownloadTransfer(message)");
    // The store-backed cancel aborts the in-flight fetch.
    expect(desktop).toContain("() => xhr.abort()");
  });
});
