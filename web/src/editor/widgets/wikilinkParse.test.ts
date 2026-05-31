import { describe, expect, test } from "vitest";

import { parseInternalLink } from "./wikilink";
import { wikiLinkToMarkdown } from "../links";

// `[[` completion writes relative-markdown links with the path
// percent-encoded (so a filename with a space lands on disk as
// `Brazilian%20Rice.md`). The backend graph scanner (pulldown-cmark)
// decodes the destination before resolving, so the editor's resolver
// must decode too or a perfectly valid on-disk edge renders as a broken
// link. These tests lock that encode (emit) / decode (resolve) contract.
describe("internal markdown link encode/decode round-trip", () => {
  test("percent-encoded spaced path resolves to the real workspace file", () => {
    const parsed = parseInternalLink(
      "./Brazilian%20Rice.md",
      "Brazilian Rice",
      "Recipes/Pasta.md",
    );
    expect(parsed?.target).toBe("Recipes/Brazilian Rice.md");
  });

  test("wikiLinkToMarkdown output parses back to the source path", () => {
    const md = wikiLinkToMarkdown(
      "Recipes/Brazilian Rice.md",
      undefined,
      undefined,
      "Recipes/Pasta.md",
    );
    expect(md).toBe("[Brazilian Rice](./Brazilian%20Rice.md)");
    const url = md.slice(md.indexOf("(") + 1, md.lastIndexOf(")"));
    const parsed = parseInternalLink(url, "Brazilian Rice", "Recipes/Pasta.md");
    expect(parsed?.target).toBe("Recipes/Brazilian Rice.md");
  });

  test("plain path and heading anchor round-trip without corruption", () => {
    const parsed = parseInternalLink(
      "../Welcome.md#getting-started",
      "Welcome",
      "Recipes/Pasta.md",
    );
    expect(parsed?.target).toBe("Welcome.md");
    expect(parsed?.anchor).toBe("getting-started");
  });

  test("a literal stray percent is left untouched, not dropped", () => {
    // `decodeURIComponent` throws on a lone `%`; the resolver must fall
    // back to the raw path instead of returning null.
    const parsed = parseInternalLink("./100%.md", "100%", "Notes/Pct.md");
    expect(parsed?.target).toBe("Notes/100%.md");
  });
});
