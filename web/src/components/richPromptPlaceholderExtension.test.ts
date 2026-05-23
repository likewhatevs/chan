import { describe, expect, test } from "vitest";
import richPrompt from "./TerminalRichPrompt.svelte?raw";
import wysiwyg from "../editor/Wysiwyg.svelte?raw";
import source from "../editor/Source.svelte?raw";

// `fullstack-a-89` architectural fix: replace the `-a-24` CSS
// overlay placeholder with CM6's built-in `placeholder`
// extension. Cursor + placeholder share the exact same
// coordinate system because CM6 renders the placeholder
// INSIDE the first line at the cursor position. Supersedes
// `-a-84` (X-offset CSS hack) + `-a-87` (line-height CSS
// match) — both fought CM6's internal positioning instead of
// using it.

describe("fullstack-a-89: Wysiwyg.svelte threads placeholderText", () => {
  test("imports `placeholder` from @codemirror/view", () => {
    expect(wysiwyg).toMatch(
      /import \{[\s\S]*?placeholder[\s\S]*?\} from "@codemirror\/view";/,
    );
  });

  test("accepts optional `placeholderText` prop", () => {
    expect(wysiwyg).toMatch(/placeholderText\?: string;/);
  });

  test("adds placeholder() extension when the prop is set", () => {
    expect(wysiwyg).toMatch(
      /\.\.\.\(placeholderText \? \[placeholder\(placeholderText\)\] : \[\]\),/,
    );
  });
});

describe("fullstack-a-89: Source.svelte threads placeholderText", () => {
  test("imports `placeholder` from @codemirror/view", () => {
    expect(source).toMatch(
      /import \{[\s\S]*?placeholder[\s\S]*?\} from "@codemirror\/view";/,
    );
  });

  test("accepts optional `placeholderText` prop", () => {
    expect(source).toMatch(/placeholderText\?: string;/);
  });

  test("adds placeholder() extension when the prop is set", () => {
    expect(source).toMatch(
      /\.\.\.\(placeholderText \? \[placeholder\(placeholderText\)\] : \[\]\),/,
    );
  });
});

describe("fullstack-a-89: TerminalRichPrompt wires the new prop + drops the overlay", () => {
  test("PROMPT_PLACEHOLDER_TEXT constant declared with leading space (per -a-89b)", () => {
    // `fullstack-a-89b`: leading space added to satisfy
    // @@Alex's literal spec `{cursor}{space}{default-text}`.
    expect(richPrompt).toMatch(
      /const PROMPT_PLACEHOLDER_TEXT = " Write a multi-line command and Cmd\+Enter";/,
    );
  });

  test("Wysiwyg + Source both receive placeholderText", () => {
    expect(richPrompt).toMatch(
      /<Wysiwyg[\s\S]*?placeholderText=\{PROMPT_PLACEHOLDER_TEXT\}/,
    );
    expect(richPrompt).toMatch(
      /<Source[\s\S]*?placeholderText=\{PROMPT_PLACEHOLDER_TEXT\}/,
    );
  });

  test("pre-`-a-89` CSS overlay markup removed", () => {
    expect(richPrompt).not.toMatch(/<div class="prompt-placeholder"/);
    expect(richPrompt).not.toMatch(
      /\{#if prompt\.buffer === ""\}[\s\S]*?prompt-placeholder/,
    );
  });

  test("pre-`-a-89` .prompt-placeholder CSS rule removed", () => {
    expect(richPrompt).not.toMatch(/\.prompt-placeholder \{[\s\S]*?position: absolute;/);
  });

  test("rationale comment cites the architecture swap + the superseded tasks", () => {
    expect(richPrompt).toMatch(
      /`fullstack-a-89`:[\s\S]*?placeholder moved from[\s\S]*?CSS overlay/i,
    );
    expect(richPrompt).toMatch(/`-a-84`[\s\S]*?`-a-87`/);
  });
});
