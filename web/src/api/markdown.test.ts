// @vitest-environment jsdom

import { describe, expect, test } from "vitest";
import { renderMarkdown, renderMarkdownWithBreaks } from "./markdown";

describe("survey body line breaks", () => {
  test("renderMarkdown collapses a single newline (global breaks:false soft-break)", () => {
    const html = renderMarkdown("line one\nline two");
    expect(html).not.toContain("<br");
    expect(html).toContain("line one");
    expect(html).toContain("line two");
  });

  test("renderMarkdownWithBreaks renders a single newline as a <br>", () => {
    const html = renderMarkdownWithBreaks("line one\nline two");
    expect(html).toContain("<br");
    expect(html).toContain("line one");
    expect(html).toContain("line two");
  });

  test("renderMarkdownWithBreaks still separates paragraphs on a blank line", () => {
    const html = renderMarkdownWithBreaks("para one\n\npara two");
    // Two distinct paragraphs, not one with a <br>.
    expect(html.match(/<p>/g)?.length).toBe(2);
  });

  test("renderMarkdownWithBreaks is DOMPurify-sanitized", () => {
    const html = renderMarkdownWithBreaks("hi <script>alert(1)</script>");
    expect(html).not.toContain("<script");
  });
});
