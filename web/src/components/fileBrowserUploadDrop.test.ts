import { describe, expect, test } from "vitest";
import statusBar from "./AppStatusBar.svelte?raw";
import tree from "./FileTree.svelte?raw";
import client from "../api/client.ts?raw";
import store from "../state/store.svelte.ts?raw";

describe("File Browser upload via the Upload button", () => {
  // Bug 2 / round-1: external OS file drops (drag-IN) are no longer
  // accepted by the tree; importing files is now the Upload button's
  // job. The drop handlers only resolve the app-internal tree-move.
  test("FileTree no longer routes external file drops to upload", () => {
    expect(tree).not.toContain('types.includes("Files")');
    expect(tree).not.toContain("fileOps.uploadFilesTo(destDir, files)");
    expect(tree).not.toContain("hasExternalFiles");
    // The internal tree-move drop wiring stays on the rows.
    expect(tree).toContain("ondrop={(e) => onRowDrop(e, node.path)}");
    expect(tree).toContain("ondrop={(e) => onRowDrop(e, parentOf(node.path))}");
    // The Upload button still drives uploadFilesTo at the picked target.
    expect(tree).toContain("fileOps.uploadFilesTo(target.path");
  });

  test("store upload flow exposes progress, cancel, and tree refresh", () => {
    expect(store).toContain("fileTransferStatus");
    expect(store).toContain("AbortController");
    expect(store).toContain("api.uploadFile(file, destDir");
    expect(store).toContain("await refreshTreeForPath(result.path)");
    expect(store).toContain("upload failed: '${target}' already exists");
  });

  test("api client uses XHR so upload progress can drive status", () => {
    expect(client).toContain("new XMLHttpRequest()");
    expect(client).toContain("xhr.upload.onprogress");
    expect(client).toContain('/api/files/upload');
    expect(client).toContain("opts.signal?.addEventListener");
  });

  test("status bar renders upload progress with cancellation", () => {
    expect(statusBar).toContain("fileTransferStatus");
    expect(statusBar).toContain('aria-label="file transfer status"');
    expect(statusBar).toContain('aria-label="cancel upload"');
  });
});
