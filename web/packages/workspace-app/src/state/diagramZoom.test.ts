// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { openDiagramZoom } from "./diagramZoom";

const SVG =
  '<svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10"/></svg>';

function backdrop(): HTMLElement | null {
  return document.querySelector(".md-diagram-zoom");
}
function layer(): HTMLElement | null {
  return document.querySelector(".md-diagram-zoom-layer");
}

afterEach(() => {
  document.querySelectorAll(".md-diagram-zoom").forEach((e) => e.remove());
});

describe("openDiagramZoom", () => {
  test("mounts a backdrop with the SVG and three zoom controls", () => {
    openDiagramZoom(SVG);
    const bd = backdrop()!;
    expect(bd).toBeTruthy();
    expect(layer()?.querySelector("svg")).toBeTruthy();
    expect(bd.querySelectorAll(".md-diagram-zoom-btn").length).toBe(3);
  });

  test("no-ops on an empty svg", () => {
    openDiagramZoom("");
    expect(backdrop()).toBeNull();
  });

  test("Escape dismisses and unhooks the document keydown", () => {
    openDiagramZoom(SVG);
    expect(backdrop()).toBeTruthy();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    expect(backdrop()).toBeNull();
  });

  test("a plain backdrop click dismisses", () => {
    openDiagramZoom(SVG);
    backdrop()!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(backdrop()).toBeNull();
  });

  test("+/arrow keys transform the layer and 0 resets it", () => {
    openDiagramZoom(SVG);
    const before = layer()!.style.transform;
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "+" }));
    expect(layer()!.style.transform).toContain("scale(1.2)");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    expect(layer()!.style.transform).toContain("-48px");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "0" }));
    expect(layer()!.style.transform).toBe(before);
  });

  test("captures its shortcuts so they do not leak to the editor", () => {
    openDiagramZoom(SVG);
    const ev = new KeyboardEvent("keydown", { key: "0", cancelable: true });
    document.dispatchEvent(ev);
    expect(ev.defaultPrevented).toBe(true);
  });
});
