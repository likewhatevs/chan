// @vitest-environment jsdom

import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, describe, expect, test, vi } from "vitest";
import { chanMarkdown } from "../markdown/grammar";
import { renderExcalidrawFile } from "../excalidraw_render";
import { imageDecorations } from "./image";

vi.mock("../excalidraw_render", () => ({
  renderExcalidrawFile: vi.fn(async (_url: string, dark: boolean) => ({
    ok: true,
    svg: `<svg data-excalidraw-theme="${dark ? "dark" : "light"}"></svg>`,
  })),
}));

function mount(
  doc: string,
  dark: boolean,
): { parent: HTMLElement; view: EditorView } {
  const parent = document.createElement("div");
  document.body.append(parent);
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc,
      extensions: [
        chanMarkdown(),
        imageDecorations({
          getCurrentPath: () => "slides-test.md",
          isDark: () => dark,
        }),
      ],
    }),
  });
  return { parent, view };
}

afterEach(() => {
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

describe("excalidraw image embeds", () => {
  test("render a .excalidraw image source as a themed static SVG", async () => {
    const { parent, view } = mount("![](board.excalidraw#w=320)", true);
    try {
      await vi.waitFor(() => {
        expect(
          parent
            .querySelector(".cm-md-excalidraw-embed svg")
            ?.getAttribute("data-excalidraw-theme"),
        ).toBe("dark");
      });
      expect(parent.querySelector(".cm-md-image-wrap[data-excalidraw='true']")).toBeTruthy();
      expect(parent.querySelector(".cm-md-image-wrap img")).toBeNull();
      expect(parent.querySelector<HTMLElement>(".cm-md-excalidraw-embed")?.style.width).toBe(
        "320px",
      );
      expect(renderExcalidrawFile).toHaveBeenCalledWith(
        expect.stringContaining("/api/files/board.excalidraw"),
        true,
      );
    } finally {
      view.destroy();
      parent.remove();
    }
  });

  test("resize handle persists the width fragment", async () => {
    const { parent, view } = mount("![](board.excalidraw#w=320)", true);
    try {
      await vi.waitFor(() => {
        expect(parent.querySelector(".cm-md-excalidraw-embed svg")).toBeTruthy();
      });
      const embed = parent.querySelector<HTMLElement>(".cm-md-excalidraw-embed");
      const handle = parent.querySelector<HTMLElement>(".cm-md-image-handle");
      expect(embed).toBeTruthy();
      expect(handle).toBeTruthy();
      embed!.getBoundingClientRect = () =>
        ({
          x: 0,
          y: 0,
          left: 0,
          top: 0,
          right: parseInt(embed!.style.width || "320", 10),
          bottom: 120,
          width: parseInt(embed!.style.width || "320", 10),
          height: 120,
          toJSON: () => {},
        }) as DOMRect;

      handle!.dispatchEvent(
        new MouseEvent("mousedown", {
          bubbles: true,
          button: 0,
          clientX: 10,
        }),
      );
      document.dispatchEvent(
        new MouseEvent("mousemove", { bubbles: true, clientX: 70 }),
      );
      document.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));

      expect(view.state.doc.toString()).toBe("![](board.excalidraw#w=380)");
    } finally {
      view.destroy();
      parent.remove();
    }
  });
});
