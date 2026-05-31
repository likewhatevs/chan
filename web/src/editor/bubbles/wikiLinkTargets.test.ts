import { describe, expect, test } from "vitest";
import client from "../../api/client.ts?raw";
import types from "../../api/types.ts?raw";
import wiki from "./wiki.ts?raw";

describe("wiki file completion uses graph link targets", () => {
  test("typed API client exposes /api/link-targets with q and limit", () => {
    expect(types).toMatch(/export type LinkTarget = \{/);
    expect(client).toMatch(
      /linkTargets: \(q: string, limit = 10\) => \{[\s\S]*?new URLSearchParams\(\{ q, limit: String\(limit\) \}\)[\s\S]*?req<LinkTarget\[\]>\("GET", `\/api\/link-targets\?\$\{params\}`\)/,
    );
  });

  test("file-mode wiki bubble calls api.linkTargets instead of api.search", () => {
    expect(wiki).toMatch(/\.linkTargets\(query, SEARCH_LIMIT\)/);
    expect(wiki).not.toMatch(/\.search\(query, SEARCH_LIMIT/);
  });

  test("file-mode wiki bubble keeps file and heading targets", () => {
    expect(wiki).toMatch(/let fileHits: LinkTarget\[\] = \[\];/);
    expect(wiki).toMatch(/fileHits = results;/);
    expect(wiki).not.toMatch(/\.filter\(\(hit\) => hit\.kind === "File"\)/);
  });

  test("file-mode commit extracts the heading anchor and defers to fileLinkInsert", () => {
    expect(wiki).toMatch(
      /const lt = hit as LinkTarget;[\s\S]*?const anchor = lt\.kind === "Heading" \? \(lt\.anchor \?\? null\) : null;[\s\S]*?insert = fileLinkInsert\(lt\.path, anchor, raw\);/,
    );
  });

  test("fileLinkInsert emits relative markdown by default and keeps wiki links only in a wiki-mode file", () => {
    // Default (markdown-mode file): relative markdown via links.ts.
    expect(wiki).toMatch(
      /return wikiLinkToMarkdown\(\s*path,\s*undefined,\s*anchor \?\? undefined,\s*opts\.fromPath \?\? undefined,\s*\);/,
    );
    // Wiki-mode file: preserve the `[[path#anchor]]` form.
    expect(wiki).toMatch(
      /if \(fileUsesWikiLinks\) \{[\s\S]*?const ref = anchor \? `\$\{path\}#\$\{anchor\}` : path;[\s\S]*?return `\[\[\$\{ref\}\]\]`;/,
    );
    // The per-file style snapshot is a complete `[[...]]` match.
    expect(wiki).toMatch(/const fileUsesWikiLinks = WIKI_LINK_RE\.test\(/);
  });
});
