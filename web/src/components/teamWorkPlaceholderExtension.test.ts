import { describe, expect, test } from "vitest";
import teamWork from "./TeamWork.svelte?raw";
import wysiwyg from "../editor/Wysiwyg.svelte?raw";
import source from "../editor/Source.svelte?raw";

// The prompt placeholder uses CM6's built-in `placeholder`
// extension, not a CSS overlay. Cursor + placeholder share the exact
// same coordinate system because CM6 renders the placeholder INSIDE
// the first line at the cursor position; a CSS overlay would fight
// CM6's internal positioning instead of using it.

describe("Wysiwyg.svelte threads placeholderText", () => {
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

describe("Source.svelte threads placeholderText", () => {
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

describe("TeamWork wires placeholderText and has no overlay", () => {
  test("PROMPT_PLACEHOLDER_TEXT constant declared with leading space", () => {
    // Leading space satisfies `{cursor}{space}{default-text}`. The
    // copy advertises the Enter / Shift+Enter chord split.
    expect(teamWork).toMatch(
      /const PROMPT_PLACEHOLDER_TEXT = " Write your prompt; Enter to send, Shift\+Enter for a new line";/,
    );
  });

  test("Wysiwyg + Source both receive placeholderText", () => {
    expect(teamWork).toMatch(
      /<Wysiwyg[\s\S]*?placeholderText=\{PROMPT_PLACEHOLDER_TEXT\}/,
    );
    expect(teamWork).toMatch(
      /<Source[\s\S]*?placeholderText=\{PROMPT_PLACEHOLDER_TEXT\}/,
    );
  });

  test("no CSS overlay placeholder markup", () => {
    expect(teamWork).not.toMatch(/<div class="prompt-placeholder"/);
    expect(teamWork).not.toMatch(
      /\{#if prompt\.buffer === ""\}[\s\S]*?prompt-placeholder/,
    );
  });

  test("no .prompt-placeholder CSS rule", () => {
    expect(teamWork).not.toMatch(/\.prompt-placeholder \{[\s\S]*?position: absolute;/);
  });

  test("rationale comment cites the CM6 placeholder extension", () => {
    expect(teamWork).toMatch(
      /CM6's built-in `placeholder` extension[\s\S]*?placeholderText/i,
    );
  });
});
