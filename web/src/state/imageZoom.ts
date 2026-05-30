// Fullscreen image viewer. Imperative DOM helper so any surface
// (editor zoom button, inspector image entries) can open the same
// overlay. Click the backdrop or press Escape to dismiss; the
// overlay cleans itself up.
//
// Styles are applied inline so the helper is self-contained - the
// previous "CSS lives in Wysiwyg.svelte" arrangement broke during
// the CM6 cutover when the `:global(.md-image-zoom)` block was
// dropped, leaving the View button technically working (backdrop
// appended) but invisibly stacked below the editor in static flow.
// Inline keeps the helper resilient to future component shuffles.

import { resolveImageSrc } from "../editor/extensions/image";

/// Open the fullscreen viewer.
///
///   src       Markdown-style image src. May be workspace-rooted
///             ("attachments/pic.png"), source-relative
///             ("./img.png"), or an http(s)/data URL.
///   fromPath  Workspace-relative path of the document the src was
///             authored in. Used to resolve source-relative refs.
///             Pass null when src is already workspace-rooted (the
///             inspector's link list passes resolved paths).
///
/// No-op on empty / unresolvable src.
export function openImageZoom(src: string, fromPath: string | null = null): void {
  if (!src) return;
  const resolved = resolveImageSrc(src, fromPath);
  if (!resolved) return;

  const backdrop = document.createElement("div");
  backdrop.className = "md-image-zoom";
  backdrop.style.cssText =
    "position:fixed;inset:0;z-index:40000;" +
    "background:rgba(0,0,0,0.92);" +
    "display:flex;align-items:center;justify-content:center;" +
    "cursor:zoom-out;";
  const img = document.createElement("img");
  img.src = resolved;
  img.alt = "";
  img.draggable = false;
  img.style.cssText =
    "max-width:92vw;max-height:92vh;width:auto;height:auto;" +
    "object-fit:contain;box-shadow:0 8px 32px rgba(0,0,0,0.5);";
  backdrop.appendChild(img);
  document.body.appendChild(backdrop);

  const dismiss = (): void => {
    document.removeEventListener("keydown", onKey, true);
    backdrop.remove();
  };
  const onKey = (ev: KeyboardEvent): void => {
    if (ev.key === "Escape") {
      ev.preventDefault();
      dismiss();
    }
  };
  backdrop.addEventListener("click", () => dismiss());
  document.addEventListener("keydown", onKey, true);
}
