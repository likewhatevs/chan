import { describe, expect, test } from "vitest";
import richPrompt from "./TerminalRichPrompt.svelte?raw";

// `fullstack-a-84`: shift the empty-prompt placeholder right
// so the CM6 cursor sits cleanly to its LEFT instead of
// overlapping the first character.

describe("fullstack-a-84: rich prompt placeholder offset", () => {
  test("placeholder `left` offsets past the CM6 cursor by ~10px", () => {
    expect(richPrompt).toMatch(
      /\.prompt-placeholder \{[\s\S]*?left: calc\(1rem \+ 10px\);/,
    );
  });

  test("rationale comment cites the offset + the cursor-left-of-placeholder semantic", () => {
    expect(richPrompt).toMatch(/fullstack-a-84/);
    expect(richPrompt).toMatch(/cursor sits cleanly to the LEFT/);
    expect(richPrompt).toMatch(/`\|W` overlap/);
  });

  test("placeholder is still conditionally rendered when buffer empty (no hide-on-focus)", () => {
    // Audit pin: confirm the Svelte conditional render that
    // shows the placeholder only when the buffer is empty
    // still exists. `-a-84` doesn't touch that — fix is
    // purely CSS-offset. The visible-on-focus semantic must
    // survive (NOT option A hide-on-focus per @@Alex).
    expect(richPrompt).toMatch(/\{#if prompt\.buffer === ""\}/);
  });

  test("placeholder retains its accessible class hook", () => {
    expect(richPrompt).toMatch(/class="prompt-placeholder"/);
  });
});
