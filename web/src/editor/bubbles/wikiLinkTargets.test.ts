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

  test("heading link-target hits insert anchored wiki links", () => {
    expect(wiki).toMatch(
      /function linkTargetRef\(hit: LinkTarget\): string \{[\s\S]*?if \(hit\.kind === "Heading" && hit\.anchor\) \{[\s\S]*?return `\$\{hit\.path\}#\$\{hit\.anchor\}`;/,
    );
  });
});
