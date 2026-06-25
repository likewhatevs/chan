import { describe, expect, test } from "vitest";
import inspector from "./InspectorBody.svelte?raw";
import fileInfo from "./FileInfoBody.svelte?raw";
import graphPanel from "./GraphPanel.svelte?raw";
import apiClient from "../api/client.ts?raw";

// The graph folder inspector and the File Browser folder inspector must
// render the same body. Both surfaces route through FileInfoBody's is_dir
// branch; there is no separate DirectoryInfoBody. These pins lock the routing.

describe("folder inspector parity (graph == File Browser)", () => {
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
    // Prefer api.reportDir (the directory cache) and fall back to
    // api.reportPrefix on a 404 so all surfaces share the cheap path.
    expect(fileInfo).toMatch(
      /target\.is_dir[\s\S]*?api\.reportDir\(target\.path\)\.catch/,
    );
    expect(fileInfo).toMatch(
      /\/404\/\.test\(msg\)[\s\S]*?api\.reportPrefix\(target\.path\)/,
    );
  });

  test("api.reportDir calls the /api/report/dir cache endpoint", () => {
    expect(apiClient).toMatch(
      /reportDir: \(path: string\) =>[\s\S]*?\/api\/report\/dir\?path=\$\{encodeURIComponent\(path\)\}/,
    );
    expect(apiClient).toMatch(/reportDir[\s\S]*?req<ReportPrefix>/);
  });

  test("GraphPanel maps `folder` selected nodes to the directory selection", () => {
    // The folder node becomes a `directory` selection; InspectorBody
    // routes it to FileInfoBody's is_dir branch.
    expect(graphPanel).toMatch(
      /selectedNode\.kind === "folder"[\s\S]*?kind: "directory",[\s\S]*?path: selectedNode\.path/,
    );
  });
});
