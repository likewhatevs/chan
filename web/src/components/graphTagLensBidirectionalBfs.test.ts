import { describe, expect, test } from "vitest";
import graphPanel from "./GraphPanel.svelte?raw";

// Phase-13 round-1 closing (B8): the tag lens BFS in
// `computeScopedNodeSet` was forward-only (only `e.source` ->
// `e.target`). The backend emits tag edges as
// `source: <file>, target: <tagId>`, so seeding from the tag
// node would never traverse the incoming file->tag edges and
// the lens rendered empty for the entire round-1 smoke. The
// fix is a BIDIRECTIONAL BFS, same shape the contact arm has
// shipped since slice 2b. This file pins the contract.

describe("closing B8: tag lens BFS is bidirectional", () => {
  test("tag arm walks edges in both directions", () => {
    // Match the body of the `currentScope.kind === \"tag\"` arm,
    // then assert both the forward (`e.source` -> `e.target`)
    // and the reverse (`e.target` -> `e.source`) checks are
    // present before the early-exit on an empty frontier. The
    // assertion is intentionally string-shape rather than a
    // pure-function unit because `computeScopedNodeSet` is a
    // `$derived` inside the .svelte component.
    expect(graphPanel).toMatch(
      /if \(currentScope\.kind === "tag"\) \{[\s\S]{1,1600}if \(frontier\.has\(e\.source\) && !visited\.has\(e\.target\)\) \{[\s\S]{1,200}next\.add\(e\.target\);[\s\S]{1,200}visited\.add\(e\.target\);[\s\S]{1,300}if \(frontier\.has\(e\.target\) && !visited\.has\(e\.source\)\) \{[\s\S]{1,200}next\.add\(e\.source\);[\s\S]{1,200}visited\.add\(e\.source\);[\s\S]{1,200}if \(next\.size === 0\) break;/,
    );
  });

  test("tag arm no longer carries the forward-only G9 comment", () => {
    // The old forward-only rationale comment ("forward-only BFS
    // (outgoing edges only)") would mislead a future reader now
    // that the BFS is bidirectional; the bidirectional rationale
    // is recorded inline above the arm instead.
    expect(graphPanel).not.toMatch(
      /forward-only BFS \(outgoing edges only\)/,
    );
  });
});
