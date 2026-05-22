import { describe, expect, test } from "vitest";
import dirInfo from "./DirectoryInfoBody.svelte?raw";

// `fullstack-a-66` slice c: when the synthetic Drafts row is
// selected in the FB, the inspector renders a notice
// explaining that Drafts lives in chan-drive's metadata
// folder (NOT under the drive root), plus a Drafts-tinted
// kind chip so the inspector header reads as "different
// category" at a glance.

describe("fullstack-a-66 slice c: Drafts inspector notice", () => {
  test("kind chip swaps to DRAFTS label when path === 'Drafts'", () => {
    expect(dirInfo).toMatch(
      /class:drafts=\{path === "Drafts"\}/,
    );
    expect(dirInfo).toMatch(
      /path === "Drafts" \? "DRAFTS" : "DIR"/,
    );
  });

  test("Drafts kind chip picks up --fb-drafts-fg tint", () => {
    expect(dirInfo).toMatch(
      /\.kind-chip\.drafts \{[\s\S]*?background: var\(--fb-drafts-fg\);/,
    );
  });

  test("notice block renders for Drafts directory", () => {
    expect(dirInfo).toMatch(
      /\{#if path === "Drafts"\}[\s\S]*?class="drafts-notice"/,
    );
  });

  test("notice text explains 'outside the drive's root'", () => {
    expect(dirInfo).toMatch(
      /<strong>Drafts lives outside the drive's root\.<\/strong>/,
    );
  });

  test("notice cross-references Cmd+N + Rich Prompt history paths", () => {
    expect(dirInfo).toMatch(/Drafts\/untitled-N/);
    expect(dirInfo).toMatch(/Drafts\/rich-prompt-N/);
  });

  test("CSS styles the notice with the Drafts tint", () => {
    expect(dirInfo).toMatch(
      /\.drafts-notice \{[\s\S]*?background: var\(--fb-drafts-bg\);[\s\S]*?border-left: 3px solid var\(--fb-drafts-fg\);/,
    );
  });

  test("rationale comment cites the chan-drive metadata routing", () => {
    expect(dirInfo).toMatch(/chan-drive's/i);
    expect(dirInfo).toMatch(/metadata folder \(drafts_dir handle\)/i);
  });
});
