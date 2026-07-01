/// Clipboard bridge for `cs copy` / `cs paste`.
///
/// The `cs` client base64-encodes stdin (copy) or base64-decodes the reply
/// (paste); the raw bytes cross the control socket and `/ws` as base64 inside
/// JSON. This module turns those bytes into the right clipboard representation
/// on write, and reads the clipboard back on paste, branching browser
/// `navigator.clipboard` vs the desktop's native arboard IPC exactly like
/// `readClipboardText` / `writeClipboardText`.

import {
  isTauriDesktop,
  readClipboardHtml,
  readClipboardImage,
  readClipboardText,
  writeClipboardHtml,
  writeClipboardImage,
  writeClipboardText,
} from "./desktop";

/// Which representation `cs paste` wants. `auto` is image-first then plain
/// text; the others force one. Mirrors the Rust `PastePrefer`.
export type PastePrefer = "auto" | "text" | "html" | "image";

/// One clipboard representation: a MIME plus the raw bytes.
export type ClipboardPayload = { mime: string; bytes: Uint8Array };

const PLAIN_MIME = "text/plain;charset=utf-8";

/// Largest raw clipboard payload we carry, mirroring the Rust
/// `MAX_CLIPBOARD_BYTES` (chan-shell `wire.rs`). A paste larger than this is
/// refused before we build a giant base64 string that the reply route would
/// reject anyway.
const MAX_CLIPBOARD_BYTES = 32 * 1024 * 1024;

/// Largest decoded image we will rasterize to a canvas on copy. A tiny image
/// can declare enormous dimensions (a decompression bomb); reject those before
/// allocating a `width*height*4` canvas. 64 MPx (~256 MB RGBA) clears any real
/// screenshot while bounding the canvas/encode step.
const MAX_IMAGE_PIXELS = 64 * 1024 * 1024;

/// Standard-alphabet base64 of `bytes`, chunked so a multi-megabyte image
/// does not overflow the argument count of a `String.fromCharCode` spread.
export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

/// Inverse of `bytesToBase64`.
export function base64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

/// A user-facing hint for a clipboard failure. A permission / gesture denial
/// is the common browser case (an async `cs paste` may not carry the terminal
/// keypress's transient activation), so point the user at the fix.
export function hintClipboardError(err: unknown): string {
  const msg = err instanceof Error ? err.message : String(err);
  if (/denied|not allowed|notallowed|permission|gesture|user activation/i.test(msg)) {
    return "clipboard access denied; focus the window or grant clipboard permission";
  }
  return msg;
}

/// Copy `bytes` into a freshly allocated `ArrayBuffer`. A `Uint8Array` may be
/// backed by a `SharedArrayBuffer`, which the DOM `BlobPart` type rejects, so
/// Blob construction goes through this exact-size `ArrayBuffer` copy.
function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  return buffer;
}

/// Re-encode `bytes` to PNG unless they already are one. PNG is the only image
/// type the async clipboard reliably accepts, so a JPEG/GIF/WebP goes through
/// a canvas pass (the same trick the image widget's copy button uses).
async function toPngBlob(bytes: Uint8Array, mime: string): Promise<Blob> {
  const srcBlob = new Blob([toArrayBuffer(bytes)], { type: mime });
  if (mime === "image/png") return srcBlob;
  const bitmap = await createImageBitmap(srcBlob);
  // Reject a decompression bomb before allocating the canvas. `createImageBitmap`
  // may already have allocated the decoded bitmap (a residual we can't fully
  // avoid from JS); the F2 byte cap bounds the source file, and this bounds the
  // far larger canvas + re-encode step.
  if (bitmap.width * bitmap.height > MAX_IMAGE_PIXELS) {
    bitmap.close?.();
    throw new Error("image too large to copy to the clipboard");
  }
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no 2d canvas context");
  ctx.drawImage(bitmap, 0, 0);
  bitmap.close?.();
  return await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
  });
}

