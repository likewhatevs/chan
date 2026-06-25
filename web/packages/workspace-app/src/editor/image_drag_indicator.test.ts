import { describe, expect, test } from "vitest";
import { rowSnippet } from "./image_drag_indicator";
import indicator from "./image_drag_indicator.ts?raw";
import imageWidget from "./widgets/image.ts?raw";
import imageDrop from "./bubbles/image_drop.ts?raw";
import wysiwyg from "./Wysiwyg.svelte?raw";

describe("rowSnippet (badge text)", () => {
  test("trims and passes short lines through", () => {
    expect(rowSnippet("  hello world  ")).toBe("hello world");
  });

  test("truncates long lines with an ellipsis", () => {
    const long = "x".repeat(60);
    const out = rowSnippet(long);
    expect(out.endsWith("…")).toBe(true);
    expect(out.length).toBe(41); // 40 chars + ellipsis
  });

  test("an empty line reads as a placeholder, not blank", () => {
    expect(rowSnippet("   ")).toBe("(empty line)");
    expect(rowSnippet("")).toBe("(empty line)");
  });
});

describe("image-drag indicator wiring", () => {
  test("the drop target tracks the pointer and flags the no-op own row", () => {
    expect(indicator).toMatch(/noop: line\.from === srcLine\.from/);
    expect(indicator).toContain('class: "cm-md-image-drop-line"');
    expect(indicator).toMatch(/line \$\{target\.lineNo\} · \$\{rowSnippet\(target\.text\)\}/);
  });

  test("dragstart arms, dragover refreshes, drop/dragend/leave clear", () => {
    // widget dragstart records the source range; dragend clears.
    expect(imageWidget).toMatch(/startImageDragIndicator\(view, range\)/);
    expect(imageWidget).toMatch(/clearImageDragIndicator\(view\)/);
    // editor dragover refreshes; drop clears; dragleave hides (keeps src).
    expect(imageDrop).toMatch(
      /updateImageDropTarget\(view, event\.clientX, event\.clientY\)/,
    );
    expect(imageDrop).toMatch(/clearImageDragIndicator\(view\)/);
    expect(imageDrop).toMatch(/hideImageDropTarget\(view\)/);
  });

  test("the indicator extension is wired into the write-side bundle", () => {
    expect(wysiwyg).toContain("imageDragIndicator");
    expect(wysiwyg).toContain(".cm-md-image-drop-badge");
  });
});
