// Pasted markdown source should land as markdown, not as escaped
// literals. Default turndown emits `\*bold\*` for a
// `<span>*bold*</span>` source; we override `td.escape` with
// identity so the asterisks survive the conversion and the
// Wysiwyg parser renders them as emphasis.
//
// The test exercises the converter directly (htmlToMarkdown is
// exported for this purpose); a behavioural test through the
// CM6 EditorView would need a DOM environment + the full
// markdown extension stack, which jsdom doesn't fully support.

import { describe, expect, test } from "vitest";
import { htmlToMarkdown } from "./paste_html";

describe("pasted markdown is NOT escaped", () => {
  test("asterisk emphasis survives the conversion", async () => {
    // Xcode-style copy of `*bold*` plain text lands on the
    // clipboard with a span wrapper. The escape override
    // leaves asterisks verbatim so the Wysiwyg parser
    // renders them as emphasis.
    const md = await htmlToMarkdown("<span>*bold*</span>");
    expect(md).toContain("*bold*");
    expect(md).not.toContain("\\*");
  });

  test("double-asterisk strong survives", async () => {
    const md = await htmlToMarkdown("<span>**strong**</span>");
    expect(md).toContain("**strong**");
    expect(md).not.toContain("\\*");
  });

  test("underscore emphasis survives", async () => {
    const md = await htmlToMarkdown("<span>_emphasis_</span>");
    expect(md).toContain("_emphasis_");
    expect(md).not.toContain("\\_");
  });

  test("markdown link survives", async () => {
    const md = await htmlToMarkdown("<span>[chan](https://chan.app)</span>");
    expect(md).toContain("[chan](https://chan.app)");
    expect(md).not.toContain("\\[");
    expect(md).not.toContain("\\]");
  });

  test("backtick inline code survives", async () => {
    const md = await htmlToMarkdown("<span>`code`</span>");
    expect(md).toContain("`code`");
    expect(md).not.toContain("\\`");
  });

  test("heading hash survives at line start", async () => {
    // Xcode-shaped `<p># Heading</p>` paste; escape override
    // keeps the `#` verbatim so it renders as h1.
    const md = await htmlToMarkdown("<p># Heading</p>");
    expect(md).toContain("# Heading");
    expect(md).not.toContain("\\#");
  });

  test("list dash survives at line start", async () => {
    const md = await htmlToMarkdown("<p>- item</p>");
    expect(md).toContain("- item");
    // Turndown's default escape emits `\-` for line-start
    // dashes (would-be list markers); the override stops it.
    expect(md).not.toMatch(/\\-\s*item/);
  });

  test("rich HTML still converts to markdown via turndown rules", async () => {
    // The escape override only changes plain-text node
    // serialisation; structural conversion (b -> ** / a -> []())
    // still happens through turndown's rules so this paste
    // pathway keeps its raison d'être.
    const md = await htmlToMarkdown("<p><b>real bold</b> and <i>real italic</i></p>");
    expect(md).toContain("**real bold**");
    expect(md).toContain("*real italic*");
  });
});
