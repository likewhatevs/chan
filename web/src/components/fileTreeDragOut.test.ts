import { describe, expect, test } from "vitest";
import client from "../api/client.ts?raw";
import fileTree from "./FileTree.svelte?raw";
import fileInfo from "./FileInfoBody.svelte?raw";
import store from "../state/store.svelte.ts?raw";

// Bug 2 / round-1: File Browser native drag IN and OUT is removed. The
// macOS native drag-out crashed; the user now exports via the Download
// button and imports via the Upload button. These tests assert the
// drag-out payloads + the native Tauri command are GONE, while the
// app-internal tree-move drag and the Download/Upload buttons stay.
describe("FileTree browser drag-out removed", () => {
  test("api still exposes token-bearing download URLs (for the Download button)", () => {
    expect(client).toMatch(/downloadUrl: \(path: string\) =>/);
    expect(client).toMatch(
      /withTokenQuery\(`\/api\/files\/\$\{encPath\(path\)\}\?download=1`\)/,
    );
  });

  test("file drags no longer carry DownloadURL / uri-list drag-out payloads", () => {
    expect(fileTree).not.toMatch(/setData\(\s*"DownloadURL"/);
    expect(fileTree).not.toMatch(/setData\("text\/uri-list"/);
    // The drag-out helpers are gone entirely.
    expect(fileTree).not.toContain("setDownloadDragData");
    expect(fileTree).not.toContain("function downloadMime");
    expect(fileTree).not.toContain("absoluteDownloadUrl");
  });

  test("dragstart keeps the app-internal tree-move + editor-open payloads only", () => {
    expect(fileTree).toMatch(/e\.dataTransfer\.effectAllowed = "move"/);
    expect(fileTree).toMatch(/e\.dataTransfer\.setData\(TREE_MOVE_MIME, payload\)/);
    expect(fileTree).toMatch(/e\.dataTransfer\.setData\(FILE_DRAG_MIME/);
    expect(fileTree).not.toContain("setDownloadDragData(e, path, isDir)");
  });

  test("the native Tauri drag-out command + its desktop import are gone", () => {
    expect(fileTree).not.toContain("start_file_browser_drag_out");
    expect(fileTree).not.toContain("startNativeDragOut");
    expect(fileTree).not.toMatch(/from "\.\.\/api\/desktop"/);
  });

  test("docked selection context menu uses Upload and Download transfer rows", () => {
    expect(fileTree).toMatch(/const docked = \$derived\(dockSide !== undefined\)/);
    expect(fileTree).toMatch(/function downloadSelection\(path: string, isDir: boolean\): void/);
    expect(fileTree).toMatch(/link\.href = api\.downloadUrl\(path\)/);
    expect(fileTree).toMatch(
      /\{#if docked\}[\s\S]{1,500}<span>Open in File Browser<\/span>[\s\S]{1,300}<div class="ctx-sep" role="separator"><\/div>[\s\S]{1,500}<span>Upload<\/span>[\s\S]{1,500}<span>Download<\/span>[\s\S]{1,300}\{\/if\}[\s\S]{1,120}<div class="ctx-sep" role="separator"><\/div>/,
    );
  });

  test("shared inspectors expose Upload and Download transfer actions", () => {
    // I1/I2: the Upload + Download buttons live in the shared
    // actionsSection (multiline markup); Download gained a
    // disabled={downloadBusy} attr for the desktop progress path.
    expect(fileInfo).toMatch(/onclick=\{triggerUpload\}/);
    expect(fileInfo).toMatch(/onclick=\{downloadSelection\}/);
    expect(fileInfo).toMatch(/disabled=\{downloadBusy\}/);
    expect(fileInfo).toMatch(/fileOps\.replaceFileAt\(entry\.path, files\[0\]!\)/);
    expect(fileInfo).toMatch(/fileOps\.uploadFilesTo\(entry\.path, files\)/);
    // I3: the divergent DirectoryInfoBody was retired; the folder
    // inspector is now FileInfoBody's dir branch, which uploads to the
    // directory + downloads it as a tar through the same handlers.
    expect(fileInfo).toMatch(/fileOps\.uploadFilesTo\(entry\.path, files\)/);
  });

  test("file replacement uses upload replace mode and refreshes open tabs", () => {
    expect(client).toMatch(/replaceFile: \(/);
    expect(client).toMatch(/form\.append\("path", path\)/);
    expect(fileTree).toMatch(/fileOps\.replaceFileAt\(target\.path, input\.files\[0\]!\)/);
    expect(fileTree).toMatch(/fileOps\.uploadFilesTo\(target\.path, input\.files\)/);
    expect(store).toMatch(/replaceFileAt\(targetPath: string, picked: File\)/);
    expect(store).toMatch(/api\.replaceFile\(picked, targetPath/);
    expect(store).toMatch(/tabsForPath\(targetPath\)/);
    expect(store).toMatch(/refreshTabFromDisk\(tab\.tabId\)/);
  });
});
