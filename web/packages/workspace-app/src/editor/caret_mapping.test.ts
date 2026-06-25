import { describe, expect, test } from "vitest";
import {
  renderedCaretForSourceCaret,
  sourceCaretForRenderedCaret,
} from "./caret_mapping";

const DOC = "before\n![alt](images/photo.png#w=250)\nafter";
const IMAGE_FROM = DOC.indexOf("!");
const URL_FROM = DOC.indexOf("images/");
const IMAGE_TO = DOC.indexOf("\nafter");

describe("source/rendered caret mapping", () => {
  test("source caret inside image syntax selects the rendered image boundary", () => {
    expect(renderedCaretForSourceCaret(DOC, { from: URL_FROM + 3, to: URL_FROM + 3 })).toEqual({
      from: IMAGE_FROM,
      to: IMAGE_FROM,
    });
  });

  test("rendered image boundary maps back inside the image URL syntax", () => {
    const mapped = sourceCaretForRenderedCaret(DOC, { from: IMAGE_FROM, to: IMAGE_FROM });

    expect(mapped.from).toBeGreaterThan(URL_FROM - 1);
    expect(mapped.from).toBeLessThan(IMAGE_TO);
    expect(DOC.slice(mapped.from, mapped.from + 6)).toBe("images");
  });

  test("non-image carets are unchanged", () => {
    expect(renderedCaretForSourceCaret(DOC, { from: 2, to: 2 })).toEqual({ from: 2, to: 2 });
    expect(sourceCaretForRenderedCaret(DOC, { from: 2, to: 2 })).toEqual({ from: 2, to: 2 });
  });
});
