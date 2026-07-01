import { describe, expect, test } from "vitest";
import graphPanel from "./GraphPanel.svelte?raw";

// The semantic BFS lenses (mention / tag / contact) surface a seed's
// neighbourhood but stop at the referencing documents, so a surfaced
// doc's OTHER @@handle / #tag / language edges point one hop past the
// frontier and get culled by the both-endpoints edge filter. Each arm
// closes over those incident meta-nodes via pullMetaNeighbours so every
// surfaced document renders its full first-order semantic edge set.
// Asserted as a source shape because scopedNodeIds is a $derived inside
// the Svelte component, not a pure function (the behaviour itself is
// covered by graph/lensClosure.test.ts).

describe("semantic lens meta-neighbour closure is wired", () => {
  test("GraphPanel imports pullMetaNeighbours from the graph module", () => {
    expect(graphPanel).toMatch(
      /import \{ pullMetaNeighbours \} from "\.\.\/graph\/lensClosure"/,
    );
  });

  for (const kind of ["tag", "mention", "contact"] as const) {
    test(`${kind} arm closes meta-neighbours before the spine pull`, () => {
      expect(graphPanel).toMatch(
        new RegExp(
          `if \\(currentScope\\.kind === "${kind}"\\) \\{[\\s\\S]{1,1800}pullMetaNeighbours\\(visited, nodes, edges\\);[\\s\\S]{0,200}pullContainsSpine\\(visited\\);[\\s\\S]{0,80}return visited;`,
        ),
      );
    });
  }
});
