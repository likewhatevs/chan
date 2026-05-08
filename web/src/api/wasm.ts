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

type WikiLinkToMarkdown = (
  target: string,
  label?: string,
  anchor?: string,
) => string;

let extractLinksImpl: ExtractLinks | null = null;
/// Wasm version of wikiLinkToMarkdown is the legacy 3-arg form
/// (target / label / anchor) and always emits a drive-rooted URL.
/// We keep it for the no-fromPath path; calls that pass fromPath
/// always go through `fallbackWikiLinkToMarkdown` which computes
/// the file-relative URL in pure TS.
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

/// Serialize a wikiLink atom's attrs back to markdown.
///
/// `fromPath` is the path of the file whose markdown is being
/// produced (drive-rooted POSIX, no leading slash). When provided,
/// the URL portion is rewritten to a file-relative path with an
/// explicit `./` or `../` prefix so the discriminator at parse
/// time can tell relative URLs from legacy drive-rooted ones.
/// When omitted (e.g. assistant prompt context, no source file),
/// the URL stays drive-rooted.
export function wikiLinkToMarkdown(
  target: string,
  label?: string,
  anchor?: string,
  fromPath?: string,
) {
  if (fromPath) {
    // The wasm impl is the legacy 3-arg form; relative-path
    // serialization lives entirely in the TS fallback so the
    // wasm boundary stays untouched.
    return fallbackWikiLinkToMarkdown(target, label, anchor, fromPath);
  }
  return (wikiLinkToMarkdownImpl ?? fallbackWikiLinkToMarkdown)(
    target,
    label,
    anchor,
  );
}

/// Compute a file-relative path from `fromPath`'s directory to
/// `target`, both drive-rooted POSIX paths. Always emits a
/// `./` or `../` prefix so the parser can distinguish a relative
/// URL from a legacy drive-rooted one.
///
/// Examples (fromPath -> target -> result):
///   `Recipes/Pasta.md`    -> `Recipes/Brazilian Rice.md` -> `./Brazilian Rice.md`
///   `Recipes/Pasta.md`    -> `Notes/Foo.md`              -> `../Notes/Foo.md`
///   `README.md`           -> `Recipes/Pasta.md`          -> `./Recipes/Pasta.md`
export function relativizePath(target: string, fromPath: string): string {
  const fromDir = fromPath.split("/").slice(0, -1);
  const tgtParts = target.split("/");
  let i = 0;
  while (
    i < fromDir.length &&
    i < tgtParts.length - 1 &&
    fromDir[i] === tgtParts[i]
  ) {
    i += 1;
  }
  const ups = fromDir.length - i;
  const down = tgtParts.slice(i);
  if (ups === 0) {
    return ["."].concat(down).join("/");
  }
  return Array(ups).fill("..").concat(down).join("/");
}

/// Resolve a relative href against `fromPath`'s directory, returning
/// the canonical drive-rooted target. Hrefs that don't start with
/// `./` or `../` are treated as already-drive-rooted (legacy /
/// power-user form) and returned unchanged.
export function resolveRelativePath(href: string, fromPath: string): string {
  if (!href.startsWith("./") && !href.startsWith("../")) {
    return href;
  }
  const fromDir = fromPath.split("/").slice(0, -1);
  const parts = href.split("/");
  for (const p of parts) {
    if (p === "" || p === ".") continue;
    if (p === "..") {
      if (fromDir.length > 0) fromDir.pop();
    } else {
      fromDir.push(p);
    }
  }
  return fromDir.join("/");
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

function fallbackWikiLinkToMarkdown(
  target: string,
  label?: string,
  anchor?: string,
  fromPath?: string,
): string {
  const stem = (label ?? target.split("/").pop() ?? target).replace(/\.md$/, "");
  // Build the URL portion. With `fromPath` set, the URL is
  // rewritten to a file-relative path so notes stay portable
  // across project layouts (an editor opening a single file
  // outside the drive can still resolve the link). Without
  // `fromPath`, fall back to the legacy drive-rooted form so
  // the assistant prompt and other no-source-file callers keep
  // their existing semantics.
  const path = fromPath ? relativizePath(target, fromPath) : target;
  const enc = path
    .split("/")
    .map((s) => encodeURIComponent(s).replace(/%2F/g, "/"))
    .join("/");
  // Anchor is appended verbatim. Heading anchors are already
  // slugged by chan-core (kebab-case ASCII); block anchors are
  // `^id` and round-trip cleanly through encodeURIComponent's
  // identity for `^`.
  const frag = anchor ? `#${anchor}` : "";
  return `[${stem}](${enc}${frag})`;
}
