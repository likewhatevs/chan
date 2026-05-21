import { describe, expect, test } from "vitest";
import source from "./DirectoryInfoBody.svelte?raw";
import inspector from "./InspectorBody.svelte?raw";
import graphPanel from "./GraphPanel.svelte?raw";
import apiClient from "../api/client.ts?raw";

// `fullstack-a-50` (G3): directory nodes in the graph route to a
// new `DirectoryInfoBody` inspector via the existing dispatcher.
// Three load-bearing pieces:
//
// 1. `api.reportDir(path)` wired against /api/report/dir (the
//    O(1) cache from systacean-15).
// 2. `InspectorSelection` extended with `{ kind: "directory" }`;
//    `InspectorBody` dispatches it to `DirectoryInfoBody`.
// 3. `GraphPanel` maps `folder`-kind selected nodes to the new
//    directory selection + wires `onSetAsScope` to
//    `rescopeFromHere("dir:<path>")`.
//
// Tests pin the wiring shape so a future refactor can't silently
// drop the inspector or the re-rooting affordance.

describe("fullstack-a-50: DirectoryInfoBody wiring", () => {
  test("api.reportDir calls /api/report/dir", () => {
    expect(apiClient).toMatch(
      /reportDir: \(path: string\) =>[\s\S]*?\/api\/report\/dir\?path=\$\{encodeURIComponent\(path\)\}/,
    );
    // Same response shape as reportPrefix (ReportPrefix).
    expect(apiClient).toMatch(/reportDir[\s\S]*?req<ReportPrefix>/);
  });

  test("InspectorSelection adds the directory variant", () => {
    expect(inspector).toMatch(
      /\| \{ kind: "directory"; path: string; label\?\: string \}/,
    );
  });

  test("InspectorBody dispatches `directory` selection to DirectoryInfoBody", () => {
    expect(inspector).toMatch(
      /\{:else if selection\.kind === "directory"\}[\s\S]*?<DirectoryInfoBody/,
    );
    expect(inspector).toMatch(/import DirectoryInfoBody from "\.\/DirectoryInfoBody\.svelte"/);
  });

  test("DirectoryInfoBody fetches via api.reportDir on path change", () => {
    expect(source).toMatch(/import \{ api \} from "\.\.\/api\/client"/);
    expect(source).toMatch(/report = await api\.reportDir\(p\)/);
    expect(source).toMatch(/\$effect\(\(\) => \{\s*void load\(path\)/);
  });

  test("DirectoryInfoBody renders totals + by-language table + COCOMO sections", () => {
    expect(source).toMatch(/<h4>Totals<\/h4>/);
    expect(source).toMatch(/<h4>By language<\/h4>/);
    expect(source).toMatch(/<h4>COCOMO/);
    // Totals fields.
    expect(source).toMatch(/report\.totals\.files/);
    expect(source).toMatch(/report\.totals\.code/);
    // By-language iteration.
    expect(source).toMatch(/\{#each report\.by_language as lang/);
    // COCOMO summary fields.
    expect(source).toMatch(/report\.cocomo\.effort_person_months/);
    expect(source).toMatch(/report\.cocomo\.schedule_months/);
    expect(source).toMatch(/report\.cocomo\.estimated_cost_usd/);
  });

  test("DirectoryInfoBody renders the Graph-from-here action when onSetAsScope is wired", () => {
    expect(source).toMatch(
      /\{#if onSetAsScope\}[\s\S]*?<button class="set-as-scope" onclick=\{onSetAsScope\}/,
    );
    expect(source).toMatch(/Graph from here/);
  });

  test("DirectoryInfoBody renders the no-stats branch when /api/report/dir 404s", () => {
    // Empty directory or chan-reports indexing not run → 404. The
    // body falls through to a "no chan-report data" affordance
    // mentioning the chan-reports toggle (-a-48).
    expect(source).toMatch(/if \(\/404\/\.test\(message\)/);
    expect(source).toMatch(/No chan-report data for this directory/);
    expect(source).toMatch(/chan-reports[\s\S]*?Hybrid[\s\S]*?Browser/);
  });

  test("DirectoryInfoBody handles drive-root display name fallback", () => {
    // path = "" → "Drive root" header.
    expect(source).toMatch(/path === "" \? "Drive root" : path/);
  });

  test("GraphPanel maps `folder` selected nodes to the directory selection", () => {
    expect(graphPanel).toMatch(
      /selectedNode\.kind === "folder"[\s\S]*?kind: "directory",[\s\S]*?path: selectedNode\.path/,
    );
  });

  test("GraphPanel wires onSetAsScope to rescopeFromHere('dir:<path>') for directories", () => {
    expect(graphPanel).toMatch(
      /onSetAsScope=\{[\s\S]*?inspectorSelection\?\.kind === "directory"[\s\S]*?rescopeFromHere\(`dir:\$\{inspectorSelection\.path\}`\)/,
    );
  });
});
