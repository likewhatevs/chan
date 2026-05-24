import { describe, expect, test } from "vitest";
import client from "../api/client.ts?raw";
import fileTree from "./FileTree.svelte?raw";

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

  test("selection context menu uses the same download URL helper", () => {
    expect(fileTree).toMatch(/function downloadSelection\(path: string, isDir: boolean\): void/);
    expect(fileTree).toMatch(/link\.href = api\.downloadUrl\(path\)/);
    expect(fileTree).toMatch(
      /onclick=\{\(\) => downloadSelection\(menu!\.path, menu!\.isDir\)\}[\s\S]{1,160}<span>Download<\/span>/,
    );
  });
});
