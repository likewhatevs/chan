import { describe, expect, test } from "vitest";
import fileInfo from "./FileInfoBody.svelte?raw";

// FileInfoBody's Drafts copy stays aligned with DirectoryInfoBody in
// case a caller passes the metadata-backed Drafts root through the
// file inspector path.

describe("FileInfoBody Drafts header", () => {
  test("dir branch renders DRAFTS chip when entry.path === 'Drafts'", () => {
    expect(fileInfo).toMatch(
      /\{#if entry\.path === "Drafts"\}[\s\S]*?<span class="kind-chip drafts-chip">DRAFTS<\/span>/,
    );
  });

  test("non-Drafts dir branch still uses KindChip kind='folder'", () => {
    expect(fileInfo).toMatch(
      /\{:else\}[\s\S]*?<KindChip kind="folder" block onClick=\{onSetAsScope\} \/>/,
    );
  });

  test("'outside workspace's root' notice renders for Drafts entry", () => {
    expect(fileInfo).toMatch(
      /\{#if entry\.path === "Drafts"\}[\s\S]*?<div class="drafts-notice"[\s\S]*?<strong>Drafts lives outside the workspace's root\.<\/strong>/,
    );
  });

  test("notice references the Cmd+N draft path pattern", () => {
    // The notice references the Cmd+N draft path and carries no
    // Team Work / team-work-workspace archival copy.
    expect(fileInfo).toMatch(/Drafts\/untitled-N/);
    expect(fileInfo).not.toMatch(/Drafts\/team-work-N/);
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

  test("rationale comment explains why FileInfoBody keeps Drafts copy", () => {
    expect(fileInfo).toMatch(
      /file-inspector Drafts copy aligned with the[\s\S]*?graph[\s\S]*?directory inspector/i,
    );
  });
});
