// Optional wasm bridge to chan-shared.
//
// Calls into the same Rust functions the server uses for link extraction
// and wiki-link serialization. If wasm-pack hasn't been run yet (i.e.
// /web/pkg/ doesn't exist), we fall back to TS approximations so the
// editor still works during early development.

type ExtractLinks = (markdown: string) => Array<{
  target: string;
  label: string | null;
  wiki: boolean;
}>;

type WikiLinkToMarkdown = (target: string, label?: string) => string;

let extractLinksImpl: ExtractLinks | null = null;
let wikiLinkToMarkdownImpl: WikiLinkToMarkdown | null = null;

/// Try to load the wasm module. Safe to call repeatedly; only loads once.
///
/// We deliberately route the import through a string the bundler cannot
/// resolve statically. If `web/pkg/` is missing (no wasm-pack run yet),
/// the dynamic import throws at runtime and we fall back to TS. If
/// `web/pkg/` exists, the browser fetches and instantiates it lazily.
export async function loadShared(): Promise<void> {
  if (extractLinksImpl) return;
  try {
    const url = /* @vite-ignore */ new URL(
      "../../pkg/chan_shared.js",
      import.meta.url,
    ).href;
    const mod = (await import(/* @vite-ignore */ url)) as {
      default: () => Promise<unknown>;
      extractLinks: ExtractLinks;
      wikiLinkToMarkdown: WikiLinkToMarkdown;
    };
    await mod.default();
    extractLinksImpl = mod.extractLinks;
    wikiLinkToMarkdownImpl = mod.wikiLinkToMarkdown;
  } catch {
    // Fallback: simple TS implementations.
    extractLinksImpl = fallbackExtractLinks;
    wikiLinkToMarkdownImpl = fallbackWikiLinkToMarkdown;
  }
}

export function extractLinks(markdown: string) {
  return (extractLinksImpl ?? fallbackExtractLinks)(markdown);
}

export function wikiLinkToMarkdown(target: string, label?: string) {
  return (wikiLinkToMarkdownImpl ?? fallbackWikiLinkToMarkdown)(target, label);
}

// ----- TS fallbacks -------------------------------------------------------

function fallbackExtractLinks(markdown: string): ReturnType<ExtractLinks> {
  const out: ReturnType<ExtractLinks> = [];
  const wiki = /\[\[([^\]\n]+)\]\]/g;
  let m: RegExpExecArray | null;
  while ((m = wiki.exec(markdown))) {
    const inner = m[1] ?? "";
    const [t, l] = inner.includes("|") ? inner.split("|", 2) : [inner, null];
    out.push({ target: (t ?? "").trim(), label: l?.trim() ?? null, wiki: true });
  }
  const std = /\[([^\]]+)\]\(([^)]+)\)/g;
  while ((m = std.exec(markdown))) {
    out.push({ target: m[2] ?? "", label: m[1] ?? null, wiki: false });
  }
  return out;
}

function fallbackWikiLinkToMarkdown(target: string, label?: string): string {
  const stem = (label ?? target.split("/").pop() ?? target).replace(/\.md$/, "");
  const enc = target
    .split("/")
    .map((s) => encodeURIComponent(s).replace(/%2F/g, "/"))
    .join("/");
  return `[${stem}](${enc})`;
}
