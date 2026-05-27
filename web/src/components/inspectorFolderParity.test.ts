import { describe, expect, test } from "vitest";
import inspector from "./InspectorBody.svelte?raw";
import fileInfo from "./FileInfoBody.svelte?raw";
import graphPanel from "./GraphPanel.svelte?raw";
import apiClient from "../api/client.ts?raw";

// I3 (inspector consistency + layout, inspector-spec.md): the graph
// folder inspector must render the SAME body as the File Browser folder
// inspector. The drift was that `directory` selections routed to a
// separate `DirectoryInfoBody` while the File Browser used FileInfoBody.
// I3 retires DirectoryInfoBody and routes BOTH surfaces' folder
// selections through FileInfoBody's is_dir branch, so there is one
// folder inspector. These pins lock the unified routing.

describe("I3: folder inspector parity (graph == File Browser)", () => {
  test("InspectorBody routes `directory` selections to FileInfoBody", () => {
    expect(inspector).toMatch(
      /\{:else if selection\.kind === "directory"\}[\s\S]*?<FileInfoBody/,
    );
    // The dispatcher passes the graph node's label + the host actions.
    expect(inspector).toMatch(
      /\{:else if selection\.kind === "directory"\}[\s\S]*?label=\{selection\.label\}/,
    );
  });

  test("the divergent DirectoryInfoBody is no longer imported or rendered", () => {
    expect(inspector).not.toContain("import DirectoryInfoBody");
    expect(inspector).not.toContain("<DirectoryInfoBody");
  });

  test("FileInfoBody accepts the optional label prop for the folder header", () => {
    expect(fileInfo).toMatch(/label\?\: string;/);
    expect(fileInfo).toMatch(
      /\{label \|\| basename\(entry\.path\) \|\| workspace\.info\?\.label/,
    );
  });

  test("FileInfoBody dir report prefers the O(1) report/dir cache", () => {
    // Prefer api.reportDir (the cache the graph folder inspector used)
    // and fall back to api.reportPrefix on a 404 so the folder gets the
    // same cheap path on every surface.
    expect(fileInfo).toMatch(
      /target\.is_dir[\s\S]*?api\.reportDir\(target\.path\)\.catch/,
    );
    expect(fileInfo).toMatch(
      /\/404\/\.test\(msg\)[\s\S]*?api\.reportPrefix\(target\.path\)/,
    );
  });

  test("api.reportDir still calls the /api/report/dir cache endpoint", () => {
    // Preserved from the retired DirectoryInfoBody.test.ts: the cache
    // endpoint is now consumed by FileInfoBody's dir branch.
    expect(apiClient).toMatch(
      /reportDir: \(path: string\) =>[\s\S]*?\/api\/report\/dir\?path=\$\{encodeURIComponent\(path\)\}/,
    );
    expect(apiClient).toMatch(/reportDir[\s\S]*?req<ReportPrefix>/);
  });

  test("GraphPanel maps `folder` selected nodes to the directory selection", () => {
    // Preserved from the retired DirectoryInfoBody.test.ts. The folder
    // node still becomes a `directory` selection; it now lands on
    // FileInfoBody via the unified InspectorBody routing.
    expect(graphPanel).toMatch(
      /selectedNode\.kind === "folder"[\s\S]*?kind: "directory",[\s\S]*?path: selectedNode\.path/,
    );
  });
});
