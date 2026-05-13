// Image action overlay. Two-button floating panel shown when the user
// clicks a rendered image atom. Mirrors the legacy editor's overlay:
//
//   [Zoom] [Edit]
//
// - Zoom: opens openImageZoom(src, currentPath) — the existing modal
//   that paints a backdrop and full-resolution image.
// - Edit: places the caret at the image's source position so the
//   selection-intersect rule on the image atom widget kicks in,
//   collapsing the pill to its `![alt](src)` source for in-place
//   editing.
//
// Lifetime: dismissed on outside click, Esc, scroll, or when the host
// (Wysiwyg.svelte) opens a different overlay. Pure DOM, no CM6 state.

import { EditorView } from "@codemirror/view";
import { openImageZoom } from "../../state/imageZoom";

export interface ImageActionOverlayOpts {
  view: EditorView;
  src: string;
  pos: number;
  fromPath: string | null;
}

export function openImageActionOverlay(
  opts: ImageActionOverlayOpts,
): { dismiss: () => void } {
  const wrap = document.createElement("div");
  wrap.className = "md-image-action-overlay";
  wrap.style.position = "absolute";
  wrap.style.zIndex = "30000";

  // Anchor: the rendered image's top-right corner. Look up the DOM
  // node CM6 mounted at the image's position.
  const coords = opts.view.coordsAtPos(opts.pos);
  if (!coords) return { dismiss: () => {} };
  // Place the overlay just inside the image's top-right; the rendered
  // image is at coords.top..bottom, the right edge requires walking
  // the DOM. coordsAtPos returns the position of the source character
  // (the image atom is replace-decoration); we approximate by placing
  // at coords.top with an x offset slightly right of left.
  wrap.style.top = `${Math.round(coords.top + window.scrollY + 4)}px`;
  wrap.style.left = `${Math.round(coords.left + window.scrollX + 8)}px`;

  const zoomBtn = makeBtn("Zoom", () => {
    openImageZoom(opts.src, opts.fromPath);
    dismiss();
  });
  const editBtn = makeBtn("Edit", () => {
    opts.view.dispatch({
      selection: { anchor: opts.pos },
    });
    opts.view.focus();
    dismiss();
  });
  wrap.appendChild(zoomBtn);
  wrap.appendChild(editBtn);
  document.body.appendChild(wrap);

  let alive = true;
  function dismiss(): void {
    if (!alive) return;
    alive = false;
    document.removeEventListener("mousedown", outsideClick, true);
    document.removeEventListener("keydown", escListener, true);
    window.removeEventListener("scroll", dismiss, true);
    wrap.remove();
  }
  function outsideClick(e: MouseEvent): void {
    if (wrap.contains(e.target as Node)) return;
    dismiss();
  }
  function escListener(e: KeyboardEvent): void {
    if (e.key === "Escape") dismiss();
  }
  // Defer listener wiring one tick so the click that opened us
  // doesn't immediately count as an outside click.
  window.setTimeout(() => {
    if (!alive) return;
    document.addEventListener("mousedown", outsideClick, true);
    document.addEventListener("keydown", escListener, true);
    window.addEventListener("scroll", dismiss, true);
  }, 0);

  return { dismiss };
}

function makeBtn(label: string, onClick: () => void): HTMLElement {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "md-image-action-btn";
  btn.textContent = label;
  btn.addEventListener("mousedown", (e) => {
    e.preventDefault();
    e.stopPropagation();
    onClick();
  });
  return btn;
}
