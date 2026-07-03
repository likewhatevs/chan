// @vitest-environment jsdom

import { mount, unmount } from "svelte";
import { afterEach, describe, expect, test, vi } from "vitest";

import ExcalidrawCanvas from "./ExcalidrawCanvas.svelte";
import canvasSrc from "./ExcalidrawCanvas.svelte?raw";
import fileEditorSrc from "../components/FileEditorTab.svelte?raw";

// Excalidraw + React are heavy; mock the three runtime modules the wrapper
// dynamic-imports so mounting the island in jsdom never pulls the real
// React runtime (mirrors diagram.test.ts). vi.mock is hoisted and
// intercepts dynamic imports too; the spies go through vi.hoisted so the
// hoisted mock factory can reference them without a TDZ error.
const { createRootMock, renderMock, unmountMock } = vi.hoisted(() => {
  const renderMock = vi.fn();
  const unmountMock = vi.fn();
  const createRootMock = vi.fn(() => ({ render: renderMock, unmount: unmountMock }));
  return { createRootMock, renderMock, unmountMock };
});
vi.mock("react-dom/client", () => ({ createRoot: createRootMock }));
vi.mock("react", () => ({
  createElement: (type: unknown, props: unknown) => ({ type, props }),
}));
vi.mock("@excalidraw/excalidraw", () => ({
  Excalidraw: () => null,
  serializeAsJSON: () => "{}",
}));

const mounted: Array<Record<string, unknown>> = [];

afterEach(() => {
  for (const c of mounted.splice(0)) unmount(c);
  document.body.innerHTML = "";
  createRootMock.mockClear();
  renderMock.mockClear();
  unmountMock.mockClear();
});

describe("ExcalidrawCanvas island", () => {
  test("creates exactly one React root and renders the board", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(
      mount(ExcalidrawCanvas, {
        target,
        props: { content: "", dark: false, onSceneChange: () => {} },
      }),
    );
    // onMount awaits the dynamic imports before it renders.
    await vi.waitFor(() => expect(renderMock).toHaveBeenCalled());
    expect(createRootMock).toHaveBeenCalledTimes(1);
    expect(target.querySelector(".excalidraw-host")).not.toBeNull();
  });

  test("unmounting tears the React root down", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    const comp = mount(ExcalidrawCanvas, {
      target,
      props: { content: "", dark: false, onSceneChange: () => {} },
    });
    await vi.waitFor(() => expect(renderMock).toHaveBeenCalled());
    unmount(comp);
    expect(unmountMock).toHaveBeenCalledTimes(1);
  });

  test("the board host follows the shared page-width cap", () => {
    expect(canvasSrc).toContain("width: min(100%, var(--chan-page-max-width, 100%))");
    expect(canvasSrc).toContain(".excalidraw-shell");
    expect(canvasSrc).toContain("var(--page-shade)");
  });
});

describe("inactive canvas tab hides via display:none (WKWebView island leak)", () => {
  // A GPU-composited Excalidraw island (the zoom/undo footer) leaks through
  // an ancestor's visibility:hidden under the flip-card's preserve-3d
  // context in WKWebView; hiding the shell with display:none stops it.
  test("the shell gains an offscreen hook toggled off the active prop", () => {
    expect(canvasSrc).toContain("class:offscreen={!active}");
    expect(canvasSrc).toMatch(/\.excalidraw-shell\.offscreen \{\s*display: none;\s*\}/);
  });

  test("FileEditorTab passes active into the canvas island", () => {
    expect(fileEditorSrc).toMatch(/<ExcalidrawCanvas[\s\S]*?\{active\}/);
  });

  test("mounting with active:false applies the offscreen class", () => {
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(
      mount(ExcalidrawCanvas, {
        target,
        props: { content: "", dark: false, active: false, onSceneChange: () => {} },
      }),
    );
    const shell = target.querySelector(".excalidraw-shell");
    expect(shell).not.toBeNull();
    expect(shell?.classList.contains("offscreen")).toBe(true);
  });

  test("the active prop defaults true so a plain mount is not hidden", () => {
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(
      mount(ExcalidrawCanvas, {
        target,
        props: { content: "", dark: false, onSceneChange: () => {} },
      }),
    );
    const shell = target.querySelector(".excalidraw-shell");
    expect(shell?.classList.contains("offscreen")).toBe(false);
  });
});

describe("excalidraw stays out of the eager bundle", () => {
  test("the wrapper dynamic-imports react-dom, react, and excalidraw", () => {
    expect(canvasSrc).toMatch(/import\("react-dom\/client"\)/);
    expect(canvasSrc).toMatch(/import\("react"\)/);
    expect(canvasSrc).toMatch(/import\("@excalidraw\/excalidraw"\)/);
    // A static runtime import would drag React into the eager editor bundle.
    expect(canvasSrc).not.toMatch(/from "react"/);
    expect(canvasSrc).not.toMatch(/from "react-dom\/client"/);
    expect(canvasSrc).not.toMatch(/from "@excalidraw\/excalidraw"/);
  });

  test("index.css is a side-effect import so it rides the async chunk", () => {
    expect(canvasSrc).toMatch(/import "@excalidraw\/excalidraw\/index\.css"/);
  });

  test("FileEditorTab reaches the wrapper only via dynamic import", () => {
    expect(fileEditorSrc).toMatch(/import\("\.\.\/editor\/ExcalidrawCanvas\.svelte"\)/);
    expect(fileEditorSrc).not.toMatch(/from "\.\.\/editor\/ExcalidrawCanvas\.svelte"/);
  });
});
