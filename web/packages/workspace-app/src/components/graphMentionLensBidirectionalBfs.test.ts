import { describe, expect, test } from "vitest";
import graphPanel from "./GraphPanel.svelte?raw";

// The mention lens mirrors the tag lens: the backend emits mention
// edges as `source: <file>, target: <@@Name>`, so seeding from the
// mention node needs a BIDIRECTIONAL BFS to traverse the incoming
// file->mention edges (the backlinks the lens exists to surface).
// A forward-only walk would render the lens empty.

describe("mention lens BFS is bidirectional", () => {
  test("mention arm walks edges in both directions", () => {
    // Asserted as a string shape because scopedNodeIds is a $derived
    // inside the Svelte component, not a pure function.
    expect(graphPanel).toMatch(
      /if \(currentScope\.kind === "mention"\) \{[\s\S]{1,1600}if \(frontier\.has\(e\.source\) && !visited\.has\(e\.target\)\) \{[\s\S]{1,200}next\.add\(e\.target\);[\s\S]{1,200}visited\.add\(e\.target\);[\s\S]{1,300}if \(frontier\.has\(e\.target\) && !visited\.has\(e\.source\)\) \{[\s\S]{1,200}next\.add\(e\.source\);[\s\S]{1,200}visited\.add\(e\.source\);[\s\S]{1,200}if \(next\.size === 0\) break;/,
    );
  });

  test("mention arm pulls the directory spine before returning", () => {
    // Re-anchor: every file the lens surfaced gets its `contains`
    // spine pulled in so no file renders edgeless.
    expect(graphPanel).toMatch(
      /if \(currentScope\.kind === "mention"\) \{[\s\S]{1,1800}pullContainsSpine\(visited\);[\s\S]{0,80}return visited;/,
    );
  });
});
