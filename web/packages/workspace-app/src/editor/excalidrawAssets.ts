// Point excalidraw's font registry at the self-hosted woff2 bundle
// (web/dist/static/excalidraw/fonts, emitted by vite.config.ts) instead
// of the esm.sh CDN fallback the package falls through to when
// window.EXCALIDRAW_ASSET_PATH is unset. Without this, the shipped fence
// renderer and the interactive canvas both fetch label fonts from the
// network, so offline sessions and chan-desktop degrade silently.
//
// Called on BOTH lazy excalidraw entry points (the mermaid fence
// renderer's loadExcalidraw() and the canvas wrapper) so a fence-only
// session is covered too. The value is read lazily by excalidraw on
// first font-registry access, so setting it before the first render is
// enough; there is no import-time read to beat.
//
// Prefix-aware: excalidraw resolves the value against the window origin,
// but chan serves each workspace under a mount prefix (chan-desktop
// loads {prefix}/index.html; devservers mount /{slug}-{8hex}). apiPath
// prepends the same injected chan-prefix the rest of the transport uses,
// so a prefixed tenant fetches {prefix}/static/excalidraw/... and not the
// launcher HTML shell (which an origin-absolute /static/... would hit).
import { apiPath } from "../api/transport";

declare global {
  interface Window {
    EXCALIDRAW_ASSET_PATH?: string | string[];
  }
}

export function configureExcalidrawAssets(): void {
  if (typeof window === "undefined") return;
  window.EXCALIDRAW_ASSET_PATH = apiPath("/static/excalidraw/");
}