/// Derive a plain-text fallback from HTML (its rendered text) so a plain-only
/// paste target still gets readable text alongside the rich version.
function htmlToPlainText(html: string): string {
  try {
    return new DOMParser().parseFromString(html, "text/html").body.textContent ?? "";
  } catch {
    return html;
  }
}

/// Write `bytes` onto the clipboard as `mime` (`cs copy`). Images normalize to
/// PNG; HTML carries a plain-text fallback; anything else writes as text.
export async function writeClipboardPayload(mime: string, bytes: Uint8Array): Promise<void> {
  if (mime.startsWith("image/")) {
    const pngBlob = await toPngBlob(bytes, mime);
    if (isTauriDesktop()) {
      await writeClipboardImage(new Uint8Array(await pngBlob.arrayBuffer()));
      return;
    }
    await navigator.clipboard.write([new ClipboardItem({ "image/png": pngBlob })]);
    return;
  }
  if (mime.startsWith("text/html")) {
    const html = new TextDecoder().decode(bytes);
    const plain = htmlToPlainText(html);
    if (isTauriDesktop()) {
      await writeClipboardHtml(html, plain);
      return;
    }
    await navigator.clipboard.write([
      new ClipboardItem({
        "text/html": new Blob([html], { type: "text/html" }),
        "text/plain": new Blob([plain], { type: "text/plain" }),
      }),
    ]);
    return;
  }
  // Plain text: writeClipboardText already branches desktop/web.
  await writeClipboardText(new TextDecoder().decode(bytes));
}

async function readImagePayload(): Promise<ClipboardPayload | null> {
  if (isTauriDesktop()) {
    const bytes = await readClipboardImage();
    return bytes ? { mime: "image/png", bytes } : null;
  }
  const items = await navigator.clipboard.read();
  for (const item of items) {
    const type = item.types.find((t) => t.startsWith("image/"));
    if (type) {
      const blob = await item.getType(type);
      // Label with the ACTUAL clipboard type: browsers sanitize images to
      // image/png in practice, but if one ever surfaces a non-PNG type the CLI
      // must not stamp png bytes as image/png (wrong container under a .png).
      return { mime: type, bytes: new Uint8Array(await blob.arrayBuffer()) };
    }
  }
  return null;
}

async function readHtmlPayload(): Promise<ClipboardPayload | null> {
  let html: string | null = null;
  if (isTauriDesktop()) {
    html = await readClipboardHtml();
  } else {
    const items = await navigator.clipboard.read();
    for (const item of items) {
      if (item.types.includes("text/html")) {
        html = await (await item.getType("text/html")).text();
        break;
      }
    }
  }
  if (!html) return null;
  return { mime: "text/html", bytes: new TextEncoder().encode(html) };
}

async function readTextPayload(): Promise<ClipboardPayload | null> {
  const text = await readClipboardText();
  if (!text) return null;
  return { mime: PLAIN_MIME, bytes: new TextEncoder().encode(text) };
}

/// Read the clipboard for `cs paste`, honoring `prefer`. Tries each candidate
/// representation in order and returns the first that yields content. Throws a
/// hinted error when a read is denied, and a plain "clipboard is empty" when
/// nothing matched.
export async function readClipboardPayload(prefer: PastePrefer): Promise<ClipboardPayload> {
  const order: Exclude<PastePrefer, "auto">[] =
    prefer === "auto" ? ["image", "text"] : [prefer];
  let lastError: unknown = null;
  for (const kind of order) {
    try {
      const payload =
        kind === "image"
          ? await readImagePayload()
          : kind === "html"
            ? await readHtmlPayload()
            : await readTextPayload();
      if (payload) {
        // Refuse an over-cap payload here rather than build a giant base64
        // string that the reply route (F1 body limit) would 413 anyway.
        if (payload.bytes.length > MAX_CLIPBOARD_BYTES) {
          throw new Error(
            `clipboard content too large (max ${MAX_CLIPBOARD_BYTES / (1024 * 1024)} MB)`,
          );
        }
        return payload;
      }
    } catch (err) {
      lastError = err;
    }
  }
  if (lastError) throw new Error(hintClipboardError(lastError));
  throw new Error("clipboard is empty");
}
