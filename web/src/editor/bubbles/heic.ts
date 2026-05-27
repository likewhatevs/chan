// HEIC/HEIF -> WebP conversion at upload time.
//
// Modern iPhones (and increasingly Macs) default to HEIC for photos.
// Chrome and Firefox can't decode HEIC for inline preview, so an
// untouched HEIC dropped into the editor uploads fine but never
// renders. We convert to WebP in the browser before the upload so
// `Workspace::write_bytes` sees a browser-decodable image.
//
// Server-side conversion was considered and rejected: production-
// ready HEIC decoders are C libraries (`libheif`) and chan's
// "no runtime deps, single static binary" principle forbids that.
// `heic2any` ships libheif as a WASM module which we lazy-import
// only when a HEIC actually arrives, so users without HEIC photos
// pay zero runtime cost (the chunk is built but never fetched).
//
// See GitHub issue #30 for the full requirements.

/// Conservative WebP quality. 0.85 keeps photo content visually
/// indistinguishable from the HEIC original while still cutting
/// ~3x off the file size. Bumping toward 0.95 buys very little on
/// photo content; dropping below 0.75 starts showing artifacts on
/// gradients and skin tones.
const WEBP_QUALITY = 0.85;

/// Detect whether a File is HEIC/HEIF and therefore needs the
/// conversion path. Checks MIME first (iPhone Safari typically
/// fills `image/heic` or `image/heif`), then falls back to the
/// extension because Chrome / Firefox on non-Apple platforms often
/// leave `File.type` empty for HEIC dropped from disk.
export function isHeicFile(file: File): boolean {
  const t = file.type.toLowerCase();
  if (t === "image/heic" || t === "image/heif") return true;
  if (t === "image/heic-sequence" || t === "image/heif-sequence") return true;
  const name = file.name.toLowerCase();
  return name.endsWith(".heic") || name.endsWith(".heif");
}

/// Convert a HEIC/HEIF file to WebP. Returns a new File with the
/// extension swapped and `image/webp` MIME so downstream code paths
/// (attachments route, inline preview, image catalog) treat it like
/// any other web image. Non-HEIC inputs are returned untouched so
/// the caller can pipe every file through unconditionally.
///
/// `onStatus` is an optional progress callback the caller can wire
/// to a status pill / notify(). The helper calls it twice on the
/// conversion path: once before the WASM decode kicks in
/// ("Converting HEIC..."), once when the encode completes (`null`
/// to clear the message). Errors raise; the caller decides whether
/// to drop the file or fall through with the original bytes.
export async function convertHeicForUpload(
  file: File,
  onStatus?: (msg: string | null) => void,
): Promise<File> {
  if (!isHeicFile(file)) return file;
  onStatus?.(`Converting ${file.name}...`);
  // Dynamic import so the heic2any chunk (libheif WASM, ~1.5MB)
  // never lands in the main bundle and never fetches for users
  // without HEIC photos. Vite splits this into its own chunk.
  const mod = await import("heic2any");
  // heic2any can return a single Blob or an array (multi-frame
  // HEIC sequences). We only want the first frame for upload.
  const result = await mod.default({
    blob: file,
    toType: "image/webp",
    quality: WEBP_QUALITY,
  });
  const blob = Array.isArray(result) ? result[0] : result;
  if (!blob) {
    onStatus?.(null);
    throw new Error("HEIC conversion returned no frames");
  }
  const newName = swapExtension(file.name, "webp");
  // File.lastModified preserves the source timestamp so subsequent
  // mtime-based sorts (in the attachments catalog, watcher events)
  // place the converted file alongside the original capture moment.
  const converted = new File([blob], newName, {
    type: "image/webp",
    lastModified: file.lastModified,
  });
  onStatus?.(null);
  return converted;
}

function swapExtension(name: string, newExt: string): string {
  const dot = name.lastIndexOf(".");
  if (dot < 0) return `${name}.${newExt}`;
  return `${name.slice(0, dot)}.${newExt}`;
}
