import { describe, expect, test } from "vitest";
import client from "../api/client.ts?raw";
import fileTree from "./FileTree.svelte?raw";
import fileInfo from "./FileInfoBody.svelte?raw";
import dirInfo from "./DirectoryInfoBody.svelte?raw";
import store from "../state/store.svelte.ts?raw";

describe("FileTree browser drag-out", () => {
  test("api exposes token-bearing download URLs", () => {
    expect(client).toMatch(/downloadUrl: \(path: string\) =>/);
    expect(client).toMatch(
      /withTokenQuery\(`\/api\/files\/\$\{encPath\(path\)\}\?download=1`\)/,
    );
  });

  test("file and directory drags carry DownloadURL and uri-list payloads", () => {
    expect(fileTree).toMatch(/e\.dataTransfer\.setData\(\s*"DownloadURL"/);
    expect(fileTree).toMatch(/api\.downloadUrl\(path\)/);
    expect(fileTree).toMatch(/e\.dataTransfer\.setData\("text\/uri-list", url\)/);
  });

  test("directory drags download archives while keeping tree move payload", () => {
    expect(fileTree).toMatch(/if \(isDir\) return "application\/x-tar"/);
    expect(fileTree).toMatch(/e\.dataTransfer\.effectAllowed = "copyMove"/);
    expect(fileTree).toMatch(/e\.dataTransfer\.setData\(TREE_MOVE_MIME, payload\)/);
    expect(fileTree).toMatch(/setDownloadDragData\(e, path, isDir\)/);
  });

  test("Tauri desktop drag-out uses the token-bearing download route", () => {
    expect(fileTree).toMatch(
      /import \{ isTauriDesktop, tauriInvoke \} from "\.\.\/api\/desktop"/,
    );
    expect(fileTree).toMatch(/new URL\(api\.downloadUrl\(path\), window\.location\.href\)/);
    expect(fileTree).toMatch(/tauriInvoke\("start_file_browser_drag_out"/);
    expect(fileTree).toMatch(/downloadUrl: absoluteDownloadUrl\(path\)/);
    expect(fileTree).toMatch(/filename: downloadFilename\(path, isDir\)/);
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
    expect(fileInfo).toMatch(/<button class="open" type="button" onclick=\{triggerUpload\}/);
    expect(fileInfo).toMatch(/<button class="open" type="button" onclick=\{downloadSelection\}/);
    expect(fileInfo).toMatch(/fileOps\.replaceFileAt\(entry\.path, files\[0\]!\)/);
    expect(fileInfo).toMatch(/fileOps\.uploadFilesTo\(entry\.path, files\)/);
    expect(dirInfo).toMatch(/onclick=\{triggerUpload\}/);
    expect(dirInfo).toMatch(/onclick=\{downloadDirectory\}/);
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
