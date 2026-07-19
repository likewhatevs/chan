import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";
import {
  lensClosure,
  type LensDirection,
  type LensEdge,
  type LensNode,
} from "./lensClosure";

type GraphFixture = { nodes: LensNode[]; edges: LensEdge[] };
type ExpectedCase = {
  lens: string;
  depth: number;
  seed: string;
  direction: LensDirection;
  meta_closure: boolean;
  language_one_hop: boolean;
  visible_node_ids: string[];
  relationship_keys: string[];
};

const fixtureRoot = "../../../fixtures/workspace-search";
const graph = JSON.parse(
  readFileSync(`${fixtureRoot}/graph.json`, "utf8"),
) as GraphFixture;
const expected = JSON.parse(
  readFileSync(`${fixtureRoot}/expected.json`, "utf8"),
) as { cases: ExpectedCase[] };

describe("workspace search lens parity golden", () => {
  for (const golden of expected.cases) {
    test(`${golden.lens} depth ${golden.depth}`, () => {
      const actual = lensClosure(graph.nodes, graph.edges, {
        seedIds: [golden.seed],
        depth: golden.depth,
        direction: golden.direction,
        metaClosure: golden.meta_closure,
        languageOneHop: golden.language_one_hop,
        containmentOnly: golden.lens === "directory",
      });
      expect(actual.nodeIds).toEqual(golden.visible_node_ids);
      expect(actual.relationshipKeys).toEqual(golden.relationship_keys);
    });
  }
});
