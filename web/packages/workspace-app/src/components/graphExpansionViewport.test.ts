import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";
import canvas from "./GraphCanvas.svelte?raw";

describe("directory expansion viewport fit", () => {
  test("GraphPanel sends a one-shot expansion fit request to GraphCanvas", () => {
    expect(graph).toMatch(/let expansionFitNonce = 0;/);
    expect(graph).toMatch(
      /let expansionFitRequest = \$state<\{ nonce: number; ids: string\[\] \} \| null>\(null\);/,
    );
    expect(graph).toMatch(/\{expansionFitRequest\}/);
  });

  test("fit request includes expanded dir, parent dir, and newly visible nodes", () => {
    expect(graph).toMatch(
      /async function requestExpansionFitAfterUpdate\([\s\S]*?path: string,[\s\S]*?before: Set<string>,[\s\S]*?\): Promise<void> \{[\s\S]*?await tick\(\);[\s\S]*?new Set<string>\(\[renderedDirectoryId\(path\)\]\);[\s\S]*?if \(path\) ids\.add\(renderedDirectoryId\(parentDirOf\(path\)\)\);[\s\S]*?for \(const id of visibleNodeIds\) \{[\s\S]*?if \(!before\.has\(id\)\) ids\.add\(id\);/,
    );
  });

  test("semantic and filesystem expansion snapshot before ids and request fit only in expand branch", () => {
    expect(graph).toMatch(
      /function toggleSemanticDirExpand\(path: string\): void \{[\s\S]*?if \(graphState\.expanded\[path\]\) \{[\s\S]*?delete graphState\.expanded\[path\];[\s\S]*?\} else \{[\s\S]*?const before = new Set\(visibleNodeIds\);[\s\S]*?graphState\.expanded\[path\] = true;[\s\S]*?void requestExpansionFitAfterUpdate\(path, before\);/,
    );
    expect(graph).toMatch(
      /async function toggleDirExpand\(path: string\): Promise<void> \{[\s\S]*?if \(graphState\.expanded\[path\]\) \{[\s\S]*?delete graphState\.expanded\[path\];[\s\S]*?\} else \{[\s\S]*?const before = new Set\(visibleNodeIds\);[\s\S]*?graphState\.expanded\[path\] = true;[\s\S]*?if \(!dirChildrenLoaded\(path\)\) await fetchDirChildren\(path\);[\s\S]*?await requestExpansionFitAfterUpdate\(path, before\);/,
    );
  });
});

describe("GraphCanvas expansion fit behavior", () => {
  test("expansion fit has its own refit state and request prop", () => {
    expect(canvas).toMatch(/type ExpansionFitRequest = \{ nonce: number; ids: string\[\] \};/);
    expect(canvas).toMatch(/expansionFitRequest\?: ExpansionFitRequest \| null;/);
    expect(canvas).toMatch(/let expansionRefit: \{ until: number; ids: Set<string> \} \| null = null;/);
  });

  test("expansion fit targets requested ids and caps zoom at current scale", () => {
    expect(canvas).toMatch(
      /const t = computeFitForNodes\(32, expansionRefit\.ids, false, transform\.k\);/,
    );
    expect(canvas).toMatch(
      /function scheduleExpansionFit\(ids: string\[\], ms: number\): void \{[\s\S]*?const set = new Set\(ids\);[\s\S]*?computeFitForNodes\(32, set, false, transform\.k\);/,
    );
    expect(canvas).toMatch(/if \(maxK !== undefined\) k = Math\.min\(k, maxK\);/);
  });

  test("expansion fit bypasses userInteracted but manual interaction cancels it", () => {
    expect(canvas).toMatch(
      /function applyExpansionFitRequest\(request: ExpansionFitRequest\): void \{[\s\S]*?seenExpansionFitNonce = request\.nonce;[\s\S]*?scheduleExpansionFit\(request\.ids, 900\);/,
    );
    expect(canvas).toMatch(
      /if \(!request \|\| request\.nonce === seenExpansionFitNonce\) return;[\s\S]*?if \(!sim\) \{[\s\S]*?void tick\(\)\.then\(\(\) => \{[\s\S]*?applyExpansionFitRequest\(request\);[\s\S]*?return;[\s\S]*?applyExpansionFitRequest\(request\);/,
    );
    expect(canvas).toMatch(
      /function cancelRefit\(\): void \{[\s\S]*?expansionRefit = null;/,
    );
  });
});
