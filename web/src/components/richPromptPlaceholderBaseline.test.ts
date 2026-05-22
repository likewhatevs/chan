import { describe, expect, test } from "vitest";
import richPrompt from "./TerminalRichPrompt.svelte?raw";

// `fullstack-a-87` follow-up to `-a-84`: the X-offset fix
// exposed/introduced a Y-axis misalignment between the CM6
// cursor and the empty-state placeholder text. Root cause:
// CM6 cm-line uses line-height 1.8 (standard density;
// Wysiwyg.svelte:749) while the placeholder used line-height
// 1.5 — different block heights → different baseline
// positions even at the same `top`.

describe("fullstack-a-87: placeholder line-height matches cm-line", () => {
  test("placeholder line-height: 1.8 matches the standard-density cm-line", () => {
    expect(richPrompt).toMatch(
      /\.prompt-placeholder \{[\s\S]*?line-height: 1\.8;/,
    );
  });

  test("rationale comment cites the cm-line line-height + density drift", () => {
    expect(richPrompt).toMatch(/fullstack-a-87/);
    expect(richPrompt).toMatch(/match the CM6 cm-line line-height/i);
    expect(richPrompt).toMatch(/standard.*1\.8.*compact.*1\.65/i);
  });

  test("`-a-84` X-offset preserved (10px)", () => {
    expect(richPrompt).toMatch(/left: calc\(1rem \+ 10px\);/);
  });

  test("placeholder top still tracks --editor-top-pad", () => {
    expect(richPrompt).toMatch(
      /\.prompt-placeholder \{[\s\S]*?top: var\(--editor-top-pad, 16px\);/,
    );
  });
});
