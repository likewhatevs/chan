import { renderMarkdown } from "../api/markdown";
import {
  isMacDesktop,
  isTauriDesktop,
  saveBytesToDownloads,
  tauriInvoke,
} from "../api/desktop";
import { notify } from "../state/notify.svelte";
import { parseImageSrc, resolveImageSrc } from "./extensions/image";

const PRINT_FRAME_ID = "chan-print-frame";

/// Offscreen page width handed to the native macOS PDF export, in CSS
/// pixels. US Letter (8.5in) at 96dpi. The print document narrows its
/// content to a percentage of this via `pageWidthCss`, so a fixed
/// letter-equivalent page makes the captured PDF match what the browser's
/// print-to-PDF would produce, regardless of the page-width ratio.
const NATIVE_PAGE_WIDTH_PX = 816;

const EDITOR_VARS = [
  "--chan-editor-body-family",
  "--chan-editor-body-size",
  "--chan-editor-body-color",
  "--chan-editor-bg",
  "--chan-editor-heading-family",
  "--chan-editor-heading-color",
  "--chan-editor-h1-size",
  "--chan-editor-h1-weight",
  "--chan-editor-h1-line-height",
  "--chan-editor-h1-border-bottom",
  "--chan-editor-h1-padding-bottom",
  "--chan-editor-h2-size",
  "--chan-editor-h2-weight",
  "--chan-editor-h2-line-height",
  "--chan-editor-h2-border-bottom",
  "--chan-editor-h2-padding-bottom",
  "--chan-editor-h3-size",
  "--chan-editor-h3-weight",
  "--chan-editor-h3-line-height",
  "--chan-editor-h4-size",
  "--chan-editor-h4-weight",
  "--chan-editor-h4-line-height",
  "--chan-editor-h5-size",
  "--chan-editor-h5-weight",
  "--chan-editor-h5-line-height",
  "--chan-editor-h6-size",
  "--chan-editor-h6-weight",
  "--chan-editor-h6-line-height",
  "--chan-editor-h6-color",
  "--chan-editor-code-family",
  "--chan-editor-code-size",
  "--chan-editor-inline-code-bg",
  "--chan-editor-inline-code-color",
  "--chan-editor-code-block-bg",
  "--chan-editor-code-block-color",
  "--chan-editor-code-block-border",
  "--chan-editor-link-color",
  "--chan-editor-quote-color",
  "--chan-editor-quote-border",
  "--chan-editor-hr-color",
  "--chan-editor-table-border",
  "--chan-editor-table-header-bg",
  "--chan-editor-table-stripe-bg",
] as const;

export type PrintMarkdownOptions = {
  title: string;
  path: string;
  markdown: string;
  pageWidthRatio: number;
  styleSource?: Element | null;
};

