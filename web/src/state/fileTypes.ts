// Editor file-type whitelist. Mirrors `fs_ops::is_editable_text` in
// `crates/chan-core/src/fs_ops.rs`; keep the two in sync when extending.
//
// Restricting the editor to plain-text content prevents round-tripping
// binary files (images, archives, PDFs) through a UTF-8 buffer, which
// would silently corrupt them on save. The backend rejects non-editable
// writes with 415; the helper here lets the UI short-circuit before the
// round trip and surface the limit in tooltips and disabled controls.

const EDITABLE_EXTENSIONS = new Set(["md", "txt"]);

export function isEditableText(path: string): boolean {
  const dot = path.lastIndexOf(".");
  if (dot < 0 || dot === path.length - 1) return false;
  return EDITABLE_EXTENSIONS.has(path.slice(dot + 1).toLowerCase());
}
