// pdf-lib byte assertions for exported PDFs: page count, A4 dims, and
// per-page nonzero raster ink. The export engine embeds each page as
// one FlateDecode image XObject (raw pixels), so ink is measured by
// inflating the stream and counting pixels that differ from the page's
// corner pixel - no canvas or native decoder needed.

import { inflateSync } from "node:zlib";
import { PDFDocument, PDFName, PDFRawStream } from "pdf-lib";

const A4 = { short: 595.28, long: 841.89 };

function approx(a, b, eps = 0.5) {
  return Math.abs(a - b) <= eps;
}

/// Fraction of pixels differing from the top-left pixel by more than
/// `tol` on any channel. 0 for a blank page.
function inkRatio(raw, width, height, channels, tol = 8) {
  const total = width * height;
  if (total === 0 || raw.length < total * channels) return 0;
  let ink = 0;
  const base = [];
  for (let c = 0; c < channels; c++) base.push(raw[c]);
  for (let p = 0; p < total; p++) {
    for (let c = 0; c < channels; c++) {
      if (Math.abs(raw[p * channels + c] - base[c]) > tol) {
        ink++;
        break;
      }
    }
  }
  return ink / total;
}

function pageImageInk(page) {
  const resources = page.node.Resources();
  const xobjects = resources?.lookup(PDFName.of("XObject"));
  if (!xobjects) return { ratio: 0, images: 0 };
  let best = { ratio: 0, images: 0 };
  for (const key of xobjects.keys()) {
    const stream = xobjects.lookup(key);
    if (!(stream instanceof PDFRawStream)) continue;
    const dict = stream.dict;
    const width = dict.lookup(PDFName.of("Width"))?.asNumber?.() ?? 0;
    const height = dict.lookup(PDFName.of("Height"))?.asNumber?.() ?? 0;
    let raw;
    try {
      raw = inflateSync(Buffer.from(stream.contents));
    } catch {
      continue;
    }
    const channels = Math.max(1, Math.round(raw.length / (width * height)));
    const ratio = inkRatio(raw, width, height, channels);
    best = {
      ratio: Math.max(best.ratio, ratio),
      images: best.images + 1,
    };
  }
  return best;
}

/// Assert exported PDF bytes: `pages` (exact count), `orientation`
/// ("portrait" | "landscape", A4), and `minInkRatio` per page. Throws
/// with a precise message on the first violation; returns a summary.
export async function assertPdf(bytes, { pages, orientation, minInkRatio = 0.001 }) {
  const doc = await PDFDocument.load(bytes);
  if (doc.getPageCount() !== pages) {
    throw new Error(`expected ${pages} pages, got ${doc.getPageCount()}`);
  }
  const summary = [];
  for (const [i, page] of doc.getPages().entries()) {
    const w = page.getWidth();
    const h = page.getHeight();
    const want =
      orientation === "landscape"
        ? { w: A4.long, h: A4.short }
        : { w: A4.short, h: A4.long };
    if (!approx(w, want.w) || !approx(h, want.h)) {
      throw new Error(
        `page ${i + 1}: expected A4 ${orientation} (${want.w}x${want.h}pt), got ${w}x${h}pt`,
      );
    }
    const ink = pageImageInk(page);
    if (ink.images === 0) {
      throw new Error(`page ${i + 1}: no raster image content`);
    }
    if (ink.ratio < minInkRatio) {
      throw new Error(
        `page ${i + 1}: raster ink ratio ${ink.ratio.toFixed(5)} below ${minInkRatio}`,
      );
    }
    summary.push({ page: i + 1, widthPt: w, heightPt: h, inkRatio: ink.ratio });
  }
  return summary;
}