function escapeHtml(s: string): string {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

export function renderPrintableMarkdown(markdown: string, fromPath: string): string {
  const wrap = document.createElement("div");
  wrap.innerHTML = renderMarkdown(markdown);
  preparePrintableImages(wrap, fromPath);
  return wrap.innerHTML;
}

function preparePrintableImages(root: ParentNode, fromPath: string): void {
  for (const img of Array.from(root.querySelectorAll("img"))) {
    const raw = img.getAttribute("src") ?? "";
    const { width, align } = parseImageSrc(raw);
    const resolved = resolveImageSrc(raw, fromPath);
    if (resolved) img.setAttribute("src", resolved);
    if (width != null) {
      img.style.maxWidth = "100%";
      img.style.width = `${width}px`;
    }
    if (align) {
      img.classList.add(`chan-print-img-${align}`);
    }
  }
}

function cssVarsFrom(source: Element | null | undefined): string {
  const style = getComputedStyle(source ?? document.documentElement);
  const vars: string[] = [];
  for (const name of EDITOR_VARS) {
    const value = style.getPropertyValue(name).trim();
    if (value) vars.push(`${name}: ${value};`);
  }
  return vars.length ? `:root {\n${vars.join("\n")}\n}` : "";
}

function pageWidthCss(ratio: number): string {
  if (!Number.isFinite(ratio) || ratio >= 1) return "none";
  const pct = Math.max(35, Math.min(100, Math.round(ratio * 100)));
  return `${pct}%`;
}

export function buildPrintDocumentHtml(opts: PrintMarkdownOptions): string {
  const body = renderPrintableMarkdown(opts.markdown, opts.path);
  const title = escapeHtml(opts.title || opts.path || "chan document");
  const maxWidth = pageWidthCss(opts.pageWidthRatio);
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>${title}</title>
<style>
${cssVarsFrom(opts.styleSource)}
html {
  background: var(--chan-editor-bg, #ffffff);
}
body {
  margin: 0;
  background: var(--chan-editor-bg, #ffffff);
  color: var(--chan-editor-body-color, #1f2328);
  font-family: var(--chan-editor-body-family, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif);
  font-size: var(--chan-editor-body-size, 16px);
  line-height: 1.65;
}
.chan-print-page {
  box-sizing: border-box;
  max-width: ${maxWidth};
  margin: 0 auto;
  padding: 0.7in;
}
h1, h2, h3, h4, h5, h6 {
  color: var(--chan-editor-heading-color, currentColor);
  font-family: var(--chan-editor-heading-family, var(--chan-editor-body-family));
  break-after: avoid;
  page-break-after: avoid;
}
h1 {
  font-size: var(--chan-editor-h1-size, 2em);
  font-weight: var(--chan-editor-h1-weight, 700);
  line-height: var(--chan-editor-h1-line-height, 1.25);
  border-bottom: var(--chan-editor-h1-border-bottom, none);
  padding-bottom: var(--chan-editor-h1-padding-bottom, 0);
}
h2 {
  font-size: var(--chan-editor-h2-size, 1.6em);
  font-weight: var(--chan-editor-h2-weight, 700);
  line-height: var(--chan-editor-h2-line-height, 1.3);
  border-bottom: var(--chan-editor-h2-border-bottom, none);
  padding-bottom: var(--chan-editor-h2-padding-bottom, 0);
}
h3 {
  font-size: var(--chan-editor-h3-size, 1.3em);
  font-weight: var(--chan-editor-h3-weight, 600);
  line-height: var(--chan-editor-h3-line-height, 1.35);
}
h4 {
  font-size: var(--chan-editor-h4-size, 1.15em);
  font-weight: var(--chan-editor-h4-weight, 600);
  line-height: var(--chan-editor-h4-line-height, 1.4);
}
h5 {
  font-size: var(--chan-editor-h5-size, 1em);
  font-weight: var(--chan-editor-h5-weight, 600);
  line-height: var(--chan-editor-h5-line-height, 1.4);
}
h6 {
  color: var(--chan-editor-h6-color, currentColor);
  font-size: var(--chan-editor-h6-size, 0.95em);
  font-weight: var(--chan-editor-h6-weight, 600);
  line-height: var(--chan-editor-h6-line-height, 1.4);
}
a {
  color: var(--chan-editor-link-color, #0969da);
}
blockquote {
  border-left: 3px solid var(--chan-editor-quote-border, #d0d7de);
  color: var(--chan-editor-quote-color, currentColor);
  margin-left: 0;
  padding-left: 1em;
}
code {
  background: var(--chan-editor-inline-code-bg, rgba(175, 184, 193, 0.2));
  color: var(--chan-editor-inline-code-color, currentColor);
  font-family: var(--chan-editor-code-family, ui-monospace, SFMono-Regular, Menlo, monospace);
  font-size: var(--chan-editor-code-size, 0.92em);
  padding: 0.12em 0.28em;
  border-radius: 4px;
}
pre {
  background: var(--chan-editor-code-block-bg, #f6f8fa);
  border: 1px solid var(--chan-editor-code-block-border, transparent);
  color: var(--chan-editor-code-block-color, currentColor);
  overflow-wrap: anywhere;
  padding: 12px;
  white-space: pre-wrap;
}
pre code {
  background: transparent;
  border-radius: 0;
  padding: 0;
}
hr {
  border: 0;
  border-top: 1px solid var(--chan-editor-hr-color, #d0d7de);
}
table {
  border-collapse: collapse;
  width: 100%;
}
th, td {
  border: 1px solid var(--chan-editor-table-border, #d0d7de);
  padding: 6px 8px;
}
th {
  background: var(--chan-editor-table-header-bg, #f6f8fa);
}
tr:nth-child(even) td {
  background: var(--chan-editor-table-stripe-bg, transparent);
}
img {
  display: block;
  height: auto;
  max-width: 100%;
  margin: 1em auto;
}
.chan-print-img-left {
  margin-left: 0;
  margin-right: auto;
}
.chan-print-img-right {
  margin-left: auto;
  margin-right: 0;
}
.chan-page-break {
  border: 0;
  margin: 0;
  break-after: page;
  page-break-after: always;
}
@page {
  margin: 0.65in;
}
@media print {
  .chan-print-page {
    padding: 0;
  }
}
</style>
</head>
<body>
<main class="chan-print-page">
${body}
</main>
</body>
</html>`;
}

function waitForFrameLoad(frame: HTMLIFrameElement): Promise<void> {
  return new Promise((resolve) => {
    frame.addEventListener("load", () => resolve(), { once: true });
  });
}

async function waitForPrintableAssets(doc: Document): Promise<void> {
  const imagePromises = Array.from(doc.images)
    .filter((img) => !img.complete)
    .map(
      (img) =>
        new Promise<void>((resolve) => {
          img.addEventListener("load", () => resolve(), { once: true });
          img.addEventListener("error", () => resolve(), { once: true });
        }),
    );
  const fontsReady = "fonts" in doc ? doc.fonts.ready.catch(() => undefined) : undefined;
  await Promise.race([
    Promise.all([...imagePromises, fontsReady].filter(Boolean)),
    new Promise((resolve) => setTimeout(resolve, 3000)),
  ]);
}

/// Derive the saved PDF filename from the document path: the basename
/// with its extension swapped for `.pdf` (or `.pdf` appended when the
/// basename has no extension). Empty paths fall back to a generic name.
function pdfFilenameFor(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  const base = i < 0 ? path : path.slice(i + 1);
  const trimmed = base.trim();
  if (!trimmed) return "document.pdf";
  const dot = trimmed.lastIndexOf(".");
  const stem = dot > 0 ? trimmed.slice(0, dot) : trimmed;
  return `${stem}.pdf`;
}

/// Native macOS PDF export. The browser's `window.print()` is a no-op
/// inside Tauri's WKWebView, so on macOS desktop we render the same
/// themed print HTML to a vector PDF via the `export_pdf_macos` command
/// (WKWebView `createPDF`) and save it to Downloads through the existing
/// `save_file_to_downloads` path. Throws on IPC / save failure so the
/// caller can surface a notice.
async function exportPdfNative(opts: PrintMarkdownOptions): Promise<void> {
  const html = buildPrintDocumentHtml(opts);
  const bytes = await tauriInvoke<number[]>("export_pdf_macos", {
    html,
    pageWidthPx: NATIVE_PAGE_WIDTH_PX,
  });
  await saveBytesToDownloads(Uint8Array.from(bytes), pdfFilenameFor(opts.path));
}

export async function printMarkdownDocument(
  opts: PrintMarkdownOptions,
): Promise<void> {
  // Desktop branch: `window.print()` does nothing in WKWebView. macOS
  // gets the native vector PDF path; other desktop OSes have no native
  // export wired and hide the trigger button, so reaching here off-macOS
  // is a defensive no-op with a notice rather than a silent failure.
  if (isTauriDesktop()) {
    if (await isMacDesktop()) {
      await exportPdfNative(opts);
    } else {
      notify("Export to PDF is not available on this platform yet.");
    }
    return;
  }

  document.getElementById(PRINT_FRAME_ID)?.remove();
  const frame = document.createElement("iframe");
  frame.id = PRINT_FRAME_ID;
  frame.title = "PDF export";
  frame.style.position = "fixed";
  frame.style.left = "-10000px";
  frame.style.top = "0";
  frame.style.width = "1024px";
  frame.style.height = "768px";
  frame.style.border = "0";
  frame.style.opacity = "0";
  frame.style.pointerEvents = "none";
  document.body.append(frame);

  const loaded = waitForFrameLoad(frame);
  frame.srcdoc = buildPrintDocumentHtml(opts);
  await loaded;

  const win = frame.contentWindow;
  const doc = frame.contentDocument;
  if (!win || !doc) throw new Error("print frame did not load");

  await waitForPrintableAssets(doc);
  win.addEventListener("afterprint", () => frame.remove(), { once: true });
  window.setTimeout(() => frame.remove(), 60_000);
  win.focus();
  win.print();
}
