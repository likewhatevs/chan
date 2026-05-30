import { describe, expect, test } from "vitest";
import tree from "./FileTree.svelte?raw";

// FileTree in-tree selection menu: "From selection" header, New Graph
// entry, label updates (Search, New Terminal), separator between
// workflow and per-row ops, and unified "New File or Directory" dialog.

describe("FileTree selection menu header + new entries", () => {
  test("From-selection label rendered at the top of the ctx menu", () => {
    expect(tree).toMatch(
      /\{#if menu\}[\s\S]{1,2000}<div class="from-selection-label">From selection<\/div>/,
    );
  });

  test("Search label relabelled (was \"Search this\")", () => {
    expect(tree).toMatch(/<span>Search<\/span>/);
    expect(tree).not.toMatch(/<span>Search this<\/span>/);
  });

  test("New Terminal label relabelled (was \"Terminal from here\")", () => {
    expect(tree).toMatch(/<span>New Terminal<\/span>/);
    expect(tree).not.toMatch(/<span>Terminal from here<\/span>/);
  });

  test("Unified \"New File or Directory\" entry replaces the separate New File / New Directory rows", () => {
    // One entry; the modal detects file-vs-dir from the trailing slash.
    expect(tree).toMatch(/<span>New File or Directory<\/span>/);
    expect(tree).not.toMatch(/<span>New File<\/span>/);
    expect(tree).not.toMatch(/<span>New Directory<\/span>/);
  });

  test("New Graph entry added, routes to graphThis", () => {
    expect(tree).toMatch(
      /onclick=\{\(\) => graphThis\(menu!\.path, menu!\.isDir\)\}[\s\S]{1,400}<span>New Graph<\/span>/,
    );
  });

  test("docked transfer rows sit between separators", () => {
    expect(tree).toMatch(
      /<span>New Graph<\/span>[\s\S]{1,500}\{#if docked\}[\s\S]{1,500}<span>Open in File Browser<\/span>[\s\S]{1,300}<div class="ctx-sep" role="separator"><\/div>[\s\S]{1,500}<span>Upload<\/span>[\s\S]{1,500}<span>Download<\/span>[\s\S]{1,300}\{\/if\}[\s\S]{1,120}<div class="ctx-sep" role="separator"><\/div>[\s\S]{1,400}<span>Copy Path<\/span>/,
    );
  });

  test("Open in File Browser spawns a selected tab with inspector open", () => {
    expect(tree).toMatch(/function openSelectionInFileBrowser\(path: string\): void/);
    expect(tree).toMatch(/const tab = openBrowserInActivePane\(\{ select: path \}\)/);
    expect(tree).toMatch(/tab\.inspectorOpen = true/);
    expect(tree).toMatch(/tab\.expanded = ancestors\.length > 0 \? ancestors : undefined/);
  });
});

describe("transfer rows gated, per-row ops kept", () => {
  test("Open in File Browser / Upload / Download are docked-only and row ops stay available", () => {
    expect(tree).toMatch(/\{#if docked\}[\s\S]{1,1000}<span>Open in File Browser<\/span>/);
    expect(tree).toMatch(/\{#if docked\}[\s\S]{1,1000}<span>Upload<\/span>/);
    expect(tree).toMatch(/\{#if docked\}[\s\S]{1,1000}<span>Download<\/span>/);
    expect(tree).toMatch(/<span>Copy Path<\/span>/);
    expect(tree).toMatch(/<span>Rename \/ Move<\/span>/);
    expect(tree).toMatch(/<span>Delete<\/span>/);
  });
});
