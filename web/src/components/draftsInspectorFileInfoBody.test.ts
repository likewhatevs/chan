import { describe, expect, test } from "vitest";
import fileInfo from "./FileInfoBody.svelte?raw";

// FileInfoBody's Drafts copy stays aligned with DirectoryInfoBody. The
// drafts dir is a real in-workspace directory now, so the inspector
// gets a normal entry; the DRAFTS chip + notice are keyed off the
// shared `isDraftPath` / `draftsDir()` helpers, not a literal.

describe("FileInfoBody Drafts header", () => {
  test("dir branch renders DRAFTS chip when the entry is a draft path", () => {
    expect(fileInfo).toMatch(
      /\{#if isDraftPath\(entry\.path\)\}[\s\S]*?<span class="kind-chip drafts-chip">DRAFTS<\/span>/,
    );
  });

  test("non-Drafts dir branch still uses KindChip kind='folder'", () => {
    expect(fileInfo).toMatch(
      /\{:else\}[\s\S]*?<KindChip kind="folder" block onClick=\{onSetAsScope\} \/>/,
    );
  });

  test("Drafts notice renders for a draft-path entry", () => {
    expect(fileInfo).toMatch(
      /\{#if isDraftPath\(entry\.path\)\}[\s\S]*?<div class="drafts-notice"[\s\S]*?<strong>Drafts are uncommitted scratch space\.<\/strong>/,
    );
  });

  test("synthetic Drafts entries are gone; no `entry.path === \"Drafts\"` literals", () => {
    expect(fileInfo).not.toMatch(/entry\.path === "Drafts"/);
    expect(fileInfo).not.toMatch(/path\.startsWith\("Drafts\/"\)/);
  });

  test("notice references the Cmd+N draftsDir() path pattern, no Team Work copy", () => {
    expect(fileInfo).toMatch(/\{draftsDir\(\)\}\/untitled-N/);
    expect(fileInfo).not.toMatch(/team-work-N/);
    expect(fileInfo).not.toMatch(/Team Work/);
  });

  test("CSS rules for .kind-chip.drafts-chip + .drafts-notice present", () => {
    expect(fileInfo).toMatch(
      /\.kind-chip\.drafts-chip \{[\s\S]*?background: var\(--fb-drafts-fg\);/,
    );
    expect(fileInfo).toMatch(
      /\.drafts-notice \{[\s\S]*?background: var\(--fb-drafts-bg\);[\s\S]*?border-left: 3px solid var\(--fb-drafts-fg\);/,
    );
  });

  test("FileInfoBody imports the shared draft helpers from the store", () => {
    expect(fileInfo).toMatch(/draftsDir,/);
    expect(fileInfo).toMatch(/isDraftPath,/);
  });
});
