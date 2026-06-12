import { describe, expect, test } from "vitest";
import graphPanel from "./GraphPanel.svelte?raw";

// The tag / contact / language lenses BFS only along semantic
// edges, so their file nodes used to render with no edge up to a
// directory ("edgeless files"). The unified /api/graph payload already
// carries the directory `contains` spine to root; the fix re-includes
// it in each lens's visible set via `pullContainsSpine`, the same
// ancestor-pull file scope uses.
//
// Asserted as a source shape because `scopedNodeIds` is a $derived
// inside the Svelte component, not a pure function (same convention as
// graphTagLensBidirectionalBfs.test.ts). The real layout check is a
// browser smoke of a lang= / hashtag / mention lens.

describe("filtered lenses anchor files to the directory spine", () => {
  test("the shared ancestor-pull helper walks contains edges UP to a fixed point", () => {
    expect(graphPanel).toMatch(
      /function pullContainsSpine\(visited: Set<string>\): void \{[\s\S]{1,400}e\.kind === "contains" &&[\s\S]{1,120}visited\.has\(e\.target\) &&[\s\S]{1,120}!visited\.has\(e\.source\)[\s\S]{1,120}visited\.add\(e\.source\);/,
    );
  });

  test("the tag lens pulls the spine before returning its visible set", () => {
    expect(graphPanel).toMatch(
      /if \(currentScope\.kind === "tag"\) \{[\s\S]{1,1800}pullContainsSpine\(visited\);[\s\S]{0,80}return visited;/,
    );
  });

  test("the mention lens pulls the spine before returning its visible set", () => {
    expect(graphPanel).toMatch(
      /if \(currentScope\.kind === "mention"\) \{[\s\S]{1,1800}pullContainsSpine\(visited\);[\s\S]{0,80}return visited;/,
    );
  });

  test("the contact lens pulls the spine before returning its visible set", () => {
    expect(graphPanel).toMatch(
      /if \(currentScope\.kind === "contact"\) \{[\s\S]{1,1800}pullContainsSpine\(visited\);[\s\S]{0,80}return visited;/,
    );
  });

  test("the language lens pulls the spine before returning its visible set", () => {
    expect(graphPanel).toMatch(
      /if \(currentScope\.kind === "language"\) \{[\s\S]{1,1200}pullContainsSpine\(visited\);[\s\S]{0,80}return visited;/,
    );
  });
});
