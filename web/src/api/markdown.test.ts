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

describe("markdown iframe embeds", () => {
  test("a YouTube image link renders a youtube-nocookie iframe", () => {
    const html = renderMarkdown("![](https://youtu.be/dQw4w9WgXcQ)");
    expect(html).toContain("<iframe");
    expect(html).toContain(
      "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ",
    );
    expect(html).toContain("sandbox");
  });

  test("a Google Maps image link renders a maps iframe", () => {
    const html = renderMarkdown(
      "![](https://www.google.com/maps/embed?pb=!1m18!1m12)",
    );
    expect(html).toContain("<iframe");
    expect(html).toContain("https://www.google.com/maps/embed?pb=");
  });

  test("a plain image stays an <img>, not an iframe", () => {
    const html = renderMarkdown("![cat](https://example.com/cat.png)");
    expect(html).toContain("<img");
    expect(html).not.toContain("<iframe");
  });

  test("a raw <iframe> on a non-allowlisted host is dropped", () => {
    const html = renderMarkdown('<iframe src="https://evil.com/x"></iframe>');
    expect(html).not.toContain("evil.com");
    expect(html).not.toContain("<iframe");
  });
});
