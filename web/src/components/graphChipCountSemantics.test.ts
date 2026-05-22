import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// `fullstack-a-63`: chip count semantics — node-tally, not edge-
// tally. Pre-`-a-63` the count loop iterated `edges` and bumped
// `c[kind]++` per edge kind, so a mention chip across 1973 mention
// edges fanning into ~48 contact nodes displayed `1973` instead of
// `48`. @@Alex's UX expectation: chip count = "how many of THIS
// thing is in the graph", which is the node count.
//
// Source-pin tests (raw string match) — the counts() derive is
// short + lives in a single $derived.by block + the new shape is
// distinctive enough to pin via regex without bringing the full
// Svelte runtime into the unit-test environment.

describe("fullstack-a-63: chip count loop is node-tally", () => {
  test("counts derive iterates `nodes` not `edges` for chip totals", () => {
    // Pre-`-a-63` shape (gone): `for (const e of edges) { ...
    // c[kind]++; }` — pin the absence.
    const stripped = graph
      .split("\n")
      .filter((line) => !line.trim().startsWith("//") && !line.trim().startsWith("///"))
      .join("\n");
    expect(stripped).not.toMatch(
      /const counts = \$derived\.by\([\s\S]*?for \(const e of edges\) \{[\s\S]*?c\[kind\]\+\+;/,
    );
  });

  test("counts derive walks nodes + bumps tag / mention / language by kind", () => {
    expect(graph).toMatch(
      /const counts = \$derived\.by\(\(\) => \{[\s\S]*?for \(const n of nodes\) \{[\s\S]*?if \(n\.kind === "tag"\) \{[\s\S]*?c\.tag\+\+;/,
    );
    expect(graph).toMatch(/if \(n\.kind === "mention"\) \{[\s\S]*?c\.mention\+\+;/);
    expect(graph).toMatch(/if \(n\.kind === "language"\) \{[\s\S]*?c\.language\+\+;/);
  });

  test("contact-discriminated file nodes add to mention count (chip toggle scope)", () => {
    // Mention chip hides BOTH mention-kind nodes AND contact-flagged
    // file nodes (see hiddenContactIds). Both bump c.mention so the
    // displayed count reflects everything the toggle hides.
    expect(graph).toMatch(/else if \(cls === "contact"\) c\.mention\+\+/);
  });

  test("img + markdown + source counts unchanged (already node-tally pre-`-a-63`)", () => {
    expect(graph).toMatch(/if \(cls === "img"\) c\.img\+\+/);
    expect(graph).toMatch(/else if \(cls === "doc"\) c\.markdown\+\+/);
    expect(graph).toMatch(/else if \(cls === "source"\) c\.source\+\+/);
  });

  test("folder count uses the folder-node walk (no edge tally)", () => {
    expect(graph).toMatch(/if \(n\.kind === "folder"\) \{[\s\S]*?c\.folder\+\+;/);
  });

  test("comment block documents the `-a-63` semantic correction", () => {
    expect(graph).toMatch(/`fullstack-a-63` semantic correction: chip counts are NODE/i);
  });
});
