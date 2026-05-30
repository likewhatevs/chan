import { describe, expect, test } from "vitest";
import fileInfo from "./FileInfoBody.svelte?raw";

// The inspector renders one consistent layout on every surface:
//   header -> actions section -> lazy content (report / refs).
// The actions (Open / View+Zoom / Upload / Download / Show / Graph from
// here) sit directly under the filename, plus a full-path toggle. These
// source pins lock that ordering so the layout can't silently drift.

describe("shared actions section under the filename", () => {
  test("defines a reusable actionsSection snippet", () => {
    expect(fileInfo).toMatch(/\{#snippet actionsSection\(\)\}/);
    expect(fileInfo).toMatch(/<div class="actions-section">/);
  });

  test("actions section carries the full-path toggle + revealed path row", () => {
    expect(fileInfo).toMatch(
      /class="path-toggle"[\s\S]*?onclick=\{\(\) => \(showFullPath = !showFullPath\)\}/,
    );
    expect(fileInfo).toMatch(
      /\{#if showFullPath\}[\s\S]*?<div class="path-row mono"/,
    );
    // The toggle state resets when the selection changes.
    expect(fileInfo).toMatch(/showFullPath = false;/);
  });

  test("actions section gates Open on isEditableText + onOpen", () => {
    // Open lives inside actionsSection now (not at the body bottom),
    // and only for editable files with an onOpen handler bound.
    expect(fileInfo).toMatch(
      /\{#snippet actionsSection\(\)\}[\s\S]*?\{@const editable = !isDir && isEditableText\(entry\.path\)\}[\s\S]*?\{#if !isDir && onOpen\}[\s\S]*?\{#if editable\}[\s\S]*?onclick=\{onOpen\}>Open/,
    );
  });

  test("media gets View / Zoom (image) or View PDF in the actions section", () => {
    expect(fileInfo).toMatch(
      /\{#snippet actionsSection\(\)\}[\s\S]*?\{#if image\}[\s\S]*?onclick=\{\(\) => openImageZoom\(entry\.path\)\}>View \/ Zoom/,
    );
    expect(fileInfo).toMatch(
      /\{:else if pdf\}[\s\S]*?onclick=\{\(\) => openPdfViewer\(entry\.path\)\}>View PDF/,
    );
  });

  test("Show File/Directory + Graph from here are host-gated in the section", () => {
    expect(fileInfo).toMatch(
      /\{#if onReveal\}[\s\S]*?\{isDir \? "Show Directory" : "Show File"\}/,
    );
    expect(fileInfo).toMatch(
      /\{#if onSetAsScope\}[\s\S]*?onclick=\{onSetAsScope\}[\s\S]*?Graph from here/,
    );
  });

  test("Export to PDF shows for markdown files + routes through the print helper", () => {
    // Gated on markdown files; the selection isn't necessarily open in an
    // editor, so the handler fetches the file content and prints it.
    expect(fileInfo).toMatch(/\{@const markdown = !isDir && isMarkdown\(entry\.path\)\}/);
    expect(fileInfo).toMatch(/\{#if markdown\}[\s\S]*?onclick=\{doExportPdf\}[\s\S]*?Export to PDF/);
    expect(fileInfo).toMatch(
      /async function doExportPdf\(\): Promise<void> \{[\s\S]*?printMarkdownDocument\(\{[\s\S]*?markdown: file\.content/,
    );
  });

  test("dir branch renders actions BEFORE the dir stats meta-grid", () => {
    // The dir branch order is: ... badges -> {@render actionsSection()}
    // -> {#if dirStats} meta-grid. The actions must precede the stats.
    const actionsIdx = fileInfo.indexOf("{@render actionsSection()}");
    const dirStatsIdx = fileInfo.indexOf("{#if dirStats}");
    expect(actionsIdx).toBeGreaterThan(0);
    expect(dirStatsIdx).toBeGreaterThan(actionsIdx);
  });

  test("file branch renders actions BEFORE the size/modified meta-grid", () => {
    // The file branch renders the (optional) image preview, then
    // {@render actionsSection()}, then the size/modified meta-grid.
    // (lastIndexOf gives the file-branch render; the dir-branch render
    // is the earlier occurrence. Search for the file-branch size span
    // AFTER that render so we don't match the dir-branch stats grid.)
    const lastActions = fileInfo.lastIndexOf("{@render actionsSection()}");
    const sizeGrid = fileInfo.indexOf(
      '<span class="k">size</span>',
      lastActions,
    );
    expect(lastActions).toBeGreaterThan(0);
    expect(sizeGrid).toBeGreaterThan(lastActions);
  });

  test("actions live only in the reusable section, not standalone bottom blocks", () => {
    // The action buttons are defined once inside actionsSection and
    // rendered via {@render actionsSection()}; there is no separate
    // bottom-of-body action block to drift out of sync.
    const sectionDefs = fileInfo.match(/<div class="actions-section">/g) ?? [];
    expect(sectionDefs.length).toBe(1);
    expect(fileInfo).toMatch(/\{@render actionsSection\(\)\}/);
  });
});
