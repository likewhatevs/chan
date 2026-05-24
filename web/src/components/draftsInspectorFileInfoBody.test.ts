import { describe, expect, test } from "vitest";
import fileInfo from "./FileInfoBody.svelte?raw";

// `fullstack-a-66` slice c follow-up: keep FileInfoBody's
// Drafts copy aligned with DirectoryInfoBody in case a caller
// passes the metadata-backed Drafts root through the file
// inspector path.

describe("fullstack-a-66 slice c follow-up: FileInfoBody Drafts header", () => {
  test("dir branch renders DRAFTS chip when entry.path === 'Drafts'", () => {
    expect(fileInfo).toMatch(
      /\{#if entry\.path === "Drafts"\}[\s\S]*?<span class="kind-chip drafts-chip">DRAFTS<\/span>/,
    );
  });

  test("non-Drafts dir branch still uses KindChip kind='folder'", () => {
    expect(fileInfo).toMatch(
      /\{:else\}[\s\S]*?<KindChip kind="folder" block \/>/,
    );
  });

  test("'outside drive's root' notice renders for Drafts entry", () => {
    expect(fileInfo).toMatch(
      /\{#if entry\.path === "Drafts"\}[\s\S]*?<div class="drafts-notice"[\s\S]*?<strong>Drafts lives outside the drive's root\.<\/strong>/,
    );
  });

  test("notice cross-references Cmd+N + Rich Prompt path patterns", () => {
    expect(fileInfo).toMatch(/Drafts\/untitled-N/);
    expect(fileInfo).toMatch(/Drafts\/rich-prompt-N/);
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
      /file-inspector Drafts copy aligned with the graph[\s\S]*?directory inspector/i,
    );
  });
});
