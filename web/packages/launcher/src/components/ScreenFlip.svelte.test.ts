// The flip shell plays the shape-aware turn when its trigger bumps: the same
// mechanics as the workspace-app pane's side flip (Pane.test.ts "side changes
// trigger a shape-aware flip animation"), driven here by the forward-only
// `flips` counter instead of a visible-side change.

import { describe, expect, test, afterEach } from "vitest";
import { mount, unmount, flushSync, createRawSnippet } from "svelte";
import ScreenFlip from "./ScreenFlip.svelte";

const children = createRawSnippet(() => ({ render: () => "<p>body</p>" }));

function nextFrame(): Promise<void> {
  return new Promise((r) => requestAnimationFrame(() => r()));
}

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;
const originalRect = HTMLElement.prototype.getBoundingClientRect;

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
  HTMLElement.prototype.getBoundingClientRect = originalRect;
});

function pinShellRect(width: number, height: number): void {
  HTMLElement.prototype.getBoundingClientRect = function () {
    if (this.classList.contains("screen-flip")) {
      return {
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: width,
        bottom: height,
        width,
        height,
        toJSON: () => ({}),
      } as DOMRect;
    }
    return originalRect.call(this);
  };
}

describe("ScreenFlip", () => {
  test("a trigger bump plays the shape-aware turn; mount does not", async () => {
    pinShellRect(320, 120);
    const props = $state({ flips: 0, backLabel: "Gateways", children });
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(ScreenFlip, { target, props });
    flushSync();

    const shell = target.querySelector<HTMLElement>(".screen-flip")!;
    expect(shell).not.toBeNull();
    expect(shell.classList.contains("flipActive")).toBe(false);
    expect(shell.textContent).toContain("body");

    props.flips = 1;
    flushSync();
    await nextFrame();
    flushSync();

    // 320x120 is wide, so the turn is horizontal: rotateX.
    expect(shell.classList.contains("flipActive")).toBe(true);
    expect(shell.classList.contains("flipHorizontal")).toBe(true);
    expect(shell.classList.contains("flipVertical")).toBe(false);
    expect(shell.style.getPropertyValue("--screen-flip-start")).toContain("rotateX(-180deg)");
    expect(shell.querySelector(".screen-flip-inner")?.getAttribute("data-flip-label")).toBe(
      "Gateways",
    );
  });

  test("a tall shell turns vertically", async () => {
    pinShellRect(120, 320);
    const props = $state({ flips: 0, backLabel: "Computers", children });
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(ScreenFlip, { target, props });
    flushSync();

    props.flips = 1;
    flushSync();
    await nextFrame();
    flushSync();

    const shell = target.querySelector<HTMLElement>(".screen-flip")!;
    expect(shell.classList.contains("flipVertical")).toBe(true);
    expect(shell.style.getPropertyValue("--screen-flip-start")).toContain("rotateY(-180deg)");
  });
});
