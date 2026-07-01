// Shared contract for the editor's diagram renderers (mermaid,
// mermaid-to-excalidraw). A renderer turns a fenced block's source plus the
// editor surface's light/dark theme into an SVG string, or a structured
// parse error. Renderers own no DOM: the diagram widget injects the SVG.

/// A diagram render result: an SVG string on success, or an error message
/// (plus the 1-indexed line/column in the ORIGINAL source when the parser
/// reported them) on failure. A parse/render failure resolves to
/// { ok:false, error } rather than throwing, so the widget can show the
/// message on the diagram face and accent the failing source line.
export interface DiagramResult {
  ok: boolean;
  svg?: string;
  error?: string;
  errorLine?: number;
  errorCol?: number;
}

/// A renderer: fenced-block source + theme in, an SVG (or error) out. Both
/// mermaid and excalidraw follow this shape, so the diagram widget is
/// renderer-agnostic.
export type DiagramRenderer = (source: string, dark: boolean) => Promise<DiagramResult>;

/// Pull a "line N, column M" position out of a parser error message. Both
/// mermaid and mermaid-to-excalidraw (which parses mermaid underneath)
/// number lines against the string they parsed - `source.trim()` - so the
/// leading blank lines `.trim()` removed are added back to land on the
/// right line in the ORIGINAL source.
export function parseErrorPos(
  source: string,
  message: string,
): { line?: number; col?: number } {
  const m = /line (\d+)(?:,?\s*column (\d+))?/i.exec(message);
  if (!m) return {};
  const removedLeadingLines = (
    source.slice(0, source.length - source.trimStart().length).match(/\n/g) ?? []
  ).length;
  return {
    line: Number(m[1]) + removedLeadingLines,
    col: m[2] !== undefined ? Number(m[2]) : undefined,
  };
}
