// Fullscreen image viewer. Imperative DOM helper so any surface
// (editor zoom button, inspector image entries) can open the same
// overlay. Click the backdrop or press Escape to dismiss; the
// overlay cleans itself up.
//
// Styles live next to the editor in Wysiwyg.svelte (.md-image-zoom);
// reusing the same class keeps the visual identical without dragging
// the CSS into every host.

import { resolveImageSrc } from "../editor/extensions/image";

/// Open the fullscreen viewer.
///
///   src       Markdown-style image src. May be drive-rooted
///             ("attachments/pic.png"), source-relative
///             ("./img.png"), or an http(s)/data URL.
///   fromPath  Drive-relative path of the document the src was
///             authored in. Used to resolve source-relative refs.
///             Pass null when src is already drive-rooted (the
///             inspector's link list passes resolved paths).
///
/// No-op on empty / unresolvable src.
export function openImageZoom(src: string, fromPath: string | null = null): void {
  if (!src) return;
  const resolved = resolveImageSrc(src, fromPath);
  if (!resolved) return;

  const backdrop = document.createElement("div");
  backdrop.className = "md-image-zoom";
  const img = document.createElement("img");
  img.src = resolved;
  img.alt = "";
  img.draggable = false;
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
