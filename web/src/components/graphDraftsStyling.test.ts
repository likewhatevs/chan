import { describe, expect, test } from "vitest";
import canvas from "./GraphCanvas.svelte?raw";
import types from "../api/types.ts?raw";

// `fullstack-a-66` slice e: Graph Drafts root styling +
// `drafts_link` edge. Closes the -a-66 umbrella.

describe("fullstack-a-66 slice e: GraphViewEdgeKind union extended", () => {
  test("`drafts_link` is in the GraphViewEdgeKind union", () => {
    expect(types).toMatch(
      /export type GraphViewEdgeKind =[\s\S]*?\| "drafts_link";/,
    );
  });
});

describe("fullstack-a-66 slice e: GraphCanvas RenderedEdgeKind", () => {
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

  // `phase-11` Slice F split `link` edges into their own
  // per-source-document-kind pass, so the single-stroke-per-kind
  // iteration no longer lists `link`; `drafts_link` still rides this
  // loop.
  test("kind-iteration order includes `drafts_link`", () => {
    expect(canvas).toMatch(
      /\["tag", "mention", "contains", "language", "group", "drafts_link"\] as const/,
    );
  });
});

describe("fullstack-a-66 slice e: drafts_link edge styling", () => {
  test("strokeStyle for `drafts_link` maps to theme.drafts", () => {
    expect(canvas).toMatch(
      /kind === "drafts_link" \? theme\.drafts/,
    );
  });

  // `phase-11` Slice F relocated the alpha bump into the `strokePass`
  // call: drafts_link is the one kind passed 0.4 instead of the 0.18
  // base, preserving the category-boundary emphasis.
  test("alpha is bumped from 0.18 to 0.4 for `drafts_link`", () => {
    expect(canvas).toMatch(
      /strokePass\(edgesByKind\[kind\], strokeForKind\(kind\), kind === "drafts_link" \? 0\.4 : 0\.18\);/,
    );
  });
});

describe("fullstack-a-66 slice e: theme.drafts wiring", () => {
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

describe("fullstack-a-66 slice e: Drafts directory node tinted", () => {
  test("isDraftsRoot derived from `n.kind === 'folder' && n.id === 'directory:Drafts'`", () => {
    expect(canvas).toMatch(
      /const isDraftsRoot = n\.kind === "folder" && n\.id === "directory:Drafts";/,
    );
  });

  test("fill branch routes isDraftsRoot to theme.drafts before the regular folder fallback", () => {
    expect(canvas).toMatch(
      /isGhost\s*\?\s*theme\.bgCard\s*:\s*isDraftsRoot \? theme\.drafts/,
    );
  });

  test("rationale comment cites cross-surface consistency (FB row + inspector chip)", () => {
    expect(canvas).toMatch(
      /fullstack-a-66[\s\S]*?slice e[\s\S]*?Drafts yellow[\s\S]*?FB row \+ the inspector chip/i,
    );
  });
});
