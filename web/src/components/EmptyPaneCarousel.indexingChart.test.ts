import { describe, expect, test } from "vitest";
import carousel from "./EmptyPaneCarousel.svelte?raw";

// `fullstack-b-4`: the indexing-graph slide in the empty-pane
// carousel used to render at a fixed `viewBox="0 0 280 280"`,
// clipping any workspace whose hierarchy extended past the
// viewport with no way to pan or zoom. Parity with
// `GraphCanvas.svelte`: drag-to-pan, wheel-to-zoom, recenter
// affordance.
//
// These checks pin the source so a future refactor (notably the
// Round-2 backlog item that splits the carousel into Infographics
// tabs) can't accidentally drop the pan/zoom wiring at the chart
// level. The gestures sit on the chart's `<svg>` element, not on
// the slide container, so they carry forward through the
// container refactor.

describe("fullstack-b-4: EmptyPaneCarousel indexing chart pan / zoom", () => {
  test("chart transform state is local to the indexing slide", () => {
    expect(carousel).toContain("let chartTransform = $state({ tx: 0, ty: 0, scale: 1 })");
  });

  test("recenter resets translation + scale", () => {
    expect(carousel).toMatch(
      /function recenterChart\(\)[\s\S]*?chartTransform = \{ tx: 0, ty: 0, scale: 1 \}/,
    );
  });

  test("leaving the indexing slide resets the transform", () => {
    expect(carousel).toMatch(/if \(slideIndex !== 2\) \{\s*recenterChart\(\)/);
  });

  test("pointer drag on the SVG updates the transform", () => {
    expect(carousel).toMatch(/onpointerdown=\{onChartPointerDown\}/);
    expect(carousel).toMatch(/onpointermove=\{onChartPointerMove\}/);
    expect(carousel).toMatch(/onpointerup=\{onChartPointerUp\}/);
  });

  test("pointer down on a node is allowed to take the click", () => {
    // The node's onclick toggles selection. Pan-start has to
    // bail out when the pointerdown is on a node, otherwise the
    // pointer-capture swallows the click event the node needs.
    expect(carousel).toMatch(/target\?\.closest\(["']\.node["']\)/);
  });

  test("wheel zoom anchors the world point under the cursor", () => {
    // Same anchor math as `GraphCanvas.svelte`: solve for the new
    // (tx, ty) that keeps the world point under the cursor invariant
    // across the scale change.
    expect(carousel).toMatch(/p\.x - \(\(p\.x - chartTransform\.tx\) \* k\) \/ chartTransform\.scale/);
    expect(carousel).toMatch(/p\.y - \(\(p\.y - chartTransform\.ty\) \* k\) \/ chartTransform\.scale/);
  });

  test("recenter button is rendered", () => {
    expect(carousel).toMatch(/class="recenter-btn"/);
    expect(carousel).toMatch(/aria-label="recenter graph"/);
  });

  test("transform group wraps edges + nodes", () => {
    // The transform must wrap BOTH groups, not just one — panning
    // the nodes while leaving the edges anchored would split the
    // graph visually.
    expect(carousel).toMatch(
      /transform=\{`translate\(\$\{chartTransform\.tx\} \$\{chartTransform\.ty\}\) scale\(\$\{chartTransform\.scale\}\)`\}/,
    );
  });
});
