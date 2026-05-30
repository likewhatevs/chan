import { describe, expect, test } from "vitest";
import canvas from "./GraphCanvas.svelte?raw";
import types from "../api/types.ts?raw";

// Graph Drafts root styling + `drafts_link` edge.

describe("GraphViewEdgeKind union extended", () => {
  test("`drafts_link` is in the GraphViewEdgeKind union", () => {
    expect(types).toMatch(
      /export type GraphViewEdgeKind =[\s\S]*?\| "drafts_link";/,
    );
  });
});

describe("GraphCanvas RenderedEdgeKind", () => {
  test("`drafts_link` is in the RenderedEdgeKind union", () => {
    expect(canvas).toMatch(
      /type RenderedEdgeKind =[\s\S]*?\| "drafts_link";/,
    );
  });

  test("edgesByKind preallocates a `drafts_link` bucket", () => {
    expect(canvas).toMatch(
      /const edgesByKind: Record<RenderedEdgeKind, DEdge\[\]> = \{[\s\S]*?drafts_link: \[\][\s\S]*?\};/,
    );
  });

  // `link` edges have their own per-source-document-kind pass, so the
  // single-stroke-per-kind iteration does not list `link`;
  // `drafts_link` rides this loop.
  test("kind-iteration order includes `drafts_link`", () => {
    expect(canvas).toMatch(
      /\["tag", "mention", "contains", "language", "group", "drafts_link"\] as const/,
    );
  });
});

describe("drafts_link edge styling", () => {
  test("strokeStyle for `drafts_link` maps to theme.drafts", () => {
    expect(canvas).toMatch(
      /kind === "drafts_link" \? theme\.drafts/,
    );
  });

  // The alpha bump lives in the `strokePass` call: drafts_link is the
  // one kind passed 0.4 instead of the 0.18 base, preserving the
  // category-boundary emphasis.
  test("alpha is bumped from 0.18 to 0.4 for `drafts_link`", () => {
    expect(canvas).toMatch(
      /strokePass\(edgesByKind\[kind\], strokeForKind\(kind\), kind === "drafts_link" \? 0\.4 : 0\.18\);/,
    );
  });
});

describe("theme.drafts wiring", () => {
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
  test("isDraftsRoot derived from `n.kind === 'folder' && n.id === 'directory:Drafts'`", () => {
    expect(canvas).toMatch(
      /const isDraftsRoot = n\.kind === "folder" && n\.id === "directory:Drafts";/,
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
