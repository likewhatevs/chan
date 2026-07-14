// Self-contained page snapshots for the PDF export engine. A page
// element is made fully self-contained (every image, font, and url()
// reference inlined as a data: URI), audited so nothing external
// survives, then rasterized <svg><foreignObject> -> canvas -> PNG.
//
// The audit is load-bearing, not defensive polish: an SVG-image
// document loads NO external resources (same-origin included), so a
// missed reference rasterizes as a blank region or a broken-image
// glyph. Failing the export with a named offender beats shipping a
// silently incomplete PDF.

/// Default per-step timeout. Every await in the snapshot pipeline is
/// bounded so a wedged fetch or decode degrades to an error, never a
/// hang.
const DEFAULT_STEP_TIMEOUT_MS = 15_000;

/// Raster scale: CSS px -> device px. 2x keeps text legible in the
/// rasterized PDF at normal zoom.
export const RASTER_SCALE = 2;

/// Largest canvas we will allocate, mirroring diagram_copy.ts's bound:
/// a pathological page must not allocate an unbounded width*height*4
/// buffer.
const MAX_PAGE_PIXELS = 64 * 1024 * 1024;

export type PageBoxPx = { widthPx: number; heightPx: number };

export class SnapshotError extends Error {}

function withTimeout<T>(
  work: Promise<T>,
  ms: number,
  what: string,
): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new SnapshotError(`${what} timed out after ${ms}ms`)),
      ms,
    );
    work.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (err) => {
        clearTimeout(timer);
        reject(err);
      },
    );
  });
}

/// Read a Blob as a base64 data: URL (the copy_html.ts pattern:
/// FileReader preserves the source bytes verbatim).
function blobToDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const fr = new FileReader();
    fr.onload = () => resolve(fr.result as string);
    fr.onerror = () => reject(fr.error ?? new Error("readAsDataURL failed"));
    fr.readAsDataURL(blob);
  });
}

/// Fetch a same-origin resource and return it as a data: URL, bounded
/// by `timeoutMs`. Returns null on any failure; the audit names the
/// leftover.
async function fetchAsDataUrl(
  url: string,
  timeoutMs: number,
): Promise<string | null> {
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      const resp = await fetch(url, { signal: controller.signal });
      if (!resp.ok) return null;
      const blob = await resp.blob();
      return await withTimeout(blobToDataUrl(blob), timeoutMs, `encode ${url}`);
    } finally {
      clearTimeout(timer);
    }
  } catch {
    return null;
  }
}

