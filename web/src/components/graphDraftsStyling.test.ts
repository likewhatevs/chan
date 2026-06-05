import { describe, expect, test } from "vitest";
import canvas from "./GraphCanvas.svelte?raw";
import types from "../api/types.ts?raw";

// Graph Drafts root styling. The drafts dir is a normal directory node
// (id `directory:${draftsDir()}`), tinted by matching its id. There is
// no synthesized `drafts_link` edge anymore.

describe("drafts_link edge kind removed", () => {
  test("`drafts_link` is gone from the GraphViewEdgeKind union", () => {
    expect(types).not.toMatch(/drafts_link/);
  });

  test("`drafts_link` is gone from GraphCanvas", () => {
    expect(canvas).not.toMatch(/drafts_link/);
  });

  test("edgesByKind no longer preallocates a drafts bucket", () => {
    expect(canvas).toMatch(
      /const edgesByKind: Record<RenderedEdgeKind, DEdge\[\]> = \{[\s\S]*?link: \[\], tag: \[\], mention: \[\], contains: \[\], language: \[\], group: \[\],[\s\S]*?\};/,
    );
  });

  test("kind-iteration order drops the drafts kind", () => {
    expect(canvas).toMatch(
      /\["tag", "mention", "contains", "language", "group"\] as const/,
    );
  });
});

describe("theme.drafts wiring (node fill)", () => {
  test("ThemeColors interface declares `drafts: string`", () => {
    expect(canvas).toMatch(/\bdrafts: string;/);
  });

  test("readTheme pulls --fb-drafts-fg into theme.drafts", () => {
    expect(canvas).toMatch(
      /drafts: v\("--fb-drafts-fg", "#e3b341"\),/,
    );
  });

  test("theme state initial value includes drafts", () => {
    expect(canvas).toMatch(/drafts: "#e3b341"/);
  });
});

describe("Drafts directory node tinted", () => {
  test("isDraftsRoot derived from the configured draftsDir() node id", () => {
    expect(canvas).toMatch(
      /const isDraftsRoot =\s*n\.kind === "folder" && n\.id === `directory:\$\{draftsDir\(\)\}`;/,
    );
  });

  test("GraphCanvas imports draftsDir from the workspace leaf module", () => {
    expect(canvas).toMatch(
      /import \{ draftsDir \} from "\.\.\/state\/workspace\.svelte";/,
    );
  });

  test("fill branch routes isDraftsRoot to theme.drafts before the regular folder fallback", () => {
    // An `indexFill ?? (...)` sits between the ghost guard and the
    // regular-fill cascade so the Dashboard indexing slide can
    // override folder colours with the indexing palette. The Drafts
    // tint wins over the standard folder fall-back inside the
    // parenthesised cascade; pin both halves so a future refactor
    // can't accidentally re-order them.
    expect(canvas).toMatch(
      /isGhost\s*\?\s*theme\.bgCard\s*:\s*indexFill \?\? \(\s*isDraftsRoot \? theme\.drafts/,
    );
  });

  test("rationale comment cites cross-surface consistency (FB row + inspector chip)", () => {
    expect(canvas).toMatch(
      /Drafts yellow[\s\S]*?FB row \+ the inspector chip/i,
    );
  });
});
