// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import { DOC_CONTAINER_CLASS, buildDocDom, docCss } from "./doc_dom";

vi.mock("./mermaid_render", () => ({
  renderMermaid: vi.fn(async (_source: string, dark: boolean) => ({
    ok: true,
    svg: `<svg data-mermaid-theme="${dark ? "dark" : "light"}"></svg>`,
  })),
}));

vi.mock("./excalidraw_render", () => ({
  renderExcalidraw: vi.fn(async () => ({ ok: true, svg: "<svg></svg>" })),
  renderExcalidrawFile: vi.fn(async (_url: string, dark: boolean) => ({
    ok: true,
    svg: `<svg data-excalidraw-theme="${dark ? "dark" : "light"}"></svg>`,
  })),
}));

afterEach(() => {
  document.body.innerHTML = "";
});

describe("buildDocDom", () => {
  test("renders markdown into a scoped, width-fixed container", () => {
    const { root, content } = buildDocDom({
      markdown: "# Title\n\nbody text\n",
      path: "notes/doc.md",
      theme: "light",
      contentWidthPx: 669,
    });
    document.body.append(root);

    expect(root.classList.contains(DOC_CONTAINER_CLASS)).toBe(true);
    expect(root.style.width).toBe("669px");
    expect(root.style.colorScheme).toBe("light");
    expect(root.querySelector("style")?.textContent).toContain(
      `.${DOC_CONTAINER_CLASS} h1`,
    );
    expect(content.querySelector("h1")?.textContent).toBe("Title");
    expect(content.textContent).toContain("body text");
  });

  test("hydrates mermaid fences through the slide renderer path", async () => {
    const { root, completion } = buildDocDom({
      markdown: "```mermaid\nflowchart LR\n  A --> B\n```\n",
      path: "notes/doc.md",
      theme: "dark",
      contentWidthPx: 669,
    });
    document.body.append(root);

    await completion;
    expect(root.querySelector("code.language-mermaid")).toBeNull();
    expect(
      root.querySelector(".md-slide-diagram svg")?.getAttribute(
        "data-mermaid-theme",
      ),
    ).toBe("dark");
  });

  test("hydrates excalidraw embeds and image grammar", async () => {
    const { root, completion } = buildDocDom({
      markdown: "![](board.excalidraw)\n\n![](photo.png#w=120&right)\n",
      path: "notes/doc.md",
      theme: "dark",
      contentWidthPx: 669,
    });
    document.body.append(root);

    await completion;
    expect(
      root.querySelector(".md-slide-excalidraw-body svg")?.getAttribute(
        "data-excalidraw-theme",
      ),
    ).toBe("dark");
    const img = root.querySelector("img")!;
    expect(img.style.width).toBe("120px");
    expect(img.classList.contains("chan-slide-align-right")).toBe(true);
  });

  test("keeps page-break markers invisible", () => {
    const { root } = buildDocDom({
      markdown: 'a\n\n<hr class="chan-page-break">\n\nb\n',
      path: "doc.md",
      theme: "light",
      contentWidthPx: 669,
    });
    document.body.append(root);
    expect(root.querySelector("hr.chan-page-break")).not.toBeNull();
    expect(docCss()).toContain(`.${DOC_CONTAINER_CLASS} hr.chan-page-break`);
  });
});

describe("docCss", () => {
  test("scopes every selector under the container class", () => {
    const selectors = docCss()
      .split("}")
      .map((block) => block.split("{")[0]!.trim())
      .filter(Boolean);
    for (const selector of selectors) {
      for (const part of selector.split(",")) {
        expect(part.trim().startsWith(`.${DOC_CONTAINER_CLASS}`)).toBe(true);
      }
    }
  });
});
