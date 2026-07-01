// Lazy mermaid-to-excalidraw rendering for the excalidraw code-block flip
// card. @excalidraw/excalidraw is a large React-based dependency (and
// @excalidraw/mermaid-to-excalidraw carries its own mermaid), so both are
// DYNAMIC-IMPORTED on first use only: the initial editor bundle never pulls
// them until an excalidraw block is actually flipped to its diagram. Theme
// is fed from the editor surface's light/dark so diagrams match the page.
//
// The render is headless: parseMermaidToExcalidraw turns a mermaid
// definition into an excalidraw element SKELETON, convertToExcalidrawElements
// fleshes it out, and exportToSvg serializes it to a static SVG. No
// <Excalidraw> React editor is ever mounted; the widget injects the SVG
// exactly as it does for a mermaid render.

import { type DiagramResult, parseErrorPos } from "./diagram_render";
import { renderMermaid } from "./mermaid_render";

// The two libraries load together on first render and memoize. `exportToSvg`
// is imported for its type so the elements cast below reads off its real
// signature rather than a hand-maintained shape.
type ExcalidrawModules = {
  parseMermaidToExcalidraw: typeof import("@excalidraw/mermaid-to-excalidraw").parseMermaidToExcalidraw;
  convertToExcalidrawElements: typeof import("@excalidraw/excalidraw").convertToExcalidrawElements;
  exportToSvg: typeof import("@excalidraw/excalidraw").exportToSvg;
};

let loader: Promise<ExcalidrawModules> | null = null;

async function loadExcalidraw(): Promise<ExcalidrawModules> {
  if (!loader) {
    loader = Promise.all([
      import("@excalidraw/mermaid-to-excalidraw"),
      import("@excalidraw/excalidraw"),
    ]).then(([m2e, excalidraw]) => ({
      parseMermaidToExcalidraw: m2e.parseMermaidToExcalidraw,
      convertToExcalidrawElements: excalidraw.convertToExcalidrawElements,
      exportToSvg: excalidraw.exportToSvg,
    }));
  }
  return loader;
}

/// Render a mermaid definition to an excalidraw SVG string. When the
/// excalidraw conversion fails on a source the plain mermaid renderer can
/// still draw, it degrades to that renderer so the block shows a diagram
/// instead of an error (WebKit/WKWebView cannot convert a flowchart with a
/// `subgraph`; see the catch below). A genuine parse failure resolves to
/// { ok:false, error } rather than throwing, so the caller can show the
/// message on the card's back face. mermaid-to-excalidraw parses mermaid
/// underneath, so its errors carry the same "line N" format the mermaid
/// renderer surfaces.
export async function renderExcalidraw(
  source: string,
  dark: boolean,
): Promise<DiagramResult> {
  try {
    const { parseMermaidToExcalidraw, convertToExcalidrawElements, exportToSvg } =
      await loadExcalidraw();
    const { elements: skeleton, files } = await parseMermaidToExcalidraw(source.trim());
    // convertToExcalidrawElements returns OrderedExcalidrawElement[]; exportToSvg
    // wants NonDeleted<ExcalidrawElement>[]. The freshly converted elements are
    // never deleted, so cast off exportToSvg's own parameter type rather than
    // pulling in excalidraw's internal element types.
    const elements = convertToExcalidrawElements(skeleton);
    const svg = await exportToSvg({
      elements: elements as Parameters<typeof exportToSvg>[0]["elements"],
      // Transparent background so the diagram sits on the editor surface;
      // exportWithDarkMode flips excalidraw's palette so strokes and text read
      // on a dark page (light-editor renders keep the default palette).
      files: files ?? null,
      appState: { exportBackground: false, exportWithDarkMode: dark },
      exportPadding: 10,
    });
    return { ok: true, svg: svg.outerHTML };
  } catch (err) {
    // mermaid-to-excalidraw's cluster lookup throws "SubGraph element not
    // found" in WebKit/WKWebView (chan-desktop): a flowchart with a
    // `subgraph` converts in Blink but its cluster elements are not found
    // here. Degrade to the plain mermaid renderer so the block still shows
    // the diagram; only when mermaid ALSO fails (genuinely bad source) do we
    // surface the excalidraw error, so the failing source line is accented.
    const fallback = await renderMermaid(source, dark);
    if (fallback.ok) return fallback;
    const error = (err as Error)?.message ?? String(err);
    const { line, col } = parseErrorPos(source, error);
    return { ok: false, error, errorLine: line, errorCol: col };
  }
}