const URL_TOKEN_RE = /url\(\s*(['"]?)([^'")]+)\1\s*\)/g;

function isInlineUrl(value: string): boolean {
  return value.startsWith("data:") || value.startsWith("#");
}

/// Rewrite every non-data: url(...) token in a CSS text by fetching and
/// inlining it. Unresolvable tokens stay verbatim for the audit.
async function inlineCssUrls(css: string, timeoutMs: number): Promise<string> {
  const targets = new Map<string, string | null>();
  for (const match of css.matchAll(URL_TOKEN_RE)) {
    const url = match[2]!;
    if (!isInlineUrl(url) && !targets.has(url)) targets.set(url, null);
  }
  for (const url of targets.keys()) {
    targets.set(url, await fetchAsDataUrl(url, timeoutMs));
  }
  return css.replace(URL_TOKEN_RE, (token, _quote, url: string) => {
    const inlined = targets.get(url);
    return inlined ? `url(${inlined})` : token;
  });
}

const FONT_FACE_BLOCK_RE = /@font-face\s*\{[^}]*\}/g;
const FONT_FAMILY_DESC_RE = /font-family\s*:\s*(['"]?)([^'";}]+)\1/i;

/// Raw CSS text of a document stylesheet. A <style> owner node carries
/// the authored text (which keeps `src` descriptors that some CSSOM
/// serializers drop); link-loaded sheets fall back to the browser's
/// cssText, which is complete in every shipping engine.
function styleSheetText(sheet: CSSStyleSheet): string {
  if (sheet.ownerNode instanceof HTMLStyleElement) {
    return sheet.ownerNode.textContent ?? "";
  }
  try {
    return Array.from(sheet.cssRules)
      .map((rule) => rule.cssText)
      .join("\n");
  } catch {
    return ""; // cross-origin sheet; the app bundles none.
  }
}

/// Collect the app's @font-face rules whose family the page references,
/// inline their src urls, and prepend them as a <style> on the page.
/// The page's own <style> elements (e.g. the ones excalidraw exports
/// carry inside their SVG) are inlined in place.
async function inlineFonts(root: HTMLElement, timeoutMs: number): Promise<void> {
  // Page-embedded styles first: excalidraw SVG exports declare their
  // fonts in an inner <style> with /static/excalidraw/ urls.
  for (const style of Array.from(root.querySelectorAll("style"))) {
    const css = style.textContent ?? "";
    URL_TOKEN_RE.lastIndex = 0;
    if (URL_TOKEN_RE.test(css)) {
      URL_TOKEN_RE.lastIndex = 0;
      style.textContent = await inlineCssUrls(css, timeoutMs);
    }
    URL_TOKEN_RE.lastIndex = 0;
  }

  // App-level @font-face rules (fonts.css: the bundled code font). Only
  // families the page actually references ride along.
  const pageHtml = root.outerHTML.toLowerCase();
  const faces: string[] = [];
  for (const sheet of Array.from(document.styleSheets)) {
    for (const block of styleSheetText(sheet).match(FONT_FACE_BLOCK_RE) ?? []) {
      const family = block.match(FONT_FAMILY_DESC_RE)?.[2]?.trim();
      if (!family || !pageHtml.includes(family.toLowerCase())) continue;
      faces.push(block);
    }
  }
  if (faces.length === 0) return;
  const style = document.createElement("style");
  style.textContent = await inlineCssUrls(faces.join("\n"), timeoutMs);
  root.prepend(style);
}

/// Inline every <img> src and every SVG <image> href under the page.
async function inlineImages(root: HTMLElement, timeoutMs: number): Promise<void> {
  for (const img of Array.from(root.querySelectorAll("img"))) {
    const src = img.getAttribute("src") ?? "";
    if (!src || isInlineUrl(src)) continue;
    const inlined = await fetchAsDataUrl(src, timeoutMs);
    if (inlined) img.setAttribute("src", inlined);
  }
  for (const image of Array.from(root.querySelectorAll("image"))) {
    for (const attr of ["href", "xlink:href"]) {
      const href = image.getAttribute(attr);
      if (!href || isInlineUrl(href)) continue;
      const inlined = await fetchAsDataUrl(href, timeoutMs);
      if (inlined) image.setAttribute(attr, inlined);
    }
  }
}

/// Make the page self-contained: images, SVG image hrefs, url() tokens
/// in embedded styles and style attributes, and the app font faces the
/// page references. Unresolvable references are left in place for
/// `auditSelfContained` to reject by name.
export async function inlinePageResources(
  root: HTMLElement,
  timeoutMs: number = DEFAULT_STEP_TIMEOUT_MS,
): Promise<void> {
  await inlineFonts(root, timeoutMs);
  await inlineImages(root, timeoutMs);
  for (const el of Array.from(root.querySelectorAll<HTMLElement>("[style]"))) {
    const css = el.getAttribute("style") ?? "";
    URL_TOKEN_RE.lastIndex = 0;
    if (URL_TOKEN_RE.test(css)) {
      URL_TOKEN_RE.lastIndex = 0;
      el.setAttribute("style", await inlineCssUrls(css, timeoutMs));
    }
  }
}

function externalUrlTokens(css: string): string[] {
  const out: string[] = [];
  URL_TOKEN_RE.lastIndex = 0;
  for (const match of css.matchAll(URL_TOKEN_RE)) {
    const url = match[2]!;
    if (!isInlineUrl(url)) out.push(url);
  }
  return out;
}

/// Reject any external reference left on the page. Anchor hrefs are
/// page CONTENT (never fetched during raster) and pass; everything an
/// SVG-image document would try to LOAD must be a data: URI by now.
/// Throws a SnapshotError naming every offender.
export function auditSelfContained(root: HTMLElement): void {
  const offenders: string[] = [];

  for (const img of Array.from(root.querySelectorAll("img"))) {
    const src = img.getAttribute("src") ?? "";
    if (src && !isInlineUrl(src)) offenders.push(`img src ${src}`);
  }
  for (const image of Array.from(root.querySelectorAll("image, use"))) {
    for (const attr of ["href", "xlink:href"]) {
      const href = image.getAttribute(attr);
      if (href && !isInlineUrl(href)) {
        offenders.push(`${image.tagName.toLowerCase()} ${attr} ${href}`);
      }
    }
  }
  for (const el of Array.from(
    root.querySelectorAll("script, iframe, embed, object, video, audio, source, link"),
  )) {
    offenders.push(`disallowed element <${el.tagName.toLowerCase()}>`);
  }
  for (const style of Array.from(root.querySelectorAll("style"))) {
    for (const url of externalUrlTokens(style.textContent ?? "")) {
      offenders.push(`style url() ${url}`);
    }
  }
  for (const el of Array.from(root.querySelectorAll<HTMLElement>("[style]"))) {
    for (const url of externalUrlTokens(el.getAttribute("style") ?? "")) {
      offenders.push(`inline style url() ${url}`);
    }
  }

  if (offenders.length > 0) {
    throw new SnapshotError(
      `page is not self-contained: ${offenders.join("; ")}`,
    );
  }
}

/// Serialize the page element into an <svg><foreignObject> document.
/// XMLSerializer emits well-formed XHTML with the namespace on the
/// root, which is what the foreignObject content model requires.
export function pageSvgDocument(root: HTMLElement, box: PageBoxPx): string {
  const xhtml = new XMLSerializer().serializeToString(root);
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${box.widthPx}" height="${box.heightPx}">` +
    `<foreignObject width="100%" height="100%">${xhtml}</foreignObject></svg>`
  );
}

/// Decode an SVG document into an image, bounded by `timeoutMs`.
function loadSvgPageImage(
  svgDoc: string,
  timeoutMs: number,
): Promise<HTMLImageElement> {
  const img = new Image();
  const loaded = new Promise<HTMLImageElement>((resolve, reject) => {
    img.onload = () => resolve(img);
    img.onerror = () => reject(new SnapshotError("page SVG decode failed"));
  });
  img.src = `data:image/svg+xml;utf8,${encodeURIComponent(svgDoc)}`;
  return withTimeout(loaded, timeoutMs, "page SVG decode");
}

/// Rasterize a self-contained page element to a canvas at
/// `scale` device px per CSS px. The caller runs the audit first;
/// this step only draws.
export async function rasterizePage(
  root: HTMLElement,
  box: PageBoxPx,
  opts: { scale?: number; timeoutMs?: number } = {},
): Promise<HTMLCanvasElement> {
  const scale = opts.scale ?? RASTER_SCALE;
  const timeoutMs = opts.timeoutMs ?? DEFAULT_STEP_TIMEOUT_MS;
  const width = Math.ceil(box.widthPx * scale);
  const height = Math.ceil(box.heightPx * scale);
  if (width * height > MAX_PAGE_PIXELS) {
    throw new SnapshotError(
      `page raster ${width}x${height} exceeds the pixel budget`,
    );
  }
  const img = await loadSvgPageImage(pageSvgDocument(root, box), timeoutMs);
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new SnapshotError("no 2d canvas context");
  ctx.drawImage(img, 0, 0, width, height);
  return canvas;
}

/// Canvas -> PNG bytes.
export async function canvasPngBytes(
  canvas: HTMLCanvasElement,
  timeoutMs: number = DEFAULT_STEP_TIMEOUT_MS,
): Promise<Uint8Array> {
  const blob = await withTimeout(
    new Promise<Blob>((resolve, reject) => {
      canvas.toBlob(
        (b) => (b ? resolve(b) : reject(new SnapshotError("toBlob failed"))),
        "image/png",
      );
    }),
    timeoutMs,
    "PNG encode",
  );
  return new Uint8Array(await blob.arrayBuffer());
}

export type PageSnapshot = {
  png: Uint8Array;
  /// Raster size in device px (CSS px * scale).
  widthPx: number;
  heightPx: number;
};

/// The full snapshot pipeline for one page element: inline -> audit ->
/// raster -> PNG. The element must be attached (layout done) before
/// this runs.
export async function snapshotPage(
  root: HTMLElement,
  box: PageBoxPx,
  opts: { scale?: number; timeoutMs?: number } = {},
): Promise<PageSnapshot> {
  await inlinePageResources(root, opts.timeoutMs);
  auditSelfContained(root);
  const canvas = await rasterizePage(root, box, opts);
  return {
    png: await canvasPngBytes(canvas, opts.timeoutMs),
    widthPx: canvas.width,
    heightPx: canvas.height,
  };
}
