// Lazy mermaid rendering for the mermaid code-block flip card. mermaid
// is a large dependency, so it is DYNAMIC-IMPORTED on first use only:
// the initial editor bundle never pulls it until a mermaid block is
// actually flipped to its diagram. Theme is fed from the editor
// surface's light/dark so diagrams match the page.

type MermaidApi = typeof import("mermaid").default;

let loader: Promise<MermaidApi> | null = null;
let seq = 0;

async function loadMermaid(): Promise<MermaidApi> {
  if (!loader) {
    loader = import("mermaid").then((m) => m.default);
  }
  return loader;
}

export interface MermaidResult {
  ok: boolean;
  svg?: string;
  error?: string;
  // 1-indexed line/column of the parse error WITHIN the original
  // (untrimmed) source, when mermaid's message carries them. Used to
  // point the editor at the failing line.
  errorLine?: number;
  errorCol?: number;
}

/// Mermaid parse errors carry "line N, column M" (Lexer/Parse errors).
/// Pull them out so the caller can locate the failure. mermaid numbers
/// lines against the string it parsed - `source.trim()` below - so the
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

/// Render mermaid source to an SVG string. A parse/render failure (bad
/// diagram source) resolves to { ok:false, error } rather than throwing,
/// so the caller can show the message on the card's back face.
export async function renderMermaid(
  source: string,
  dark: boolean,
): Promise<MermaidResult> {
  try {
    const mermaid = await loadMermaid();
    // Theme is a global init option; set it per render so a surface
    // theme flip is honoured on the next render. securityLevel "strict"
    // keeps mermaid's own sanitizer on.
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      theme: dark ? "dark" : "default",
    });
    const id = `chan-mermaid-${seq++}`;
    const { svg } = await mermaid.render(id, source.trim());
    return { ok: true, svg };
  } catch (err) {
    const error = (err as Error)?.message ?? String(err);
    const { line, col } = parseErrorPos(source, error);
    return { ok: false, error, errorLine: line, errorCol: col };
  }
}
